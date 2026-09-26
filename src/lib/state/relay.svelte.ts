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
import type { FinishReason } from "../ipc/bindings/FinishReason";
import { backend as defaultBackend, type Backend, type IpcError } from "../ipc/backend";
import { DEFAULT_PLAYBACK, DEFAULT_SETTINGS } from "../defaults";
import { isTauri, savedExpanded } from "../platform/window";
import { lastIndexAtOrBefore } from "../preview/geometry";
import { currentStepIndex, jumpTarget } from "../timeline/lanes";
import { plural, slug } from "../format";
import { followStep } from "./selection";

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
  /**
   * For an Undo of an edit: the macro it was made to. The next edit would
   * make it undo something else, so edits withdraw it, and so does opening
   * another macro.
   */
  undoes?: string;
}

export class RelayStore {
  private readonly backend: Backend;

  constructor(backend: Backend = defaultBackend) {
    this.backend = backend;
  }

  get editable(): boolean {
    return this.backend.editable;
  }

  // — session (from the engine stream) —
  mode = $state<Mode>("idle");
  /** The playhead, in macro ms. */
  cur = $state(0);
  countLeft = $state(0);
  loopIdx = $state(0);
  /** The loops (null: forever) and speed of the playback running, from its ticks; null before the first. */
  playInfo = $state.raw<{ loops: number | null; speed: number } | null>(null);
  /** How and with what timing the last playback ended (for diagnostics and the end-to-end tests). */
  lastFinish: FinishReason | null = null;
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
  /** The row whose step editor is open, or -1. It follows its step across edits (see selection.ts). */
  selected = $state(-1);

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
  /** A name being typed, not saved yet (no timer when it's blank: it won't be). */
  private rename_: { id: string; name: string; timer: ReturnType<typeof setTimeout> | undefined } | null = null;
  /** The open macro's name as Rust has it, for when a blank one is abandoned. */
  private savedName = "";
  private toastTimer: ReturnType<typeof setTimeout> | undefined;
  // Settings, triggers and their pause are saved optimistically: a change
  // shows at once, only the newest request's answer is shown, and when that
  // fails the last state Rust confirmed comes back (an older answer, or a
  // failure, never overwrites a newer change).
  private settingsSaves = { seq: 0, savedSeq: 0, saved: DEFAULT_SETTINGS, failed: false };
  private triggerSaves = { seq: 0, savedSeq: 0, saved: null as TriggerStatus | null, failed: false };
  private pauseSaves = { seq: 0, saved: false };
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
  /** Whether the open macro can be edited now: in the app, with a macro open, while idle (Rust refuses edits during a session). */
  canEdit = $derived(this.editable && this.mode === "idle" && !!this.view);
  canUndo = $derived(this.canEdit && !!this.view?.can_undo);
  canRedo = $derived(this.canEdit && !!this.view?.can_redo);
  /** Pauses "Trim pauses" would shorten. */
  longPauses = $derived(this.steps.filter((s) => s.pause > TRIM_PAUSE_MS).length);
  error = $derived(this.toast?.kind === "error" ? this.toast.message : null);
  exportName = $derived((slug(this.name) || "macro") + "." + this.exportFmt);

  // — lifecycle —

