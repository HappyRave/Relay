// The app store. In M0 it is a faithful in-browser port of the prototype's
// logic (Design/Macro Recorder.dc.html) so the UI is fully interactive; from
// M2/M3 recording and playback move to the Rust engine and these methods
// become thin IPC calls.
import type { ExportFormat, MacroView, Mode, MovePoint, PlaybackOptions, Rect, Settings, Step, Tab, Triggers } from "../types";
import {
  type MockEvent,
  type MockMacro,
  defaultPlayback,
  defaultTriggers,
  deriveSteps,
  eventsDuration,
  newEventId,
  sampleLibrary,
} from "../dev/sampleMacros";
import { lastIndexAtOrBefore } from "../preview/geometry";
import { jumpTarget } from "../timeline/lanes";
import { slug } from "../format";

const COUNTDOWN_MS = 3000;
const MOVE_INTERVAL_MS = 16;
const MODIFIERS = ["Control", "Alt", "Shift", "Meta"];

function toView(m: MockMacro, events: MockEvent[], desktop: Rect, frames: Rect[]): MacroView {
  const moves: MovePoint[] = [];
  for (const e of events) if (e.type === "move") moves.push({ t: e.t, x: e.x, y: e.y });
  return { id: m.id, name: m.name, desktop, frames, moves, steps: deriveSteps(events), duration: eventsDuration(events) };
}

class RelayStore {
  mode = $state<Mode>("idle");
  cur = $state(0);
  countLeft = $state(0);
  loopIdx = $state(0);
  expanded = $state(true);
  tab = $state<Tab>("events");
  exportOpen = $state(false);
  exportFmt = $state<ExportFormat>("rly");
  settings = $state<Settings>({
    captureMoves: true,
    captureKeys: true,
    countdown: true,
    pathMode: "full",
    showClickLabels: true,
  });
  library = $state.raw<MockMacro[]>(sampleLibrary());
  selId = $state("inv");

  /** Set by the widget so input on the widget itself is never recorded. */
  widgetEl: HTMLElement | null = null;

  private recEvents: MockEvent[] = [];
  private recRev = $state(0);
  private recDesktop: Rect = { x: 0, y: 0, w: 1920, h: 1080 };
  private recStart = 0;
  private lastMoveT = -99;
  private lastRecBump = 0;
  private raf = 0;
  private lastFrame = 0;
  private cleanup: (() => void)[] = [];

  current: MockMacro = $derived(this.library.find((m) => m.id === this.selId) ?? this.library[0]);
  playback: PlaybackOptions = $derived(this.current.playback);
  triggers: Triggers = $derived(this.current.triggers);

  view: MacroView = $derived.by(() => {
    if (this.mode === "rec") {
      void this.recRev;
      return toView(this.current, this.recEvents, this.recDesktop, []);
    }
    return toView(this.current, this.current.events, this.current.desktop, this.current.frames);
  });

  duration = $derived(this.mode === "rec" ? Math.max(this.cur, 2000) : this.view.duration);
  recording = $derived(this.mode === "rec" || this.mode === "count");

  /** Library rows with summary stats. */
  libraryItems = $derived(
    this.library.map((m) => {
      const steps = deriveSteps(m.events);
      return {
        id: m.id,
        name: m.name,
        durationMs: eventsDuration(m.events),
        stepCount: steps.length,
        runs: m.runs,
        lastRun: m.lastRun,
        hotkey: m.triggers.hotkey.combo,
      };
    }),
  );

  // — lifecycle —

  start() {
    this.lastFrame = performance.now();
    const loop = (now: number) => {
      const dt = Math.min(64, now - this.lastFrame);
      this.lastFrame = now;
      this.tick(dt, now);
      this.raf = requestAnimationFrame(loop);
    };
    this.raf = requestAnimationFrame(loop);
    const on = <K extends keyof WindowEventMap>(type: K, fn: (e: WindowEventMap[K]) => void) => {
      window.addEventListener(type, fn, true);
      this.cleanup.push(() => window.removeEventListener(type, fn, true));
    };
    on("keydown", this.onKey);
    on("pointermove", this.onPointerMove);
    on("pointerdown", this.onPointerDown);
    on("contextmenu", (e) => {
      if (this.mode === "rec") e.preventDefault();
    });
  }

  dispose() {
    cancelAnimationFrame(this.raf);
    for (const c of this.cleanup) c();
    this.cleanup = [];
  }

