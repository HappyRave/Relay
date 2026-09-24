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
  MacroTriggers,
  TriggerStatus,
} from "../types";
import type { EngineMsg } from "../ipc/bindings/EngineMsg";
import type { Mode as EngineMode } from "../ipc/bindings/Mode";
import type { Settings } from "../ipc/bindings/Settings";
import type { TimingStats } from "../ipc/bindings/TimingStats";
import { backend, type IpcError } from "../ipc/backend";
import { isTauri, savedExpanded } from "../platform/window";
import { lastIndexAtOrBefore } from "../preview/geometry";
import { jumpTarget } from "../timeline/lanes";
import { slug } from "../format";

const RENAME_DEBOUNCE_MS = 250;

export interface Toast {
  kind: "error" | "info";
  message: string;
  action?: { label: string; run: () => void };
}
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
  close_to_tray: true,
};
const MODES: Record<EngineMode, Mode> = {
  idle: "idle",
  countdown: "count",
  recording: "rec",
  playing: "play",
  paused: "pause",
};

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
  /** A short message at the bottom of the side panel, optionally with an action (Undo). */
  toast = $state<Toast | null>(null);
  /** The current error message, if the toast is an error (for tests and debugging). */
  error = $derived(this.toast?.kind === "error" ? this.toast.message : null);
  /** Injection timing of the last completed playback (for diagnostics). */
  lastTiming: TimingStats | null = null;
  readonly editable = backend.editable;

  /** Set by the widget (M6 uses it for click-through during playback). */
  widgetEl: HTMLElement | null = null;

  /** The open macro's triggers, with the next scheduled run and hotkey problems. */
  triggerStatus = $state.raw<TriggerStatus | null>(null);
  /** Paused by the kill switch (or from the tray). */
  triggersPaused = $state(false);
  autostart = $state(false);
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
  private toastTimer: ReturnType<typeof setTimeout> | undefined;
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
  triggers: MacroTriggers | null = $derived(this.triggerStatus?.triggers ?? null);

  // — lifecycle —

  /** False until the saved window mode is known (so the widget never flashes the wrong size). */
  ready = $state(false);

  async init() {
    this.expanded = await savedExpanded().catch(() => true);
    this.ready = true;
    await backend.subscribe(this.onEngine);
    try {
      this.settings = await backend.getSettings();
    } catch (e) {
      this.fail(e);
    }
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
        // A trigger started another macro: show the one that's playing.
        if (mode === "play" && msg.macro_id && msg.macro_id !== this.view?.id) this.showMacro(msg.macro_id);
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
    await this.showMacro(id);
    this.tab = "events";
  };

  /** Opens a macro in the editor (also mid-playback, when a trigger started it). */
  private async showMacro(id: string) {
    try {
      const view = await backend.loadMacro(id);
      this.editSeq++;
      this.view = view;
      this.cur = 0;
      this.loopIdx = 0;
      await this.loadTriggers(id);
    } catch (e) {
      this.fail(e);
    }
  }

  private async loadTriggers(id: string) {
    try {
      const status = await backend.getTriggers(id);
      if (this.view?.id === id) {
        this.triggerStatus = status;
        this.triggersPaused = status.paused;
      }
    } catch (e) {
      this.fail(e);
    }
  }

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

  /** Saves the open macro's triggers; a refused hotkey puts the old triggers back. */
  setTriggers = async (patch: Partial<MacroTriggers>) => {
    const id = this.view?.id;
    const current = this.triggerStatus;
    if (!id || !current) return;
    const next = { ...current.triggers, ...patch };
    this.triggerStatus = { ...current, triggers: next };
    try {
      this.triggerStatus = await backend.setTriggers(id, next);
      await this.refreshLibrary();
    } catch (e) {
      this.triggerStatus = current;
      this.fail(e);
    }
  };

  setTriggersPaused = async (paused: boolean) => {
    this.triggersPaused = paused;
    await backend.setTriggersPaused(paused).catch((e) => this.fail(e));
  };

  /** "Pick" for the pixel trigger: after 3 s, watch the pixel under the cursor. */
  pickTriggerPixel = async () => {
    const t = this.triggers;
    if (!t || this.picking) return;
    this.picking = 3;
    const tick = setInterval(() => (this.picking = Math.max(1, this.picking - 1)), 1000);
    try {
      const p = await backend.pickPixel(3000);
      await this.setTriggers({ pixel: { ...t.pixel, x: p.x, y: p.y, color: p.color } });
    } catch (e) {
      this.fail(e);
    } finally {
      clearInterval(tick);
      this.picking = 0;
    }
  };

  setAutostart = async (enabled: boolean) => {
    try {
      this.autostart = await backend.setAutostart(enabled);
    } catch (e) {
      this.fail(e);
    }
  };

  deleteStep = (index: number) => this.edit({ op: "delete_step", index });

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

  /** Seconds left before "Pick" samples the cursor, or 0 when not picking. */
  picking = $state(0);

  /** After a 3 s countdown, points the pixel check at the pixel under the real cursor. */
  pickPixel = async (index: number) => {
    const step = this.steps[index];
    if (step?.kind !== "pixel_wait" || this.picking) return;
    this.picking = 3;
    const tick = setInterval(() => (this.picking = Math.max(1, this.picking - 1)), 1000);
    try {
      const p = await backend.pickPixel(3000);
      await this.edit({
        op: "update_pixel_wait",
        index,
        x: p.x,
        y: p.y,
        color: p.color,
        tolerance: step.tolerance,
        timeout_ms: step.timeout_ms,
      });
    } catch (e) {
      this.fail(e);
    } finally {
      clearInterval(tick);
      this.picking = 0;
    }
  };

  private show(toast: Toast, ms: number) {
    this.toast = toast;
    clearTimeout(this.toastTimer);
    this.toastTimer = setTimeout(() => (this.toast = null), ms);
  }

  notify(message: string, action?: Toast["action"], ms = 5000) {
    this.show({ kind: "info", message, action }, action ? Math.max(ms, 8000) : ms);
  }

  dismissToast = () => {
    clearTimeout(this.toastTimer);
    this.toast = null;
  };

  private fail(e: unknown) {
    const message = (e as IpcError)?.message ?? String(e);
    console.error("Relay:", e);
    this.show({ kind: "error", message }, 5000);
  }

  // — library management —

  duplicateMacro = async (id: string) => {
    try {
      const copy = await backend.duplicateMacro(id);
      await this.refreshLibrary();
      await this.loadMacro(copy);
      this.tab = "lib";
    } catch (e) {
      this.fail(e);
    }
  };

  /** Moves a macro to the trash, with Undo. Opens a neighbour if it was the open one. */
  deleteMacro = async (id: string) => {
    const idx = this.library.findIndex((m) => m.id === id);
    const name = this.library[idx]?.name ?? "macro";
    try {
      await backend.deleteMacro(id);
      await this.refreshLibrary();
      if (this.view?.id === id) {
        const next = this.library[Math.min(idx, this.library.length - 1)];
        if (next) await this.loadMacro(next.id);
        else this.view = null;
        this.tab = "lib";
      }
      this.notify(`Moved “${name}” to the trash`, { label: "Undo", run: () => this.restoreMacro(id) });
    } catch (e) {
      this.fail(e);
    }
  };

  restoreMacro = async (id: string) => {
    this.dismissToast();
    try {
      await backend.restoreMacro(id);
      await this.refreshLibrary();
      await this.loadMacro(id);
      this.tab = "lib";
    } catch (e) {
      this.fail(e);
    }
  };

  importMacros = async () => {
    try {
      const result = await backend.importMacros();
      if (!result) return;
      await this.refreshLibrary();
      if (result.imported[0]) await this.loadMacro(result.imported[0]);
      this.tab = "lib";
      const n = result.imported.length;
      const done = n ? `Imported ${n} macro${n === 1 ? "" : "s"}` : "Nothing imported";
      if (result.problems.length) this.fail({ code: "import", message: `${done}. ${result.problems.join("; ")}` });
      else this.notify(done);
    } catch (e) {
      this.fail(e);
    }
  };

  // — export —

  exportName = $derived(slug(this.name) + "." + this.exportFmt);

  doExport = async () => {
    if (!this.view) return;
    try {
      const path = await backend.exportMacro(this.view.id, this.exportFmt, this.exportName);
      if (!path) return; // cancelled: keep the dialog open
      this.exportOpen = false;
      this.notify(`Saved ${path.split(/[\\/]/).pop()}`);
    } catch (e) {
      this.fail(e);
    }
  };
}

export const relay = new RelayStore();

// A handle for debugging and end-to-end tests (`window.__relay` in DevTools).
(window as unknown as { __relay: RelayStore }).__relay = relay;
