// The app store. Macros, steps and edits come from relay-core through the
// backend; the session (mode, countdown, recording, playback clock) is owned
// by the Rust coordinator and streamed here as EngineMsg. The UI only
// extrapolates the playhead between ticks for smooth 60 fps motion.
//
// Async results are tied to the macro they were for: a response that
// arrives after the user opened another macro is dropped, never applied to
// the wrong one.
import type {
  EditOp,
  ExportFormat,
  MacroListItem,
  MacroTriggers,
  MacroView,
  Mode,
  MovePoint,
  PlaybackOptions,
  Rect,
  Settings,
  Step,
  Tab,
  TriggerStatus,
} from "../types";
import type { EngineMsg } from "../ipc/bindings/EngineMsg";
import type { PickedPixel } from "../ipc/bindings/PickedPixel";
import type { TimingStats } from "../ipc/bindings/TimingStats";
import { backend, type IpcError } from "../ipc/backend";
import { DEFAULT_PLAYBACK, DEFAULT_SETTINGS } from "../defaults";
import { isTauri, savedExpanded } from "../platform/window";
import { lastIndexAtOrBefore } from "../preview/geometry";
import { currentStepIndex, jumpTarget } from "../timeline/lanes";
import { plural, slug } from "../format";

const RENAME_DEBOUNCE_MS = 250;
/** "Trim pauses" shortens every pause longer than this to this. */
export const TRIM_PAUSE_MS = 1000;
/** How far past the last tick the playhead may run before the next one arrives. */
const MAX_EXTRAPOLATION_MS = 100;
/** Seconds "Pick" waits before reading the pixel under the cursor. */
const PICK_SECONDS = 3;
const EMPTY_DESKTOP: Rect = { x: 0, y: 0, w: 1920, h: 1080 };

export interface Toast {
  kind: "error" | "info";
  message: string;
  action?: { label: string; run: () => void };
}

class RelayStore {
  readonly editable = backend.editable;

  // — session (from the engine stream) —
  mode = $state<Mode>("idle");
  /** The playhead, in macro ms. */
  cur = $state(0);
  countLeft = $state(0);
  loopIdx = $state(0);
  /** Timing of the last playback (for diagnostics and the end-to-end tests). */
  lastTiming: TimingStats | null = null;

  // — data (from commands) —
  settings = $state.raw<Settings>(DEFAULT_SETTINGS);
  library = $state.raw<MacroListItem[]>([]);
  view = $state.raw<MacroView | null>(null);
  /** The open macro's triggers, with the next scheduled run and hotkey problems. */
  triggerStatus = $state.raw<TriggerStatus | null>(null);
  /** Paused by the kill switch (or from the tray). */
  triggersPaused = $state(false);
  autostart = $state(false);
  /** Running programs, for the "When app launches" suggestions. */
  processes = $state.raw<string[]>([]);

  // — UI —
  /** False until the saved window mode is known (so the widget never flashes the wrong size). */
  ready = $state(false);
  expanded = $state(true);
  tab = $state<Tab>("steps");
  exportOpen = $state(false);
  exportFmt = $state<ExportFormat>("rly");
  /** A short message, optionally with an action (Undo). */
  toast = $state.raw<Toast | null>(null);
  /** Seconds left before "Pick" reads the cursor's pixel, or 0 when not picking. */
  picking = $state(0);

  // — private —
  /** Live recording data; appended in place, `recVersion` tells the derived values. */
  private recMoves: MovePoint[] = [];
  private recSteps: Step[] = [];
  private recVersion = $state(0);
  private recDesktop = $state.raw<Rect>(EMPTY_DESKTOP);
  /** The last playback or recording tick, for extrapolation. */
  private tick = { t: 0, at: 0, speed: 1, advancing: false };
  private raf = 0;
  private listening = false;
  /** Bumped by every request that replaces the view, so older responses are dropped. */
  private viewSeq = 0;
  private rename_: { id: string; name: string; timer: ReturnType<typeof setTimeout> } | null = null;
  private toastTimer: ReturnType<typeof setTimeout> | undefined;
  /** A just-saved recording to open once the session is back to idle. */
  private pendingLoad: string | null = null;