  private tick(dt: number, now: number) {
    if (this.mode === "count") {
      const left = this.countLeft - dt;
      if (left <= 0) this.startRec();
      else this.countLeft = left;
    } else if (this.mode === "rec") {
      this.cur = now - this.recStart;
      if (now - this.lastRecBump > 33) {
        this.lastRecBump = now;
        this.recRev++;
      }
    } else if (this.mode === "play") {
      const c = this.cur + dt * this.playback.speed;
      const d = this.duration;
      if (c >= d) {
        if (this.playback.infinite || this.loopIdx + 1 < this.playback.loops) {
          this.cur = 0;
          this.loopIdx++;
        } else {
          this.mode = "idle";
          this.cur = d;
          this.loopIdx = 0;
          this.updateCurrent((m) => ({ ...m, runs: m.runs + 1, lastRun: "Just now" }));
        }
      } else this.cur = c;
    }
  }

  // — input (dev desktop only: in Tauri the window holds nothing but the widget) —

  private inWidget(e: Event) {
    return !!this.widgetEl && e.target instanceof Node && this.widgetEl.contains(e.target);
  }

  private onKey = (e: KeyboardEvent) => {
    if (e.key === "F9") {
      e.preventDefault();
      this.toggleRec();
      return;
    }
    if (e.key === "F10") {
      e.preventDefault();
      this.togglePlay();
      return;
    }
    if (e.key === "Escape") {
      if (this.exportOpen) this.exportOpen = false;
      else if (this.mode !== "idle") this.stop();
      return;
    }
    if (e.ctrlKey && e.shiftKey && e.key.toLowerCase() === "m" && this.mode === "idle") {
      e.preventDefault();
      this.expanded = !this.expanded;
      return;
    }
    if (this.mode === "play" && this.playback.stopOnKey && !this.inWidget(e)) {
      this.mode = "idle";
      return;
    }
    if (this.mode !== "rec" || !this.settings.captureKeys || this.inWidget(e)) return;
    if (MODIFIERS.includes(e.key)) return;
    e.preventDefault();
    const t = performance.now() - this.recStart;
    const nm = e.key === " " ? "Space" : e.key.length === 1 ? e.key.toUpperCase() : e.key;
    if (e.ctrlKey || e.altKey || e.metaKey) {
      const mods = [e.ctrlKey && "Ctrl", e.altKey && "Alt", e.shiftKey && "Shift", e.metaKey && "Win"].filter(Boolean);
      this.recEvents.push({ id: newEventId(), t, type: "key", key: [...mods, nm].join(" + ") });
    } else if (e.key.length === 1) this.recEvents.push({ id: newEventId(), t, type: "char", key: e.key });
    else this.recEvents.push({ id: newEventId(), t, type: "key", key: (e.shiftKey ? "Shift + " : "") + nm });
  };

  private onPointerMove = (e: PointerEvent) => {
    if (this.mode !== "rec" || !this.settings.captureMoves || this.inWidget(e)) return;
    const t = performance.now() - this.recStart;
    if (t - this.lastMoveT < MOVE_INTERVAL_MS) return;
    this.lastMoveT = t;
    this.recEvents.push({ id: newEventId(), t, type: "move", x: Math.round(e.clientX), y: Math.round(e.clientY) });
  };

  private onPointerDown = (e: PointerEvent) => {
    if (this.mode !== "rec" || this.inWidget(e)) return;
    const t = performance.now() - this.recStart;
    const x = Math.round(e.clientX);
    const y = Math.round(e.clientY);
    const btn = e.button === 2 ? "Right" : e.button === 1 ? "Middle" : "Left";
    this.recEvents.push({ id: newEventId(), t, type: "move", x, y });
    this.recEvents.push({ id: newEventId(), t, type: "click", x, y, btn, count: 1, label: "" });
  };

  // — session —

  toggleRec = () => {
    if (this.recording) return this.stopRec();
    this.loopIdx = 0;
    if (this.settings.countdown) {
      this.mode = "count";
      this.countLeft = COUNTDOWN_MS;
      this.cur = 0;
    } else this.startRec();
  };

  private startRec() {
    this.recEvents = [];
    this.lastMoveT = -99;
    this.recStart = performance.now();
    this.recDesktop = { x: 0, y: 0, w: window.innerWidth, h: window.innerHeight };
    this.mode = "rec";
    this.cur = 0;
    this.countLeft = 0;
  }