  async init() {
    this.expanded = await savedExpanded().catch(() => true);
    this.ready = true;
    await this.run(this.backend.subscribe(this.onEngine));
    this.settings = (await this.run(this.backend.getSettings())) ?? this.settings;
    this.settingsSaves.saved = this.settings;
    await this.refreshLibrary();
    if (this.library[0]) await this.loadMacro(this.library[0].id);
    this.autostart = await this.backend.getAutostart().catch(() => false);
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
    cancelAnimationFrame(this.seekFrame);
    this.seekFrame = 0;
    if (this.listening) window.removeEventListener("keydown", this.onKey, true);
    this.listening = false;
    clearTimeout(this.toastTimer);
    // A name still being typed is dropped: the store is going away.
    if (this.rename_) clearTimeout(this.rename_.timer);
    this.rename_ = null;
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
          this.playInfo = null;
          this.recMoves = [];
          this.recSteps = [];
        }
        this.mode = mode;
        if (mode === "idle" && this.pendingLoad) {
          this.loadMacro(this.pendingLoad);
          this.pendingLoad = null;
        } else if (mode === "idle" && this.rename_) {
          this.flushRename(); // typed just before the session started
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
        if (this.playInfo?.loops !== msg.loops || this.playInfo?.speed !== msg.speed) {
          this.playInfo = { loops: msg.loops, speed: msg.speed };
        }
        if (!msg.advancing) this.cur = msg.t;
        break;
      case "finished":
        // The playhead stays where this leaves it: a frame before the session's
        // idle message mustn't carry on from the last tick.
        this.tick.advancing = false;
        this.loopIdx = 0;
        this.lastFinish = msg.reason;
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
        this.triggersPaused = this.pauseSaves.saved = msg.paused;
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
    this.show({ kind: "error", message });
  }

  notify(message: string, action?: Toast["action"], ms = 5000) {
    this.show({ kind: "info", message, action }, action ? Math.max(ms, 8000) : ms);
  }

  /** Offers to undo the edit just made to macro `id`; the action only undoes on that macro. */
  private offerUndo(message: string, id: string) {
    const run = () => void (this.view?.id === id && this.undo());
    this.show({ kind: "info", message, action: { label: "Undo", run }, undoes: id }, 8000);
  }

  /** An edit is starting: an Undo offered for an earlier one would now undo this one instead. */
  private withdrawUndo() {
    if (this.toast?.undoes) this.dismissToast();
  }

  dismissToast = () => {
    clearTimeout(this.toastTimer);
    this.toast = null;
  };

  /** Shows a toast, for `ms` or, without it (errors), until it's dismissed or replaced. */
  private show(toast: Toast, ms?: number) {
    this.toast = toast;
    clearTimeout(this.toastTimer);
    if (ms != null) this.toastTimer = setTimeout(() => (this.toast = null), ms);
  }

  // — session —

  toggleRec = () => {
    if (this.playing) return;
    return this.run(this.backend.toggleRecord());
  };

  togglePlay = () => {
    if (this.recording || !this.view) return;
    return this.run(this.backend.togglePlay(this.cur >= this.duration - 1 ? 0 : this.cur));
  };

  stop = () => this.run(this.backend.stop());

  /** Moves the playhead now; the engine hears about it at most once per frame. */
  seek = (t: number) => {
    if (this.recording) return;
    const clamped = Math.max(0, Math.min(this.duration, t));
    this.cur = clamped;
    this.seekTo = clamped;
    this.tick = { ...this.tick, t: clamped, at: performance.now() };
    if (!this.seekFrame) {
      // The target, not the playhead: while playing, that's already moving on.
      this.seekFrame = requestAnimationFrame(() => {
        this.seekFrame = 0;
        this.run(this.backend.seek(this.seekTo));
      });
    }
  };
  /** The frame that will send the latest seek, or 0. */
  private seekFrame = 0;
  private seekTo = 0;

  jump = (dir: -1 | 1) => this.seek(jumpTarget(this.steps, this.cur, dir, this.duration));