  // — derived —
  recording = $derived(this.mode === "recording" || this.mode === "countdown");
  playing = $derived(this.mode === "playing" || this.mode === "paused");
  name = $derived(this.view?.name ?? "");
  playback: PlaybackOptions = $derived(this.view?.playback ?? DEFAULT_PLAYBACK);
  loops = $derived(this.playback.repeat === "forever" ? Infinity : this.playback.repeat.count);
  steps: Step[] = $derived.by(() => {
    if (this.mode !== "recording") return this.view?.steps ?? [];
    void this.recVersion;
    return this.recSteps;
  });
  moves: MovePoint[] = $derived.by(() => {
    if (this.mode !== "recording") return this.view?.moves ?? [];
    void this.recVersion;
    return this.recMoves;
  });
  /** While recording, grows a second at a time so the timeline isn't rebuilt every frame. */
  duration = $derived(
    this.mode === "recording" ? Math.max(2000, Math.ceil(this.cur / 1000) * 1000) : (this.view?.duration ?? 2000),
  );
  desktop: Rect = $derived(
    this.mode === "recording" ? this.recDesktop : (this.view?.recording.virtual_desktop ?? EMPTY_DESKTOP),
  );
  /** Outlines for the preview: the monitors and the window the macro is anchored to. */
  frames: Rect[] = $derived.by(() => {
    const r = this.view?.recording;
    if (!r || this.mode === "recording") return [];
    const out = r.monitors.map((m) => m.rect);
    if (r.anchor_window) out.push(r.anchor_window.rect);
    return out;
  });
  /** The step under the playhead, or -1. */
  curStepIdx = $derived(currentStepIndex(this.steps, Math.min(this.cur, this.duration)));
  triggers: MacroTriggers | null = $derived(this.triggerStatus?.triggers ?? null);
  canUndo = $derived(this.editable && this.mode === "idle" && !!this.view?.can_undo);
  canRedo = $derived(this.editable && this.mode === "idle" && !!this.view?.can_redo);
  /** Pauses "Trim pauses" would shorten. */
  longPauses = $derived(this.steps.filter((s) => s.pause > TRIM_PAUSE_MS).length);
  error = $derived(this.toast?.kind === "error" ? this.toast.message : null);
  exportName = $derived((slug(this.name) || "macro") + "." + this.exportFmt);

  // — lifecycle —

  async init() {
    this.expanded = await savedExpanded().catch(() => true);
    this.ready = true;
    await this.run(backend.subscribe(this.onEngine));
    this.settings = (await this.run(backend.getSettings())) ?? this.settings;
    await this.refreshLibrary();
    if (this.library[0]) await this.loadMacro(this.library[0].id);
    this.autostart = await backend.getAutostart().catch(() => false);
  }

  start() {
    const loop = (now: number) => {
      this.frame(now);
      this.raf = requestAnimationFrame(loop);
    };
    this.raf = requestAnimationFrame(loop);
    window.addEventListener("keydown", this.onKey, true);
    this.listening = true;
  }

  dispose() {
    cancelAnimationFrame(this.raf);
    if (this.listening) window.removeEventListener("keydown", this.onKey, true);
    this.listening = false;
    clearTimeout(this.toastTimer);
    if (this.rename_) clearTimeout(this.rename_.timer);
  }

  /** Moves the playhead smoothly between engine ticks. */
  private frame(now: number) {
    if ((this.mode !== "playing" && this.mode !== "recording") || !this.tick.advancing) return;
    const ahead = Math.min(now - this.tick.at, MAX_EXTRAPOLATION_MS) * this.tick.speed;
    this.cur = this.tick.t + ahead;
  }

