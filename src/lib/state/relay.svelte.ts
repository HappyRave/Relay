// The app store. Macros, steps and edits come from relay-core through the
// backend; the session (mode, countdown, recording, playback clock) is owned
// by the Rust coordinator and streamed here as EngineMsg. The UI only
// extrapolates the playhead between ticks for smooth 60 fps motion.
import type {
  EditOp,
  ExportFormat,
  MacroListItem,
  MacroView,
  Mode,
  MovePoint,
  PlaybackOptions,
  Rect,
  Step,
  Tab,
  Triggers,
} from "../types";
import type { EngineMsg } from "../ipc/bindings/EngineMsg";
import type { Mode as EngineMode } from "../ipc/bindings/Mode";
import type { Settings } from "../ipc/bindings/Settings";
import type { TimingStats } from "../ipc/bindings/TimingStats";
import { backend, type IpcError } from "../ipc/backend";
import { isTauri } from "../platform/window";
import { lastIndexAtOrBefore } from "../preview/geometry";
import { jumpTarget } from "../timeline/lanes";
import { slug } from "../format";

const RENAME_DEBOUNCE_MS = 250;
/** How far past the last tick the playhead may run before the next one arrives. */
const MAX_EXTRAPOLATION_MS = 100;
const EMPTY_DESKTOP: Rect = { x: 0, y: 0, w: 1920, h: 1080 };
const DEFAULT_PLAYBACK: PlaybackOptions = {
  speed: 1,
  repeat: { count: 1 },
  humanize: true,
  jitter_ms: 40,
  stop_on_key: true,
  coord_mode: "screen",
};
const DEFAULT_SETTINGS: Settings = {
  capture_moves: true,
  capture_keys: true,
  countdown: true,
  ignore_injected: true,
  path_mode: "full",
  show_click_labels: true,
};
const MODES: Record<EngineMode, Mode> = {
  idle: "idle",
  countdown: "count",
  recording: "rec",
  playing: "play",
  paused: "pause",
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
  settings = $state<Settings>(DEFAULT_SETTINGS);
  library = $state.raw<MacroListItem[]>([]);
  view = $state.raw<MacroView | null>(null);
  /** The last failed command or engine notice, shown briefly in the Steps tab. */
  error = $state<string | null>(null);
  /** Injection timing of the last completed playback (for diagnostics). */
  lastTiming: TimingStats | null = null;
  readonly editable = backend.editable;

  /** Set by the widget (M6 uses it for click-through during playback). */
  widgetEl: HTMLElement | null = null;

  private triggersById = $state<Record<string, Triggers>>({});
  private recMoves = $state.raw<MovePoint[]>([]);
  private recSteps = $state.raw<Step[]>([]);
  private recDesktop = $state.raw<Rect>(EMPTY_DESKTOP);
  /** The last playback or recording tick, for extrapolation. */
  private tick = { t: 0, at: 0, speed: 1, advancing: false };
  private raf = 0;
  private cleanup: (() => void)[] = [];
  private editSeq = 0;
  private renameTimer: ReturnType<typeof setTimeout> | undefined;
  private pendingName: string | null = null;
  private errorTimer: ReturnType<typeof setTimeout> | undefined;
  /** A just-saved recording to open once the session is back to idle. */
  private pendingLoad: string | null = null;

  recording = $derived(this.mode === "rec" || this.mode === "count");
  name = $derived(this.view?.name ?? "");
  playback: PlaybackOptions = $derived(this.view?.playback ?? DEFAULT_PLAYBACK);
  loops = $derived(this.playback.repeat === "forever" ? Infinity : this.playback.repeat.count);
  steps: Step[] = $derived(this.mode === "rec" ? this.recSteps : (this.view?.steps ?? []));
  moves: MovePoint[] = $derived(this.mode === "rec" ? this.recMoves : (this.view?.moves ?? []));
  duration = $derived(this.mode === "rec" ? Math.max(this.cur, 2000) : (this.view?.duration ?? 2000));
  desktop: Rect = $derived(
    this.mode === "rec" ? this.recDesktop : (this.view?.recording.virtual_desktop ?? EMPTY_DESKTOP),
  );
  /** Outlines for the preview: the monitors and the window the macro is anchored to. */
  frames: Rect[] = $derived.by(() => {
    const r = this.view?.recording;
    if (!r || this.mode === "rec") return [];
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
    await backend.subscribe(this.onEngine);
    try {
      this.settings = await backend.getSettings();
    } catch (e) {
      this.fail(e);
    }
    await this.refreshLibrary();
    if (this.library[0]) await this.loadMacro(this.library[0].id);
  }

  start() {
    const loop = (now: number) => {
      this.frame(now);
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

  /** Moves the playhead smoothly between engine ticks. */
  private frame(now: number) {
    if ((this.mode !== "play" && this.mode !== "rec") || !this.tick.advancing) return;
    const ahead = Math.min(now - this.tick.at, MAX_EXTRAPOLATION_MS) * this.tick.speed;
    this.cur = this.tick.t + ahead;
  }

  private onEngine = (msg: EngineMsg) => {
    const now = performance.now();
    switch (msg.type) {
      case "session": {
        const mode = MODES[msg.mode];
        if (mode === "rec") {
          this.recMoves = [];
          this.recSteps = [];
          this.cur = 0;
          this.tick = { t: 0, at: now, speed: 1, advancing: true };
        }
        if (mode === "count") this.cur = 0;
        if (mode === "idle") this.tick.advancing = false;
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
        if (msg.moves.length) this.recMoves = [...this.recMoves, ...msg.moves];
        if (msg.steps) this.recSteps = msg.steps;
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
        // Any stop rewinds (Stop, Esc, a key press, the kill switch); a completed run stays at the end.
        if (msg.reason !== "completed") this.cur = 0;
        break;
      case "saved":
        // Arrives just before the session returns to idle.
        if (this.mode === "idle") this.loadMacro(msg.id);
        else this.pendingLoad = msg.id;
        break;
      case "library_changed":
        this.refreshLibrary();
        break;
      case "toggle_compact":
        this.expanded = !this.expanded;
        break;
      case "error":
      case "notice":
        this.fail({ code: msg.type, message: msg.message });
        break;
    }
  };

  // In the app F9/F10 are global hotkeys handled in Rust; the browser preview
  // has no global hotkeys, so it listens here. Esc also closes the dialog.
  private onKey = (e: KeyboardEvent) => {
    if (e.key === "Escape") {
      if (this.exportOpen) this.exportOpen = false;
      else if (!isTauri() && this.mode !== "idle") this.stop();
      return;
    }
    if (isTauri()) return;
    if (e.key === "F9") {
      e.preventDefault();
      this.toggleRec();
    } else if (e.key === "F10") {
      e.preventDefault();
      this.togglePlay();
    } else if (e.ctrlKey && e.shiftKey && e.key.toLowerCase() === "m" && this.mode === "idle") {
      e.preventDefault();
      this.expanded = !this.expanded;
    }
  };

  // — session —

  toggleRec = () => this.call(backend.toggleRecord());

  togglePlay = () => {
    if (this.recording || !this.view) return;
    return this.call(backend.togglePlay(this.cur >= this.duration - 1 ? 0 : this.cur));
  };

  stop = () => this.call(backend.stop());

  seek = (t: number) => {
    if (this.recording) return;
    const clamped = Math.max(0, Math.min(this.duration, t));
    this.cur = clamped;
    this.tick = { ...this.tick, t: clamped, at: performance.now() };
    return this.call(backend.seek(clamped));
  };

  jump = (dir: -1 | 1) => this.seek(jumpTarget(this.steps, this.cur, dir, this.duration));

  private async call(p: Promise<void>) {
    try {
      await p;
    } catch (e) {
      this.fail(e);
    }
  }

  /** Cursor position at time `t` (virtual-desktop px). */
  cursorAt(t: number): { x: number; y: number } {
    const i = lastIndexAtOrBefore(this.moves, t);
    const m = this.moves[Math.max(0, i)];
    if (m) return m;
    const d = this.desktop;
    return { x: d.x + d.w / 2, y: d.y + d.h / 2 };
  }

  updateSettings = (patch: Partial<Settings>) => {
    this.settings = { ...this.settings, ...patch };
    return backend.updateSettings(this.settings).then(
      (s) => void (this.settings = s),
      (e) => this.fail(e),
    );
  };

  // — library & edits —

  async refreshLibrary() {
    try {
      this.library = await backend.listMacros();
    } catch (e) {
      this.fail(e);
    }
  }

  loadMacro = async (id: string) => {
    if (this.recording || this.mode === "play" || this.mode === "pause") return;
    try {
      const view = await backend.loadMacro(id);
      this.editSeq++;
      this.view = view;
      this.cur = 0;
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

// A handle for debugging and end-to-end tests (`window.__relay` in DevTools).
(window as unknown as { __relay: RelayStore }).__relay = relay;