  private stopRec() {
    const ev = this.recEvents;
    const acts = ev.filter((e) => e.type !== "move").length;
    const moves = ev.length - acts;
    if (this.mode === "count" || (acts === 0 && moves < 5)) {
      this.mode = "idle";
      this.cur = 0;
      return;
    }
    if (!ev.length || ev[0].type !== "move") {
      const f = ev.find((e) => "x" in e) as { x: number; y: number } | undefined;
      ev.unshift({ id: newEventId(), t: 0, type: "move", x: f?.x ?? this.recDesktop.w / 2, y: f?.y ?? this.recDesktop.h / 2 });
    }
    ev.sort((a, b) => a.t - b.t);
    const n = this.library.filter((m) => m.id.startsWith("rec-")).length + 1;
    const macro: MockMacro = {
      id: "rec-" + Date.now(),
      name: "Recording " + n,
      desktop: this.recDesktop,
      frames: [],
      events: ev.map((e) => ({ ...e, t: Math.round(e.t) })),
      runs: 0,
      lastRun: "Never",
      playback: defaultPlayback(),
      triggers: defaultTriggers(),
    };
    this.library = [macro, ...this.library];
    this.selId = macro.id;
    this.recEvents = [];
    this.mode = "idle";
    this.cur = 0;
    this.tab = "events";
  }

  togglePlay = () => {
    if (this.recording) return;
    if (this.mode === "play") {
      this.mode = "pause";
      return;
    }
    const resuming = this.mode === "pause";
    if (this.cur >= this.duration - 1) this.cur = 0;
    if (!resuming) this.loopIdx = 0;
    this.mode = "play";
  };

  stop = () => {
    if (this.recording) return this.stopRec();
    this.mode = "idle";
    this.cur = 0;
    this.loopIdx = 0;
  };

  seek = (t: number) => {
    if (this.recording) return;
    this.cur = Math.max(0, Math.min(this.duration, t));
  };

  jump = (dir: -1 | 1) => {
    this.cur = jumpTarget(this.view.steps, this.cur, dir, this.duration);
  };

  /** Cursor position at the playhead (virtual-desktop px). */
  cursorAt(t: number): { x: number; y: number } {
    const moves = this.view.moves;
    const i = lastIndexAtOrBefore(moves, t);
    const m = moves[Math.max(0, i)];
    if (m) return m;
    const d = this.view.desktop;
    return { x: d.x + d.w / 2, y: d.y + d.h / 2 };
  }

  // — library & edits —

  private updateCurrent(fn: (m: MockMacro) => MockMacro) {
    const id = this.current.id;
    this.library = this.library.map((m) => (m.id === id ? fn(m) : m));
  }

  loadMacro = (id: string) => {
    if (this.recording) return;
    this.selId = id;
    this.cur = 0;
    this.mode = "idle";
    this.loopIdx = 0;
    this.tab = "events";
  };

  rename = (name: string) => this.updateCurrent((m) => ({ ...m, name }));

  setPlayback = (patch: Partial<PlaybackOptions>) =>
    this.updateCurrent((m) => ({ ...m, playback: { ...m.playback, ...patch } }));

  setTriggers = (patch: Partial<Triggers>) =>
    this.updateCurrent((m) => ({ ...m, triggers: { ...m.triggers, ...patch } }));

  private insertAt(make: (t: number) => MockEvent, dur: number) {
    if (this.recording) return;
    const c = Math.round(this.cur);
    this.updateCurrent((m) => ({
      ...m,
      events: [...m.events.map((v) => (v.t > c ? { ...v, t: v.t + dur } : v)), make(c)].sort((a, b) => a.t - b.t),
    }));
  }

  insertWait = () => this.insertAt((t) => ({ id: newEventId(), t, type: "wait", dur: 500, label: "Inserted" }), 500);

  insertPixelCheck = () => {
    const p = this.cursorAt(this.cur);
    this.insertAt((t) => ({ id: newEventId(), t, type: "cond", dur: 800, label: "Inserted", x: p.x, y: p.y, color: "#EC3013" }), 800);
  };

  deleteStep = (step: Step) => {
    if (this.recording) return;
    const ids = new Set(step.items);
    this.updateCurrent((m) => ({ ...m, events: m.events.filter((v) => !ids.has(v.id)) }));
  };

  // — export (M5 replaces the download with a native save dialog) —

  exportName = $derived(slug(this.current.name) + "." + this.exportFmt);

  doExport = () => {
    const m = this.current;
    const body = { format: "relay-macro", version: 0, name: m.name, events: m.events };
    const blob = new Blob([JSON.stringify(body, null, this.exportFmt === "json" ? 2 : 0)], { type: "application/json" });
    const a = document.createElement("a");
    a.href = URL.createObjectURL(blob);
    a.download = this.exportName;
    a.click();
    URL.revokeObjectURL(a.href);
    this.exportOpen = false;
  };
}

export const relay = new RelayStore();