  private onEngine = (msg: EngineMsg) => {
    const now = performance.now();
    switch (msg.type) {
      case "session": {
        const mode = msg.mode;
        // A trigger started another macro: show the one that's playing.
        if (mode === "playing" && msg.macro_id && msg.macro_id !== this.view?.id) this.showMacro(msg.macro_id);
        if (mode === "recording") {
          this.recMoves = [];
          this.recSteps = [];
          this.recVersion++;
          this.cur = 0;
          this.tick = { t: 0, at: now, speed: 1, advancing: true };
        }
        if (mode === "countdown") this.cur = 0;
        if (mode === "idle") {
          this.tick.advancing = false;
          this.recMoves = [];
          this.recSteps = [];
        }
        this.mode = mode;
        if (mode === "idle" && this.pendingLoad) {
          this.loadMacro(this.pendingLoad);
          this.pendingLoad = null;
        }
        break;
      }
      case "countdown":
        this.countLeft = msg.left_ms;
        break;
      case "rec_progress":
        this.recDesktop = msg.desktop;
        for (const m of msg.moves) this.recMoves.push(m);
        if (msg.steps) this.recSteps = msg.steps;
        if (msg.moves.length || msg.steps) this.recVersion++;
        this.tick = { t: msg.elapsed_ms, at: now, speed: 1, advancing: true };
        break;
      case "play_tick":
        this.tick = { t: msg.t, at: now, speed: msg.speed, advancing: msg.advancing };
        this.loopIdx = msg.loop_idx;
        if (!msg.advancing) this.cur = msg.t;
        break;
      case "finished":
        this.loopIdx = 0;
        if (msg.timing) this.lastTiming = msg.timing;
        // Any stop rewinds (Stop, Esc, a key press, the kill switch). A completed run stays at
        // the end, and a timed-out pixel check stays on its step so the row is highlighted.
        if (msg.reason !== "completed" && msg.reason !== "pixel_timeout") this.cur = 0;
        break;
      case "saved":
        // Arrives just before the session returns to idle.
        if (this.mode === "idle") this.loadMacro(msg.id);
        else this.pendingLoad = msg.id;
        break;
      case "library_changed":
        this.refreshLibrary();
        break;
      case "triggers_paused":
        this.triggersPaused = msg.paused;
        break;
      case "toggle_compact":
        this.expanded = !this.expanded;
        break;
      case "error":
        this.fail({ code: msg.type, message: msg.message });
        break;
      case "notice":
        this.notify(msg.message, undefined, 6000);
        break;
    }
  };

  // In the app F9, F10 and Ctrl+Shift+M are global hotkeys handled in Rust;
  // the browser preview has none, so it listens here.
  private onKey = (e: KeyboardEvent) => {
    // A hotkey being set, or the export dialog (which handles Esc itself), owns the keyboard.
    if (this.exportOpen || (e.target as HTMLElement | null)?.closest?.("[data-captures-keys]")) return;
    const key = e.key.toLowerCase();
    if (e.key === "Escape") {
      if (!isTauri() && this.mode !== "idle") this.stop();
      return;
    }
    // Undo and redo, except in text fields, which have their own.
    const typing = (e.target as HTMLElement | null)?.closest?.("input, textarea");
    if ((e.ctrlKey || e.metaKey) && !e.altKey && !typing && (key === "z" || key === "y")) {
      e.preventDefault();
      if (key === "y" || e.shiftKey) this.redo();
      else this.undo();
      return;
    }
    if (isTauri()) return;
    if (e.key === "F9") {
      e.preventDefault();
      this.toggleRec();
    } else if (e.key === "F10") {
      e.preventDefault();
      this.togglePlay();
    } else if (e.ctrlKey && e.shiftKey && key === "m") {
      e.preventDefault();
      this.expanded = !this.expanded;
    }
  };

  // — errors and toasts —

  /** Awaits `p`, showing any failure; resolves to `undefined` when it failed. */
  private async run<T>(p: Promise<T>): Promise<T | undefined> {
    try {
      return await p;
    } catch (e) {
      this.fail(e);
      return undefined;
    }
  }

  private fail(e: unknown) {
    const message = (e as IpcError)?.message ?? String(e);
    console.error("Relay:", e);
    this.show({ kind: "error", message }, 5000);
  }