  /** A click on a step's row: the playhead goes to it, and its editor opens (or closes). */
  selectStep = (index: number) => {
    const s = this.steps[index];
    if (!s) return;
    this.seek(s.t);
    this.selected = this.selected === index ? -1 : index;
  };

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
    this.library = (await this.run(this.backend.listMacros())) ?? this.library;
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
    const view = await this.run(this.backend.loadMacro(id));
    if (!view || seq !== this.viewSeq) return;
    // An Undo offered for the macro left behind mustn't stay up over this one.
    if (this.toast?.undoes && this.toast.undoes !== id) this.dismissToast();
    this.view = view;
    this.savedName = view.name;
    this.selected = -1;
    this.triggerStatus = null; // the old macro's triggers mustn't be edited into this one
    this.cur = 0;
    this.loopIdx = 0;
    const saves = this.triggerSaves;
    const loading = ++saves.seq; // answers to changes made to the macro left behind are dropped
    saves.failed = false;
    const status = await this.run(this.backend.getTriggers(id));
    if (status && this.view?.id === id && loading === saves.seq) {
      this.triggerStatus = saves.saved = status;
      saves.savedSeq = loading;
      this.triggersPaused = this.pauseSaves.saved = status.paused;
    }
  }

  duplicateMacro = async (id: string) => {
    await this.flushRename(); // the copy is named after the name being typed
    const copy = await this.run(this.backend.duplicateMacro(id));
    if (!copy) return;
    await this.refreshLibrary();
    await this.loadMacro(copy);
    this.tab = "library";
  };

  /** Moves a macro to the trash, with Undo. Opens a neighbour if it was the open one. */
  deleteMacro = async (id: string) => {
    await this.flushRename();
    const wasOpen = this.view?.id === id;
    const idx = this.library.findIndex((m) => m.id === id);
    const name = this.library[idx]?.name ?? "macro";
    try {
      await this.backend.deleteMacro(id);
    } catch (e) {
      return this.fail(e);
    }
    await this.refreshLibrary();
    if (this.view?.id === id) {
      const next = this.library[Math.min(idx, this.library.length - 1)];
      if (next) await this.loadMacro(next.id);
      else {
        this.view = null;
        this.triggerStatus = null;
      }
      this.tab = "library";
    }
    this.notify(`Moved “${name}” to the trash`, { label: "Undo", run: () => this.restoreMacro(id, wasOpen) });
  };

  /** Brings a macro back from the trash; `reopen` it when it was the open one. */
  restoreMacro = async (id: string, reopen = false) => {
    this.dismissToast();
    try {
      await this.backend.restoreMacro(id);
    } catch (e) {
      return this.fail(e);
    }
    await this.refreshLibrary();
    if (!reopen) return; // stay on the macro and tab the user is on
    await this.loadMacro(id);
    this.tab = "library";
  };

  importMacros = async () => {
    const result = await this.run(this.backend.importMacros());
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
    if (!this.view || this.mode !== "idle") return;
    if (!this.editable) return this.fail({ code: "unavailable", message: "Editing needs the Relay app" });
    this.withdrawUndo();
    return this.apply(this.view.id, this.backend.editMacro(this.view.id, op), op);
  };

  /**
   * Shows a command's result for macro `id` (from `op`, when it's an edit),
   * unless another request replaced the view since.
   */
  private async apply(id: string, request: Promise<MacroView>, op?: EditOp) {
    const seq = ++this.viewSeq;
    let view = await this.run(request);
    if (!view || seq !== this.viewSeq || this.view?.id !== id) return;
    this.savedName = view.name;
    const row = this.library.find((m) => m.id === id);
    const listChanged = !row || row.name !== view.name || row.duration !== view.duration || row.step_count !== view.steps.length;
    // A name still being typed wins over the one in the response.
    if (this.rename_?.id === id) view = { ...view, name: this.rename_.name };
    this.selected = followStep(this.view.steps, view.steps, this.selected, op);
    this.view = view;
    if (listChanged) await this.refreshLibrary();
  }

  /**
   * Renames the open macro as the user types; saved after a short pause. A
   * blank name is shown but not saved: leaving the field (`endRename`) puts
   * the saved one back.
   */
  rename = (name: string) => {
    if (!this.view || this.mode !== "idle") return;
    if (!this.editable) return this.fail({ code: "unavailable", message: "Editing needs the Relay app" });
    const id = this.view.id;
    this.withdrawUndo();
    this.view = { ...this.view, name };
    if (this.rename_) clearTimeout(this.rename_.timer);
    const timer = name.trim() ? setTimeout(() => this.flushRename(), RENAME_DEBOUNCE_MS) : undefined;
    this.rename_ = { id, name, timer };
  };

  /** The name field lost focus: a blank name goes back to the saved one. */
  endRename = () => {
    if (this.rename_ && !this.rename_.name.trim()) this.flushRename();
  };

  /**
   * Saves a pending rename now (before switching macros, undoing, …). During
   * a session Rust would refuse it, so it waits for the session to end.
   */
  private async flushRename() {
    const r = this.rename_;
    if (!r || this.mode !== "idle") return;
    clearTimeout(r.timer);
    this.rename_ = null;
    if (!r.name.trim()) {
      if (this.view?.id === r.id) this.view = { ...this.view, name: this.savedName };
      return;
    }
    const op: EditOp = { op: "rename", name: r.name };
    if (this.view?.id === r.id) return this.apply(r.id, this.backend.editMacro(r.id, op), op);
    // Another macro was opened meanwhile (a trigger started it): save, and show the new name in the list.
    await this.run(this.backend.editMacro(r.id, op));
    await this.refreshLibrary();
  }

  undo = () => this.history(false);
  redo = () => this.history(true);

  private async history(redo: boolean) {
    if (!this.view || !(redo ? this.canRedo : this.canUndo)) return;
    this.withdrawUndo();
    await this.flushRename();
    if (this.view) await this.apply(this.view.id, this.backend.undoEdit(this.view.id, redo));
  }

  deleteStep = async (index: number) => {
    const before = this.view;
    await this.edit({ op: "delete_step", index });
    if (this.view !== before && this.view?.can_undo) this.offerUndo("Deleted the step", this.view.id);
  };

  /** Sets the idle time before step `index`. */
  setPause = (index: number, ms: number) => this.edit({ op: "set_pause", index, dur: Math.max(0, Math.round(ms)) });

  /** Shortens every pause longer than 1 s to 1 s. */
  trimPauses = async () => {
    const n = this.longPauses;
    if (!n) return;
    const before = this.view;
    await this.edit({ op: "cap_pauses", max: TRIM_PAUSE_MS });
    if (this.view && this.view !== before) {
      this.offerUndo(`Shortened ${plural(n, "pause")} to ${TRIM_PAUSE_MS / 1000} s`, this.view.id);
    }
  };

  insertWait = () => this.edit({ op: "insert_wait", at: Math.round(this.cur), dur: 500, label: "Inserted" });

  /** Inserts a check at the playhead for the pixel under the macro's cursor, in its current color. */
  insertPixelCheck = async () => {
    if (!this.view || this.mode !== "idle") return;
    if (!this.editable) return this.fail({ code: "unavailable", message: "Editing needs the Relay app" });
    const id = this.view.id;
    const at = Math.round(this.cur);
    const p = this.cursorAt(this.cur);
    const x = Math.round(p.x);
    const y = Math.round(p.y);
    const color = await this.backend.samplePixel(x, y).catch(() => null);
    if (this.view?.id !== id) return; // the user opened another macro meanwhile
    // Without the pixel's color there's nothing sensible to wait for.
    if (!color) return this.fail({ code: "unavailable", message: `Couldn't read the screen at ${x}, ${y}` });
    return this.edit({ op: "insert_pixel_wait", at, dur: 800, x, y, color, tolerance: 8, timeout_ms: 5000, label: "" });
  };

  /**
   * "Pick": counts down, reads the pixel under the real cursor, then calls
   * `use` if the same macro is still open (the user may have moved on).
   */
  private async pick(use: (p: PickedPixel) => Promise<unknown> | void) {
    const id = this.view?.id;
    if (!id || this.picking || this.mode !== "idle") return;
    this.picking = PICK_SECONDS;
    const countdown = setInterval(() => (this.picking = Math.max(1, this.picking - 1)), 1000);
    try {
      const p = await this.run(this.backend.pickPixel(PICK_SECONDS * 1000));
      if (p && this.view?.id === id) await use(p);
    } finally {
      clearInterval(countdown);
      this.picking = 0;
    }
  }

  /** Points the pixel check at step `index` at the pixel under the real cursor. */
  pickPixel = (index: number) => {
    const picked = this.steps[index];
    if (picked?.kind !== "pixel_wait") return;
    // Steps may be edited during the countdown: only the same check, still in
    // its row with its first event where it was, is updated.
    const item = picked.items[0];
    return this.pick((p) => {
      const step = this.steps[index];
      if (step?.kind !== "pixel_wait" || step.items[0] !== item) return;
      const { tolerance, timeout_ms } = step;
      return this.edit({ op: "update_pixel_wait", index, x: p.x, y: p.y, color: p.color, tolerance, timeout_ms });
    });
  };

  // — playback options —

  /** Changes the open macro's playback options and saves them. */
  setPlayback = (patch: Partial<PlaybackOptions>) => {
    if (!this.view) return;
    this.previewPlayback(patch);
    return this.apply(this.view.id, this.backend.setPlaybackOptions(this.view.id, this.view.playback));
  };

  /** Shows new playback options without saving them (while a slider moves). */
  previewPlayback = (patch: Partial<PlaybackOptions>) => {
    if (this.view) this.view = { ...this.view, playback: { ...this.view.playback, ...patch } };
  };

  // — triggers —

  /** Saves the open macro's triggers; a refused hotkey puts the saved triggers back. */
  setTriggers = async (patch: Partial<MacroTriggers>) => {
    const id = this.view?.id;
    const current = this.triggerStatus;
    if (!id || !current) return;
    const saves = this.triggerSaves;
    const seq = ++saves.seq;
    saves.failed = false;
    const next = { ...current.triggers, ...patch };
    this.triggerStatus = { ...current, triggers: next };
    const status = await this.run(this.backend.setTriggers(id, next));
    if (this.view?.id !== id || seq <= saves.savedSeq) return; // for a macro left since, or older than what's shown
    if (status) {
      saves.saved = status;
      saves.savedSeq = seq;
    }
    const latest = seq === saves.seq;
    if (latest) saves.failed = !status;
    if (latest || saves.failed) this.triggerStatus = latest && status ? status : saves.saved;
    if (status) await this.refreshLibrary(); // the hotkey column
  };

  setTriggersPaused = async (paused: boolean) => {
    const seq = ++this.pauseSaves.seq;
    this.triggersPaused = paused;
    try {
      await this.backend.setTriggersPaused(paused); // confirmed by a triggers_paused message
    } catch (e) {
      if (seq === this.pauseSaves.seq) this.triggersPaused = this.pauseSaves.saved;
      this.fail(e);
    }
  };

  /** "Pick" for the pixel trigger: watch the pixel under the cursor. */
  pickTriggerPixel = () =>
    this.pick((p) => {
      const t = this.triggers;
      if (t) return this.setTriggers({ pixel: { ...t.pixel, x: p.x, y: p.y, color: p.color } });
    });

  loadProcesses = async () => {
    this.processes = (await this.backend.listProcesses().catch(() => null)) ?? this.processes;
  };

  // — settings —

  updateSettings = async (patch: Partial<Settings>) => {
    const saves = this.settingsSaves;
    const seq = ++saves.seq;
    saves.failed = false;
    this.settings = { ...this.settings, ...patch };
    const saved = await this.run(this.backend.updateSettings(this.settings));
    if (saved && seq > saves.savedSeq) {
      saves.saved = saved;
      saves.savedSeq = seq;
    }
    const latest = seq === saves.seq;
    if (latest) saves.failed = !saved;
    if (latest || saves.failed) this.settings = latest && saved ? saved : saves.saved;
  };

  setAutostart = async (enabled: boolean) => {
    this.autostart = (await this.run(this.backend.setAutostart(enabled))) ?? this.autostart;
  };

  // — export —

  doExport = async () => {
    if (!this.view) return;
    const path = await this.run(this.backend.exportMacro(this.view.id, this.exportFmt, this.exportName));
    if (!path) return; // cancelled or failed: keep the dialog open
    this.exportOpen = false;
    this.notify(`Saved ${path.split(/[\\/]/).pop()}`);
  };
}

export let relay = new RelayStore();

// A handle for debugging and end-to-end tests (`window.__relay` in DevTools).
const expose = () => ((window as unknown as { __relay: RelayStore }).__relay = relay);
expose();

/** Replaces the store with a fresh one (for tests; components read `relay` live). */
export function resetRelay(backend: Backend = defaultBackend): RelayStore {
  relay.dispose();
  relay = new RelayStore(backend);
  expose();
  return relay;
}
