// The app store. Macros, steps and edits come from relay-core through the
// backend (Tauri commands in the app, a read-only fixture in the browser).
// The session clock is still simulated here: recording moves to the Rust
// engine in M2 and playback in M3.
import type {
  EditOp,
  ExportFormat,
  MacroListItem,
  MacroView,
  Mode,
  MovePoint,
  PlaybackOptions,
  Rect,
  Settings,
  Step,
  Tab,
  Triggers,
} from "../types";
import { backend, type IpcError } from "../ipc/backend";
import { lastIndexAtOrBefore } from "../preview/geometry";
import { jumpTarget } from "../timeline/lanes";
import { slug } from "../format";

const COUNTDOWN_MS = 3000;
const RENAME_DEBOUNCE_MS = 250;
const EMPTY_DESKTOP: Rect = { x: 0, y: 0, w: 1920, h: 1080 };
const DEFAULT_PLAYBACK: PlaybackOptions = {
  speed: 1,
  repeat: { count: 1 },
  humanize: true,
  jitter_ms: 40,
  stop_on_key: true,
  coord_mode: "screen",
};

const defaultTriggers = (hotkey: string | null): Triggers => ({
  hotkey: { enabled: hotkey != null, combo: hotkey ?? "—" },
  schedule: { enabled: false, days: [true, true, true, true, true, false, false], time: "09:00" },
  appLaunch: { enabled: false, exe: "EXCEL.EXE" },
  pixel: { enabled: false, x: 1210, y: 612, color: "#EC3013" },
});

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
  library = $state.raw<MacroListItem[]>([]);
  view = $state.raw<MacroView | null>(null);
  /** The last failed command, shown briefly in the Steps tab. */
  error = $state<string | null>(null);
  readonly editable = backend.editable;

  /** Set by the widget; used to keep the widget's own input out of recordings (M2). */
  widgetEl: HTMLElement | null = null;

  private triggersById = $state<Record<string, Triggers>>({});
  private recStart = 0;
  private raf = 0;
  private lastFrame = 0;
  private cleanup: (() => void)[] = [];
  private editSeq = 0;
  private renameTimer: ReturnType<typeof setTimeout> | undefined;
  private pendingName: string | null = null;
  private errorTimer: ReturnType<typeof setTimeout> | undefined;

  recording = $derived(this.mode === "rec" || this.mode === "count");
  name = $derived(this.view?.name ?? "");
  playback: PlaybackOptions = $derived(this.view?.playback ?? DEFAULT_PLAYBACK);
  loops = $derived(this.playback.repeat === "forever" ? Infinity : this.playback.repeat.count);
  /** Nothing is captured until M2, so a recording session shows an empty macro. */
  steps: Step[] = $derived(this.mode === "rec" || !this.view ? [] : this.view.steps);
  moves: MovePoint[] = $derived(this.mode === "rec" || !this.view ? [] : this.view.moves);
  duration = $derived(this.mode === "rec" ? Math.max(this.cur, 2000) : (this.view?.duration ?? 2000));
  desktop: Rect = $derived(this.view?.recording.virtual_desktop ?? EMPTY_DESKTOP);
  /** Outlines for the preview: the monitors and the window the macro is anchored to. */
  frames: Rect[] = $derived.by(() => {
    const r = this.view?.recording;
    if (!r) return [];
    const out = r.monitors.map((m) => m.rect);
    if (r.anchor_window) out.push(r.anchor_window.rect);
    return out;
  });
  triggers: Triggers = $derived(
    (this.view && this.triggersById[this.view.id]) ??
      defaultTriggers(this.library.find((m) => m.id === this.view?.id)?.hotkey ?? null),
  );

  // — lifecycle —

  async init() {
    await this.refreshLibrary();
    if (this.library[0]) await this.loadMacro(this.library[0].id);
  }

  start() {
    this.lastFrame = performance.now();
    const loop = (now: number) => {
      const dt = Math.min(64, now - this.lastFrame);
      this.lastFrame = now;
      this.tick(dt, now);
      this.raf = requestAnimationFrame(loop);
    };
    this.raf = requestAnimationFrame(loop);
    window.addEventListener("keydown", this.onKey, true);
    this.cleanup.push(() => window.removeEventListener("keydown", this.onKey, true));
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
    } else if (this.mode === "play") {
      const c = this.cur + dt * this.playback.speed;
      const d = this.duration;
      if (c < d) this.cur = c;
      else if (this.loopIdx + 1 < this.loops) {
        this.cur = 0;
        this.loopIdx++;
      } else {
        this.mode = "idle";
        this.cur = d;
        this.loopIdx = 0;
      }
    }
  }

  // Window-local shortcuts; the global hotkeys arrive with the engine (M2/M3).
  private onKey = (e: KeyboardEvent) => {
    if (e.key === "F9") {
      e.preventDefault();
      this.toggleRec();
    } else if (e.key === "F10") {
      e.preventDefault();
      this.togglePlay();
    } else if (e.key === "Escape") {
      if (this.exportOpen) this.exportOpen = false;
      else if (this.mode !== "idle") this.stop();
    } else if (e.ctrlKey && e.shiftKey && e.key.toLowerCase() === "m" && this.mode === "idle") {
      e.preventDefault();
      this.expanded = !this.expanded;
    }
  };

  // — session (simulated until M2/M3) —

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
    this.recStart = performance.now();
    this.mode = "rec";
    this.cur = 0;
    this.countLeft = 0;
  }

  private stopRec() {
    this.mode = "idle";
    this.cur = 0;
  }

  togglePlay = () => {
    if (this.recording || !this.view) return;
    if (this.mode === "play") {
      this.mode = "pause";
      return;
    }
    if (this.mode !== "pause") this.loopIdx = 0;
    if (this.cur >= this.duration - 1) this.cur = 0;
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
    this.cur = jumpTarget(this.steps, this.cur, dir, this.duration);
  };

  /** Cursor position at time `t` (virtual-desktop px). */
  cursorAt(t: number): { x: number; y: number } {
    const i = lastIndexAtOrBefore(this.moves, t);
    const m = this.moves[Math.max(0, i)];
    if (m) return m;
    const d = this.desktop;
    return { x: d.x + d.w / 2, y: d.y + d.h / 2 };
  }

  // — library & edits —

  async refreshLibrary() {
    try {
      this.library = await backend.listMacros();
    } catch (e) {
      this.fail(e);
    }
  }

  loadMacro = async (id: string) => {
    if (this.recording) return;
    try {
      const view = await backend.loadMacro(id);
      this.editSeq++;
      this.view = view;
      this.cur = 0;
      this.mode = "idle";
      this.loopIdx = 0;
      this.tab = "events";
    } catch (e) {
      this.fail(e);
    }
  };

  /** Applies a command's result, unless a newer command has started since. */
  private async apply(request: Promise<MacroView>) {
    const seq = ++this.editSeq;
    try {
      let view = await request;
      if (seq !== this.editSeq) return;
      if (this.pendingName != null) view = { ...view, name: this.pendingName };
      this.view = view;
      await this.refreshLibrary();
    } catch (e) {
      this.fail(e);
    }
  }

  edit = (op: EditOp) => {
    if (!this.view || this.recording) return;
    if (!this.editable) return this.fail({ code: "unavailable", message: "Editing needs the Relay app" });
    return this.apply(backend.editMacro(this.view.id, op));
  };

  rename = (name: string) => {
    if (!this.view) return;
    this.view = { ...this.view, name };
    if (!this.editable) return;
    this.pendingName = name;
    clearTimeout(this.renameTimer);
    this.renameTimer = setTimeout(async () => {
      await this.edit({ op: "rename", name });
      if (this.pendingName === name) this.pendingName = null;
    }, RENAME_DEBOUNCE_MS);
  };

  setPlayback = (patch: Partial<PlaybackOptions>) => {
    if (!this.view) return;
    const options = { ...this.view.playback, ...patch };
    this.view = { ...this.view, playback: options };
    return this.apply(backend.setPlaybackOptions(this.view.id, options));
  };

  setTriggers = (patch: Partial<Triggers>) => {
    if (!this.view) return;
    this.triggersById = { ...this.triggersById, [this.view.id]: { ...this.triggers, ...patch } };
  };

  deleteStep = (index: number) => this.edit({ op: "delete_step", index });

  insertWait = () => this.edit({ op: "insert_wait", at: Math.round(this.cur), dur: 500, label: "Inserted" });

  insertPixelCheck = () => {
    const p = this.cursorAt(this.cur);
    return this.edit({
      op: "insert_pixel_wait",
      at: Math.round(this.cur),
      dur: 800,
      x: p.x,
      y: p.y,
      color: "#EC3013",
      tolerance: 8,
      timeout_ms: 5000,
      label: "Inserted",
    });
  };

  private fail(e: unknown) {
    const message = (e as IpcError)?.message ?? String(e);
    console.error("Relay:", e);
    this.error = message;
    clearTimeout(this.errorTimer);
    this.errorTimer = setTimeout(() => (this.error = null), 4000);
  }

  // — export (M5 replaces the download with a native save dialog) —

  exportName = $derived(slug(this.name) + "." + this.exportFmt);

  doExport = async () => {
    if (!this.view) return;
    try {
      const text = await backend.exportText(this.view.id, this.exportFmt);
      const a = document.createElement("a");
      a.href = URL.createObjectURL(new Blob([text], { type: "application/json" }));
      a.download = this.exportName;
      a.click();
      URL.revokeObjectURL(a.href);
      this.exportOpen = false;
    } catch (e) {
      this.fail(e);
    }
  };
}

export const relay = new RelayStore();