  notify(message: string, action?: Toast["action"], ms = 5000) {
    this.show({ kind: "info", message, action }, action ? Math.max(ms, 8000) : ms);
  }

  dismissToast = () => {
    clearTimeout(this.toastTimer);
    this.toast = null;
  };

  private show(toast: Toast, ms: number) {
    this.toast = toast;
    clearTimeout(this.toastTimer);
    this.toastTimer = setTimeout(() => (this.toast = null), ms);
  }

  // — session —

  toggleRec = () => {
    if (this.playing) return;
    return this.run(backend.toggleRecord());
  };

  togglePlay = () => {
    if (this.recording || !this.view) return;
    return this.run(backend.togglePlay(this.cur >= this.duration - 1 ? 0 : this.cur));
  };

  stop = () => this.run(backend.stop());

  /** Moves the playhead now; the engine hears about it at most once per frame. */
  seek = (t: number) => {
    if (this.recording) return;
    const clamped = Math.max(0, Math.min(this.duration, t));
    this.cur = clamped;
    this.tick = { ...this.tick, t: clamped, at: performance.now() };
    if (!this.seekQueued) {
      this.seekQueued = true;
      requestAnimationFrame(() => {
        this.seekQueued = false;
        this.run(backend.seek(this.cur));
      });
    }
  };
  private seekQueued = false;

  jump = (dir: -1 | 1) => this.seek(jumpTarget(this.steps, this.cur, dir, this.duration));

  /** Cursor position at time `t` (virtual-desktop px). */
  cursorAt(t: number): { x: number; y: number } {
    const i = lastIndexAtOrBefore(this.moves, t);
    const m = this.moves[Math.max(0, i)];
    if (m) return m;
    const d = this.desktop;
    return { x: d.x + d.w / 2, y: d.y + d.h / 2 };
  }

  // — library —

  async refreshLibrary() {
    this.library = (await this.run(backend.listMacros())) ?? this.library;
  }

  loadMacro = async (id: string) => {
    if (this.recording || this.playing) return;
    await this.showMacro(id);
    this.tab = "steps";
  };

  /** Opens a macro in the editor (also mid-playback, when a trigger started it). */
  private async showMacro(id: string) {
    await this.flushRename();
    const seq = ++this.viewSeq;
    const view = await this.run(backend.loadMacro(id));
    if (!view || seq !== this.viewSeq) return;
    this.view = view;
    this.triggerStatus = null; // the old macro's triggers mustn't be edited into this one
    this.cur = 0;
    this.loopIdx = 0;
    const status = await this.run(backend.getTriggers(id));
    if (status && this.view?.id === id) {
      this.triggerStatus = status;
      this.triggersPaused = status.paused;
    }
  }

  duplicateMacro = async (id: string) => {
    const copy = await this.run(backend.duplicateMacro(id));
    if (!copy) return;
    await this.refreshLibrary();
    await this.loadMacro(copy);
    this.tab = "library";
  };

  /** Moves a macro to the trash, with Undo. Opens a neighbour if it was the open one. */
  deleteMacro = async (id: string) => {
    const idx = this.library.findIndex((m) => m.id === id);
    const name = this.library[idx]?.name ?? "macro";
    try {
      await backend.deleteMacro(id);
    } catch (e) {
      return this.fail(e);
    }
    await this.refreshLibrary();
    if (this.view?.id === id) {
      const next = this.library[Math.min(idx, this.library.length - 1)];
      if (next) await this.loadMacro(next.id);
      else this.view = null;
      this.tab = "library";
    }
    this.notify(`Moved “${name}” to the trash`, { label: "Undo", run: () => this.restoreMacro(id) });
  };

  restoreMacro = async (id: string) => {
    this.dismissToast();
    try {
      await backend.restoreMacro(id);
    } catch (e) {
      return this.fail(e);
    }
    await this.refreshLibrary();
    await this.loadMacro(id);
    this.tab = "library";
  };

  importMacros = async () => {
    const result = await this.run(backend.importMacros());
    if (!result) return;
    await this.refreshLibrary();
    if (result.imported[0]) await this.loadMacro(result.imported[0]);
    this.tab = "library";
    const n = result.imported.length;
    const done = n ? `Imported ${plural(n, "macro")}` : "Nothing imported";
    if (result.problems.length) this.fail({ code: "import", message: `${done}. ${result.problems.join("; ")}` });
    else this.notify(done);
  };

  // — edits —

  edit = (op: EditOp) => {
    if (!this.view || this.recording) return;
    if (!this.editable) return this.fail({ code: "unavailable", message: "Editing needs the Relay app" });
    // An Undo offered for an earlier change would now undo this one instead.
    if (this.toast?.action) this.dismissToast();
    return this.apply(this.view.id, backend.editMacro(this.view.id, op));
  };

  /** Shows a command's result for macro `id`, unless another request replaced the view since. */
  private async apply(id: string, request: Promise<MacroView>) {
    const seq = ++this.viewSeq;
    let view = await this.run(request);
    if (!view || seq !== this.viewSeq || this.view?.id !== id) return;
    // A name still being typed wins over the one in the response.
    if (this.rename_?.id === id) view = { ...view, name: this.rename_.name };
    const listChanged = view.name !== this.view.name || view.duration !== this.view.duration || view.steps.length !== this.view.steps.length;
    this.view = view;
    if (listChanged) await this.refreshLibrary();
  }

  /** Renames the open macro as the user types; saved after a short pause. */
  rename = (name: string) => {
    if (!this.view) return;
    const id = this.view.id;
    this.view = { ...this.view, name };
    if (!this.editable) return;
    if (this.rename_) clearTimeout(this.rename_.timer);
    this.rename_ = { id, name, timer: setTimeout(() => this.flushRename(), RENAME_DEBOUNCE_MS) };
  };

  /** Saves a pending rename now (before switching macros, undoing, …). */
  private async flushRename() {
    const r = this.rename_;
    if (!r) return;
    clearTimeout(r.timer);
    this.rename_ = null;
    await this.run(backend.editMacro(r.id, { op: "rename", name: r.name }).then((view) => {
      if (this.view?.id === r.id) this.view = { ...view, name: this.view.name };
    }));
    await this.refreshLibrary();
  }

  undo = () => this.history(false);
  redo = () => this.history(true);

  private async history(redo: boolean) {
    if (!this.view || !(redo ? this.canRedo : this.canUndo)) return;
    this.dismissToast();
    await this.flushRename();
    if (this.view) await this.apply(this.view.id, backend.undoEdit(this.view.id, redo));
  }

  deleteStep = async (index: number) => {
    const before = this.view;
    await this.edit({ op: "delete_step", index });
    if (this.view !== before && this.view?.can_undo) this.notify("Deleted the step", { label: "Undo", run: this.undo });
  };

  /** Sets the idle time before step `index`. */
  setPause = (index: number, ms: number) => this.edit({ op: "set_pause", index, dur: Math.max(0, Math.round(ms)) });

  /** Shortens every pause longer than 1 s to 1 s. */
  trimPauses = async () => {
    const n = this.longPauses;
    if (!n) return;
    const before = this.view;
    await this.edit({ op: "cap_pauses", max: TRIM_PAUSE_MS });
    if (this.view !== before) {
      this.notify(`Shortened ${plural(n, "pause")} to ${TRIM_PAUSE_MS / 1000} s`, { label: "Undo", run: this.undo });
    }
  };

  insertWait = () => this.edit({ op: "insert_wait", at: Math.round(this.cur), dur: 500, label: "Inserted" });

  /** Inserts a check at the playhead for the pixel under the macro's cursor, in its current color. */
  insertPixelCheck = async () => {
    const at = Math.round(this.cur);
    const p = this.cursorAt(this.cur);
    const x = Math.round(p.x);
    const y = Math.round(p.y);
    const color = (await backend.samplePixel(x, y).catch(() => null)) ?? "#EC3013";
    return this.edit({ op: "insert_pixel_wait", at, dur: 800, x, y, color, tolerance: 8, timeout_ms: 5000, label: "" });
  };

  /**
   * "Pick": counts down, reads the pixel under the real cursor, then calls
   * `use` if the same macro is still open (the user may have moved on).
   */
  private async pick(use: (p: PickedPixel) => Promise<unknown> | void) {
    const id = this.view?.id;
    if (!id || this.picking) return;
    this.picking = PICK_SECONDS;
    const countdown = setInterval(() => (this.picking = Math.max(1, this.picking - 1)), 1000);
    try {
      const p = await this.run(backend.pickPixel(PICK_SECONDS * 1000));
      if (p && this.view?.id === id) await use(p);
    } finally {
      clearInterval(countdown);
      this.picking = 0;
    }
  }

  /** Points the pixel check at step `index` at the pixel under the real cursor. */
  pickPixel = (index: number) =>
    this.pick((p) => {
      // Read the step again: it may have changed during the countdown.
      const step = this.steps[index];
      if (step?.kind !== "pixel_wait") return;
      const { tolerance, timeout_ms } = step;
      return this.edit({ op: "update_pixel_wait", index, x: p.x, y: p.y, color: p.color, tolerance, timeout_ms });
    });

  // — playback options —

  /** Changes the open macro's playback options and saves them. */
  setPlayback = (patch: Partial<PlaybackOptions>) => {
    if (!this.view) return;
    this.previewPlayback(patch);
    return this.apply(this.view.id, backend.setPlaybackOptions(this.view.id, this.view.playback));
  };

  /** Shows new playback options without saving them (while a slider moves). */
  previewPlayback = (patch: Partial<PlaybackOptions>) => {
    if (this.view) this.view = { ...this.view, playback: { ...this.view.playback, ...patch } };
  };

  // — triggers —

  /** Saves the open macro's triggers; a refused hotkey puts the old triggers back. */
  setTriggers = async (patch: Partial<MacroTriggers>) => {
    const id = this.view?.id;
    const current = this.triggerStatus;
    if (!id || !current) return;
    const next = { ...current.triggers, ...patch };
    this.triggerStatus = { ...current, triggers: next };
    try {
      const status = await backend.setTriggers(id, next);
      if (this.view?.id === id) this.triggerStatus = status;
      await this.refreshLibrary();
    } catch (e) {
      if (this.view?.id === id) this.triggerStatus = current;
      this.fail(e);
    }
  };

  setTriggersPaused = async (paused: boolean) => {
    this.triggersPaused = paused;
    await this.run(backend.setTriggersPaused(paused));
  };

  /** "Pick" for the pixel trigger: watch the pixel under the cursor. */
  pickTriggerPixel = () =>
    this.pick((p) => {
      const t = this.triggers;
      if (t) return this.setTriggers({ pixel: { ...t.pixel, x: p.x, y: p.y, color: p.color } });
    });

  loadProcesses = async () => {
    this.processes = (await backend.listProcesses().catch(() => null)) ?? this.processes;
  };

  // — settings —

  updateSettings = async (patch: Partial<Settings>) => {
    const before = this.settings;
    this.settings = { ...before, ...patch };
    const saved = await this.run(backend.updateSettings(this.settings));
    this.settings = saved ?? before;
  };

  setAutostart = async (enabled: boolean) => {
    this.autostart = (await this.run(backend.setAutostart(enabled))) ?? this.autostart;
  };

  // — export —

  doExport = async () => {
    if (!this.view) return;
    const path = await this.run(backend.exportMacro(this.view.id, this.exportFmt, this.exportName));
    if (!path) return; // cancelled or failed: keep the dialog open
    this.exportOpen = false;
    this.notify(`Saved ${path.split(/[\\/]/).pop()}`);
  };
}

export const relay = new RelayStore();

// A handle for debugging and end-to-end tests (`window.__relay` in DevTools).
(window as unknown as { __relay: RelayStore }).__relay = relay;
