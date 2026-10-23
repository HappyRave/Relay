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
  ImageSource,
  MacroListItem,
  MacroTriggers,
  MacroView,
  Mode,
  MovePoint,
  PlaybackOptions,
  Rect,
  Settings,
  Step,
  StepOf,
  Place,
  Tab,
  TriggerStatus,
} from "../types";
import type { SystemLook } from "../ipc/bindings/SystemLook";
import type { EngineMsg } from "../ipc/bindings/EngineMsg";
import type { PickedPixel } from "../ipc/bindings/PickedPixel";
import type { FoundImage } from "../ipc/bindings/FoundImage";
import type { TimingStats } from "../ipc/bindings/TimingStats";
import type { FinishReason } from "../ipc/bindings/FinishReason";
import type { RunEntry } from "../ipc/bindings/RunEntry";
import type { RunSource } from "../ipc/bindings/RunSource";
import type { RelayKey } from "../ipc/bindings/RelayKey";
import { comboOf } from "../hotkeys";
import type { DataFileInfo } from "../ipc/bindings/DataFileInfo";
import { backend as defaultBackend, type Backend, type IpcError } from "../ipc/backend";
import { DEFAULT_LOOK, DEFAULT_PLAYBACK, DEFAULT_SETTINGS } from "../defaults";
import { isTauri, savePanes, savedWindow, setPlayerMode, showWindow } from "../platform/window";
import { NO_PANES } from "../layout";
import { nextRepeat, nextSpeed } from "../transport";
import type { Panes } from "../ipc/bindings/Panes";
import { lastIndexAtOrBefore } from "../preview/geometry";
import { currentStepIndex, jumpTarget } from "../timeline/lanes";
import { deleteRows, type Row } from "../editor/rows";
import { failed } from "../runs";
import { plural, slug } from "../format";
import { followStep } from "./selection";
import { pngSize, testResult } from "../image";

const RENAME_DEBOUNCE_MS = 250;
/** "Trim pauses" shortens every pause longer than this to this. */
export const TRIM_PAUSE_MS = 1000;
/** How far past the last tick the playhead may run before the next one arrives. */
const MAX_EXTRAPOLATION_MS = 100;
/** Seconds "Pick" waits before reading the pixel under the cursor. */
const PICK_SECONDS = 3;
/** What an image is for, besides a step: a step being inserted, or the image trigger. */
export const INSERTING = -1;
export const TRIGGER = -2;
export const MAX_REPEATS = 99;
const EMPTY_DESKTOP: Rect = { x: 0, y: 0, w: 1920, h: 1080 };
/** Inputs with their own Ctrl + Z. */
const TEXT_INPUTS = ["text", "search", "number"];

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
  /** A change that couldn't be written: the top bar says "Couldn't save" while it shows. */
  unsaved?: boolean;
  /** For an Undo of a delete: the macro it brings back. */
  restores?: string;
}

/** A toast being shown, with what tells it from the others. */
export type ShownToast = Toast & { id: number };

/** Toasts shown at once; older ones come back as these are dismissed. */
export const MAX_TOASTS = 3;

/** Step kinds that use the mouse: a macro with one opens with the view open. */
const POINTER: Step["kind"][] = ["click", "drag", "scroll", "move", "find_image"];
/** Step kinds that take a label. */
const LABELLED: Step["kind"][] = ["click", "drag", "wait", "pixel_wait", "find_image"];
/** One edit, or several as one. */
const batch = (ops: EditOp[]): EditOp => (ops.length === 1 ? ops[0] : { op: "batch", ops });

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
  /** The macro playing, and what started it; null when nothing plays. */
  playingId = $state<string | null>(null);
  runSource = $state<RunSource | null>(null);
  /** How and with what timing the last playback ended (for diagnostics and the end-to-end tests). */
  lastFinish: FinishReason | null = null;
  /** Counts the open macro's playbacks that ended, so the player can flash or shake for each. */
  finishes = $state(0);
  lastTiming: TimingStats | null = null;

  // — data (from commands) —
  settings = $state.raw<Settings>(DEFAULT_SETTINGS);
  /** What Windows offers for the look (frosted glass by default, transparency effects). */
  systemLook = $state.raw<SystemLook>(DEFAULT_LOOK);
  library = $state.raw<MacroListItem[]>([]);
  view = $state.raw<MacroView | null>(null);
  /** The open macro's screenshot from when it was recorded (an object URL), or null if it has none. */
  screenUrl = $state<string | null>(null);
  /** The open macro's triggers, with the next scheduled run and hotkey problems. */
  triggerStatus = $state.raw<TriggerStatus | null>(null);
  /** The open macro's triggers couldn't be loaded (the Triggers tab offers to retry). */
  triggersFailed = $state(false);
  /** The open macro's data file (its columns and rows, or why it can't be used); null without one. */
  dataFile = $state.raw<DataFileInfo | null>(null);
  private dataSeq = 0;
  /** Paused by the kill switch (or from the tray). */
  triggersPaused = $state(false);
  autostart = $state(false);
  /** Running programs, for the "When app launches" suggestions. */
  processes = $state.raw<string[]>([]);
  /** The run history, newest first (loaded while the Library tab shows it). */
  runs = $state.raw<RunEntry[]>([]);

  // — UI —
  /** Which of Relay's windows this store is in: the main window, or the player. */
  role: "main" | "player" = "main";
  /** False until the saved window state is known. */
  ready = $state(false);
  /** The player stands in for the window. */
  playerMode = $state(false);
  /** Where the user put the editor's dividers (null: the default); the layout keeps them inside the window. */
  panes = $state.raw<Panes>(NO_PANES);
  place = $state<Place>("library");
  tab = $state<Tab>("steps");
  /** What the Library shows: its macros, or Activity (every run, or only the problems). */
  libraryShows = $state<"macros" | "runs" | "problems">("macros");
  /** Which macros it lists: all, those with triggers, or those run lately. */
  libFilter = $state<"all" | "triggers" | "recent">("all");
  /** The macros selected in the Library, to export or delete together. */
  libSelection = $state.raw<string[]>([]);
  /** The run history shows only this macro's runs (null: every macro's). */
  runsFilter = $state<string | null>(null);
  exportOpen = $state(false);
  exportFmt = $state<ExportFormat>("rly");
  /** Short messages, oldest first, optionally with an action (Undo). Errors stay until dismissed. */
  toasts = $state.raw<ShownToast[]>([]);
  /** The ones on screen: the newest few. */
  shownToasts = $derived(this.toasts.slice(-MAX_TOASTS));
  /** The newest toast. */
  toast = $derived<ShownToast | null>(this.toasts.at(-1) ?? null);
  /** Seconds left before "Pick" reads the cursor's pixel, or 0 when not picking. */
  picking = $state(0);
  /**
   * While an image is coming (a snip waits for the user): where from, and
   * what for: a Find image step's index, INSERTING one, or the TRIGGER.
   */
  imaging = $state.raw<{ source: ImageSource; index: number } | null>(null);
  /**
   * What "Test" last said, and about what (the macro, and a step's first
   * event or TRIGGER), with the best match and where it would be clicked.
   */
  imageTest = $state.raw<{ id: string; item: number; text: string; found?: FoundImage | null; dot?: [number, number] } | null>(
    null,
  );
  /** The step the inspector shows and the list's keys move from, or -1. It follows its step across edits (see selection.ts). */
  selected = $state(-1);
  /** The steps selected in the list, ascending (`selected` among them): the inspector edits them together. */
  selection = $state.raw<number[]>([]);
  /** The inspector's tab: the selected steps, or the macro as a whole. */
  inspector = $state<"step" | "macro">("step");
  /** The Macro tab's open section. */
  macroSection = $state<"play" | "triggers">("play");
  /** In a narrow window, the inspector slides over the list: it's open. */
  inspectorOpen = $state(false);
  /** The view (preview and timeline) is open. It opens closed for a macro without mouse steps. */
  viewOpen = $state(true);
  /** The step the last run stopped on (a pixel check that timed out), or -1. */
  failedStep = $state(-1);
  /** How many changes to the macro were saved, for the top bar's "Saved". */
  saves = $state(0);

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
  private runsSeq = 0;
  /** A name being typed, not saved yet (no timer when it's blank: it won't be). */
  private rename_: { id: string; name: string; timer: ReturnType<typeof setTimeout> | undefined } | null = null;
  /** The open macro's name as Rust has it, for when a blank one is abandoned. */
  private savedName = "";
  private toastTimers = new Map<number, ReturnType<typeof setTimeout>>();
  private toastSeq = 0;
  // Settings, triggers and their pause are saved optimistically: a change
  // shows at once, only the newest request's answer is shown, and when that
  // fails the last state Rust confirmed comes back (an older answer, or a
  // failure, never overwrites a newer change).
  private settingsSaves = { seq: 0, savedSeq: 0, saved: DEFAULT_SETTINGS, failed: false };
  private triggerSaves = { seq: 0, savedSeq: 0, saved: null as TriggerStatus | null, failed: false };
  private pauseSaves = { seq: 0, saved: false };
  /** A just-saved recording to open once the session is back to idle. */
  private pendingLoad: string | null = null;
  /** A new recording to name: its name field takes the focus once it shows. */
  naming = $state<string | null>(null);

  // — derived —
  recording = $derived(this.mode === "recording" || this.mode === "countdown");
  playing = $derived(this.mode === "playing" || this.mode === "paused");
  /** The open macro is the one playing. */
  playingHere = $derived(this.playing && (this.playingId == null || this.playingId === this.view?.id));
  /** Another macro plays (a trigger started it): the open one's Play is off, and it isn't drawn as playing. */
  playingElsewhere = $derived(this.playing && !this.playingHere);
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
  /** A change couldn't be written to disk, and its message is still up. */
  unsaved = $derived(this.toasts.some((t) => t.unsaved));
  /** The newest error shown. */
  error = $derived([...this.toasts].reverse().find((t) => t.kind === "error")?.message ?? null);
  shownRuns = $derived(
    this.runs.filter((r) => (!this.runsFilter || r.macro_id === this.runsFilter) && (this.libraryShows !== "problems" || failed(r))),
  );
  /** Runs that didn't end as planned: the Library's Problems. */
  problems = $derived(this.runs.filter(failed).length);
  /** Each macro's latest run, from the run history (newest first). */
  lastRuns = $derived(new Map([...this.runs].reverse().map((r) => [r.macro_id, r])));
  /** Never "relay.exe": a program named like Relay would pass for it. */
  exportName = $derived.by(() => {
    const base = slug(this.name) || "macro";
    return (this.exportFmt === "exe" && base === "relay" ? "relay-macro" : base) + "." + this.exportFmt;
  });

  // — lifecycle —

  async init() {
    const saved = await savedWindow().catch(() => ({ player_mode: false, panes: NO_PANES }));
    this.playerMode = saved.player_mode;
    this.panes = saved.panes;
    this.ready = true;
    await this.run(this.backend.subscribe(this.onEngine));
    this.settings = (await this.run(this.backend.getSettings())) ?? this.settings;
    this.settingsSaves.saved = this.settings;
    this.systemLook = (await this.run(this.backend.systemLook())) ?? this.systemLook;
    await this.refreshLibrary();
    if (this.role === "player") {
      // The player shows the macro the window has open (or the first, before it opens one).
      const id = (await this.run(this.backend.currentMacro())) ?? this.library[0]?.id;
      if (id) await this.loadMacro(id);
      return;
    }
    await this.refreshRuns();
    if (this.library[0]) await this.loadMacro(this.library[0].id);
    // The Library is home, unless there's only one macro to go to.
    this.place = this.library.length === 1 ? "macro" : "library";
    this.autostart = await this.backend.getAutostart().catch(() => false);
  }

  /** A divider moved (shown at once; saved by `savePanes` when the drag ends). */
  movePanes = (change: Partial<Panes>) => {
    this.panes = { ...this.panes, ...change };
  };

  savePanes = () => savePanes(this.panes).catch(() => {});

  /** Double-click on a divider: that one back to the default. */
  resetPane = (key: keyof Panes) => {
    this.movePanes({ [key]: null });
    return this.savePanes();
  };

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
    this.toastTimers.forEach(clearTimeout);
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
        const live = mode === "playing" || mode === "paused";
        this.playingId = live ? msg.macro_id : null;
        this.runSource = live ? msg.source : null;
        // The player shows the macro that plays; the window keeps its own, and
        // drops the run's first tick (it comes before this message).
        if (live && msg.macro_id && msg.macro_id !== this.view?.id) {
          if (this.role === "player") this.showMacro(msg.macro_id);
          else this.tick.advancing = false;
        }
        if (mode === "recording") {
          this.recMoves = [];
          this.recSteps = [];
          this.recVersion++;
          this.cur = 0;
          this.tick = { t: 0, at: now, speed: 1, advancing: true };
        }
        if (mode === "countdown") this.cur = 0;
        if (mode === "playing" && (!msg.macro_id || msg.macro_id === this.view?.id)) this.failedStep = -1;
        if (mode === "idle") {
          this.tick.advancing = false;
          this.playInfo = null;
          this.recMoves = [];
          this.recSteps = [];
        }
        this.mode = mode;
        if (mode === "idle" && this.pendingLoad) {
          this.openRecording(this.pendingLoad);
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
        if (this.playingElsewhere) break;
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
        this.lastFinish = msg.reason;
        if (msg.timing) this.lastTiming = msg.timing;
        if (this.playingElsewhere) break;
        this.finishes++;
        this.loopIdx = 0;
        // Any stop rewinds (Stop, Esc, a key press, the kill switch). A completed run stays at
        // the end, and a timed-out pixel check stays on its step so the row is highlighted.
        if (msg.reason !== "completed" && msg.reason !== "pixel_timeout") this.cur = 0;
        if (msg.reason === "pixel_timeout") this.failedStep = this.curStepIdx;
        break;
      case "saved":
        // Arrives just before the session returns to idle.
        if (this.mode === "idle") this.openRecording(msg.id);
        else this.pendingLoad = msg.id;
        break;
      case "library_changed":
        this.refreshLibrary();
        break;
      case "runs_changed":
        this.refreshRuns();
        break;
      case "system_look":
        this.systemLook = msg.look;
        break;
      case "triggers_paused":
        this.triggersPaused = this.pauseSaves.saved = msg.paused;
        break;
      case "current":
        // Opened in the other window, or by Relay: this one follows, so Play plays what it shows.
        if (msg.by !== this.role && this.mode === "idle" && msg.macro_id && msg.macro_id !== this.view?.id) {
          this.showMacro(msg.macro_id);
        }
        break;
      case "open":
        if (this.role === "main") this.openMacro(msg.macro_id);
        break;
      case "player_mode":
        this.playerMode = msg.on;
        break;
      case "error":
        this.fail({ code: msg.type, message: msg.message });
        break;
      case "unsaved":
        console.error("Relay:", msg.message);
        this.show({ kind: "error", message: msg.message, unsaved: true });
        break;
      case "notice":
        this.notify(msg.message, undefined, 6000);
        break;
    }
  };

  // In the app Relay's keys are global hotkeys handled in Rust; the browser
  // preview has none, so it listens here.
  private onKey = (e: KeyboardEvent) => {
    // A hotkey being set, or the export dialog (which handles Esc itself), owns the keyboard.
    if (this.exportOpen || (e.target as HTMLElement | null)?.closest?.("[data-captures-keys]")) return;
    const key = e.key.toLowerCase();
    if (e.key === "Escape") {
      if (!isTauri() && this.mode !== "idle") this.stop();
      return;
    }
    // Undo and redo, except in fields you type in, which have their own
    // (a focused slider or switch doesn't).
    const t = e.target;
    const typing = t instanceof HTMLTextAreaElement || (t instanceof HTMLInputElement && TEXT_INPUTS.includes(t.type));
    if ((e.ctrlKey || e.metaKey) && !e.altKey && !typing && (key === "z" || key === "y")) {
      e.preventDefault();
      if (key === "y" || e.shiftKey) this.redo();
      else this.undo();
      return;
    }
    if (e.ctrlKey && !e.altKey && !e.shiftKey && key === "m" && this.place === "macro") {
      e.preventDefault();
      this.inspector = "macro";
      this.inspectorOpen = true;
      return;
    }
    if (e.altKey && !e.ctrlKey && !e.shiftKey && e.key === "ArrowLeft" && !typing) {
      e.preventDefault();
      this.back();
      return;
    }
    if (isTauri()) return;
    const combo = comboOf(e);
    const keys = this.settings.keys;
    if (!combo) return;
    if (combo === keys.record) {
      e.preventDefault();
      this.toggleRec();
    } else if (combo === keys.play) {
      e.preventDefault();
      this.togglePlay();
    } else if (combo === keys.player) {
      e.preventDefault();
      this.setPlayerMode(!this.playerMode);
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
    this.dismissWhere((t) => !!t.undoes);
  }

  /** Dismisses one toast. */
  dismissToast = (id: number) => this.dismissWhere((t) => t.id === id);

  /** Dismisses every toast. */
  clearToasts = () => this.dismissWhere(() => true);

  private dismissWhere(gone: (t: ShownToast) => boolean) {
    const [out, kept] = [this.toasts.filter(gone), this.toasts.filter((t) => !gone(t))];
    if (!out.length) return;
    for (const t of out) {
      clearTimeout(this.toastTimers.get(t.id));
      this.toastTimers.delete(t.id);
    }
    this.toasts = kept;
  }

  /**
   * Shows a toast under the others, for `ms` or, without it (errors), until
   * it's dismissed. The same message again moves to the bottom instead of
   * showing twice.
   */
  private show(toast: Toast, ms?: number) {
    this.dismissWhere((t) => t.kind === toast.kind && t.message === toast.message);
    const id = ++this.toastSeq;
    this.toasts = [...this.toasts, { ...toast, id }];
    if (ms != null) this.toastTimers.set(id, setTimeout(() => this.dismissToast(id), ms));
  }

  // — session —

  toggleRec = () => {
    if (this.playing) return;
    return this.run(this.backend.toggleRecord());
  };

  togglePlay = () => {
    if (this.recording || !this.view) return;
    const from = this.cur >= this.duration - 1 ? 0 : this.cur;
    // Starting over from the end: show the playhead at the start now, not at the first tick.
    if (this.mode === "idle") this.cur = from;
    return this.run(this.backend.togglePlay(from));
  };

  stop = () => this.run(this.backend.stop());

  /** The player takes the window's place, or gives it back (in the browser preview, on its demo desktop). */
  setPlayerMode = async (on: boolean) => {
    if (!isTauri()) this.playerMode = on;
    await this.run(setPlayerMode(on));
  };

  /** The player's expand key: the window, on the player's macro. */
  openWindow = async () => {
    if (isTauri()) return void (await this.run(showWindow()));
    this.playerMode = false;
    if (this.view) this.place = "macro";
  };

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

  /** Selects these steps, `cursor` the one the inspector shows: the playhead goes to it. */
  select = (indices: number[], cursor = indices.at(-1) ?? -1) => {
    const s = this.steps[cursor];
    this.selection = [...new Set(indices)].filter((i) => this.steps[i]).sort((a, b) => a - b);
    this.selected = s ? cursor : -1;
    this.inspector = "step";
    if (s) this.seek(s.t);
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

  /** The Library's macros, or its Activity: every run, or only the problems. */
  showLibrary = async (shows: "macros" | "runs" | "problems") => {
    this.libraryShows = shows;
    if (shows !== "macros") await this.refreshRuns();
  };

  /** A macro's runs, from its More menu: Activity, only them. */
  showRunsOf = (id: string) => {
    this.runsFilter = id;
    this.go("library");
    return this.showLibrary("runs");
  };

  /** Plays a macro from its Library row, from the start, without leaving the Library. */
  playMacro = async (id: string) => {
    if (this.mode !== "idle") return;
    if (this.view?.id !== id) await this.loadMacro(id);
    if (this.view?.id !== id) return;
    this.cur = 0;
    return this.togglePlay();
  };

  /** A Library row's "Also runs": the macro, on its triggers. */
  openTriggers = async (id: string) => {
    await this.openMacro(id);
    if (this.view?.id !== id) return;
    this.inspector = "macro";
    this.macroSection = "triggers";
    this.inspectorOpen = true;
  };

  /** A run that stopped on a step: the macro, on that step (1-based), marked when it timed out. */
  openAt = async (id: string, step: number, timedOut = false) => {
    await this.openMacro(id);
    if (this.view?.id !== id || !this.steps[step - 1]) return;
    this.select([step - 1]);
    this.inspectorOpen = true;
    if (timedOut) this.failedStep = step - 1;
  };

  /** Moves these macros to the trash, with one Undo. */
  deleteMacros = async (ids: string[]) => {
    if (ids.length === 1) return this.deleteMacro(ids[0]);
    await this.flushRename();
    const open = this.view?.id;
    const gone: string[] = [];
    for (const id of ids) {
      try {
        await this.backend.deleteMacro(id);
        gone.push(id);
      } catch (e) {
        this.fail(e);
        break;
      }
    }
    this.libSelection = this.libSelection.filter((id) => !gone.includes(id));
    await this.refreshLibrary();
    if (open && gone.includes(open)) {
      if (this.library[0]) await this.loadMacro(this.library[0].id);
      else this.forgetMacro();
    }
    if (!gone.length) return;
    const run = async () => {
      this.dismissWhere((t) => t.restores === gone[0]);
      for (const id of [...gone].reverse()) await this.run(this.backend.restoreMacro(id));
      await this.refreshLibrary();
    };
    this.show({ kind: "info", message: `Moved ${plural(gone.length, "macro")} to the trash`, action: { label: "Undo", run }, restores: gone[0] }, 8000);
  };

  /** Writes these macros to a folder the user picks, as .rly files. */
  exportMacros = async (ids: string[]) => {
    const macros = this.library.filter((m) => ids.includes(m.id)).map(({ id, name }) => ({ id, name }));
    const dir = await this.run(this.backend.exportMacros(macros));
    if (dir) this.notify(`Exported ${plural(macros.length, "macro")} to ${dir.split(/[\\/]/).pop()}`);
  };

  async refreshRuns() {
    const seq = ++this.runsSeq;
    const runs = await this.run(this.backend.listRuns());
    if (runs && seq === this.runsSeq) this.runs = runs;
  }

  loadMacro = async (id: string) => {
    if (this.recording || this.playing) return;
    await this.showMacro(id);
    this.tab = "steps";
  };

  /** A recording was saved: it opens, ready to be named, and says so. */
  private async openRecording(id: string) {
    await this.openMacro(id);
    if (this.view?.id !== id || this.role !== "main") return;
    const n = this.view.steps.filter((s) => s.kind !== "move").length;
    this.naming = id;
    this.notify(`Recording stopped, ${n} ${n === 1 ? "step" : "steps"}. Name this macro.`);
  }

  /** Opens a macro in the editor, from the Library or after a recording. */
  openMacro = async (id: string) => {
    if (this.recording || this.playing) return;
    if (this.view?.id !== id) await this.loadMacro(id);
    if (this.view?.id === id) this.place = "macro";
  };

  go = (place: Place) => {
    this.place = place;
  };

  /** Alt + ←: the Library, from a macro or Settings. */
  back = () => {
    if (this.place !== "library") this.go("library");
  };

  /** Opens a macro in the editor (also mid-playback, when a trigger started it). */
  private async showMacro(id: string) {
    await this.flushRename();
    const seq = ++this.viewSeq;
    const view = await this.run(this.backend.loadMacro(id));
    if (!view || seq !== this.viewSeq) return;
    // An Undo offered for the macro left behind mustn't stay up over this one.
    this.dismissWhere((t) => !!t.undoes && t.undoes !== id);
    this.view = view;
    this.savedName = view.name;
    this.selected = -1;
    this.selection = [];
    this.failedStep = -1;
    this.inspector = "step";
    this.inspectorOpen = false;
    this.viewOpen = view.steps.some((s) => POINTER.includes(s.kind));
    this.triggerStatus = null; // the old macro's triggers mustn't be edited into this one
    this.dataFile = null;
    this.cur = 0;
    this.loopIdx = 0;
    await Promise.all([this.loadScreen(id), this.loadTriggers(), this.loadDataFile()]);
  }

  /** Fetches the open macro's screenshot. Without one (or if reading it fails), the preview shows the sketch. */
  private async loadScreen(id: string) {
    this.setScreen(null);
    const bytes = await this.backend.screenshot(id).catch(() => null);
    if (!bytes?.byteLength || this.view?.id !== id) return;
    this.setScreen(URL.createObjectURL(new Blob([bytes], { type: "image/jpeg" })));
  }

  private setScreen(url: string | null) {
    if (this.screenUrl) URL.revokeObjectURL(this.screenUrl);
    this.screenUrl = url;
  }

  /** Loads the open macro's triggers (again, with Retry, after a failure). */
  loadTriggers = async () => {
    const id = this.view?.id;
    if (!id) return;
    const saves = this.triggerSaves;
    const loading = ++saves.seq; // answers to changes made to the macro left behind are dropped
    saves.failed = false;
    this.triggersFailed = false;
    const status = await this.run(this.backend.getTriggers(id));
    if (this.view?.id !== id || loading !== saves.seq) return;
    if (!status) return void (this.triggersFailed = true);
    this.triggerStatus = saves.saved = status;
    saves.savedSeq = loading;
    this.triggersPaused = this.pauseSaves.saved = status.paused;
  };

  duplicateMacro = async (id: string) => {
    await this.flushRename(); // the copy is named after the name being typed
    const copy = await this.run(this.backend.duplicateMacro(id));
    if (!copy) return;
    await this.refreshLibrary();
    await this.loadMacro(copy);
    this.place = "library";
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
      else this.forgetMacro();
      this.place = "library";
    }
    const action = { label: "Undo", run: () => this.restoreMacro(id, wasOpen) };
    this.show({ kind: "info", message: `Moved “${name}” to the trash`, action, restores: id }, 8000);
  };

  /** No macro open (the last one went to the trash). */
  private forgetMacro() {
    this.view = null;
    this.setScreen(null);
    this.triggerStatus = null;
    this.triggersFailed = false;
  }

  /** Brings a macro back from the trash; `reopen` it when it was the open one. */
  restoreMacro = async (id: string, reopen = false) => {
    this.dismissWhere((t) => t.restores === id);
    try {
      await this.backend.restoreMacro(id);
    } catch (e) {
      return this.fail(e);
    }
    await this.refreshLibrary();
    if (!reopen) return; // stay on the macro and tab the user is on
    await this.loadMacro(id);
    this.place = "library";
  };

  importMacros = async () => {
    const result = await this.run(this.backend.importMacros());
    if (!result) return;
    await this.refreshLibrary();
    if (result.imported[0]) await this.loadMacro(result.imported[0]);
    this.place = "library";
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
    const kept = this.selection.map((i) => followStep(this.view!.steps, view.steps, i, op)).filter((i) => i >= 0);
    this.selection = [...new Set(this.selected >= 0 ? [...kept, this.selected] : kept)].sort((a, b) => a - b);
    this.failedStep = -1;
    this.saves++;
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

  /** Deletes these rows of the steps list, their folded moves too, in one undo. */
  deleteRows = async (rows: Row[]) => {
    if (!rows.length) return;
    const before = this.view;
    await this.edit(deleteRows(rows));
    const what = rows.length === 1 ? "the step" : plural(rows.length, "step");
    if (this.view !== before && this.view?.can_undo) this.offerUndo(`Deleted ${what}`, this.view.id);
  };

  /** Sets the pause before each of these steps, in one undo. */
  setPauses = (indices: number[], ms: number) => {
    const dur = Math.max(0, Math.round(ms));
    return this.edit(batch(indices.map((index) => ({ op: "set_pause", index, dur }))));
  };

  /** Labels each of these steps that takes a label, in one undo. */
  setLabels = (indices: number[], label: string) =>
    this.edit(batch(indices.filter((i) => LABELLED.includes(this.steps[i]?.kind)).map((index) => ({ op: "set_label", index, label }))));

  /** Makes each of these typed texts an editable Text step, in one undo. */
  makeAllEditable = (indices: number[]) =>
    this.edit(batch(indices.filter((i) => this.steps[i]?.kind === "type").map((index) => ({ op: "make_editable", index }))));

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

  /** Sets how long MOVE step `index` takes, from its first sample to its last. */
  setMoveDuration = (index: number, ms: number) =>
    this.edit({ op: "set_move_duration", index, dur: Math.max(0, Math.round(ms)) });

  /** Takes the wobble out of MOVE step `index`'s path. */
  smoothMove = (index: number) => this.reshapeMove({ op: "smooth_move", index }, "Smoothed the move");

  /** Makes MOVE step `index`'s path a straight line. */
  straightenMove = (index: number) => this.reshapeMove({ op: "straighten_move", index }, "Straightened the move");

  /** A reshape changes the path in the preview only, so it offers an Undo; one that changed nothing doesn't. */
  private async reshapeMove(op: EditOp, done: string) {
    const before = this.view;
    await this.edit(op);
    const after = this.view;
    if (after && before && after !== before && JSON.stringify(after.moves) !== JSON.stringify(before.moves)) {
      this.offerUndo(done, after.id);
    }
  }

  insertWait = () => this.edit({ op: "insert_wait", at: Math.round(this.cur), dur: 500, label: "Inserted" });

  /** Inserts an empty Text step at the playhead and opens its editor. */
  insertText = async () => {
    const at = Math.round(this.cur);
    const before = this.view;
    await this.edit({ op: "insert_text", at, text: "" });
    if (!this.view || this.view === before) return;
    // Inserted just after the step under the playhead: the first empty one from there.
    const i = this.view.steps.findIndex((s) => s.kind === "text" && s.text === "" && s.t >= at);
    if (i >= 0) {
      this.selected = i;
      this.selection = [i];
      this.inspector = "step";
    }
  };

  /** Changes what Text step `index` types. */
  updateText = (index: number, text: string) => this.edit({ op: "update_text", index, text });

  /** Turns TYPE step `index` into a Text step typing the same. */
  makeEditable = (index: number) => this.edit({ op: "make_editable", index });

  /**
   * What a Text step's `text` would type now, or why it can't be typed; `saves`:
   * the text itself is fine (only the data file lacks a column it types).
   */
  previewText = async (text: string): Promise<{ text: string } | { error: string; saves: boolean }> => {
    if (!this.view) return { error: "Open a macro first", saves: false };
    try {
      return { text: await this.backend.previewText(this.view.id, text) };
    } catch (e) {
      return { error: (e as IpcError)?.message ?? String(e), saves: (e as IpcError)?.code === "data_file" };
    }
  };

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

  /** An image from `source` for `index` (see `imaging`), or null (cancelled, or failed and said so). */
  private async takeImage(source: ImageSource, index: number): Promise<string | null> {
    // A snip still waiting would land later, on top of this one.
    if (this.imaging?.source === "snip" && source !== "snip") void this.backend.cancelSnip();
    const mine = { source, index };
    this.imaging = mine;
    try {
      const b = this.backend;
      const p = source === "snip" ? b.snipImage() : source === "paste" ? b.pasteImage() : b.chooseImage();
      const image = (await this.run(p)) ?? null;
      // Another request took over meanwhile (Paste instead of the snip, or Cancel).
      return this.imaging === mine ? image : null;
    } finally {
      if (this.imaging === mine) this.imaging = null;
    }
  }

  /** Inserts a Find image step at the playhead, with an image from `source`, clicked in its middle. */
  insertFindImage = async (source: ImageSource = "snip") => {
    if (!this.view || this.mode !== "idle") return;
    if (!this.editable) return this.fail({ code: "unavailable", message: "Editing needs the Relay app" });
    const id = this.view.id;
    const at = Math.round(this.cur);
    const image = await this.takeImage(source, INSERTING);
    if (!image || this.view?.id !== id || this.mode !== "idle") return;
    const [w, h] = pngSize(image);
    const click_x = Math.floor(w / 2);
    const click_y = Math.floor(h / 2);
    const step = { image, click_x, click_y, btn: "Left" as const, threshold: 85, timeout_ms: 5000, area: null };
    return this.edit({ op: "insert_find_image", at, dur: 800, ...step, label: "" });
  };

  /** Stops waiting for a snip. */
  cancelImage = () => {
    if (this.imaging?.source === "snip") void this.backend.cancelSnip();
    this.imaging = null;
  };

  /** Changes Find image step `index`'s settings (the others stay). */
  updateFindImage = (index: number, patch: Partial<Omit<StepOf<"find_image">, "kind">>) => {
    const s = this.steps[index];
    if (s?.kind !== "find_image") return;
    const { image, click_x, click_y, btn, threshold, timeout_ms, area } = { ...s, ...patch };
    this.imageTest = null; // it was about the old settings
    return this.edit({ op: "update_find_image", index, image, click_x, click_y, btn, threshold, timeout_ms, area });
  };

  /** Gives Find image step `index` a new image from `source`, clicked in its middle. */
  replaceImage = async (index: number, source: ImageSource) => {
    const picked = this.steps[index];
    if (picked?.kind !== "find_image" || !this.canEdit) return;
    const item = picked.items[0];
    const image = await this.takeImage(source, index);
    // As with Pick: only the same step, still in its row.
    const step = this.steps[index];
    if (!image || step?.kind !== "find_image" || step.items[0] !== item) return;
    const [w, h] = pngSize(image);
    return this.updateFindImage(index, { image, click_x: Math.floor(w / 2), click_y: Math.floor(h / 2) });
  };

  /** Looks for Find image step `index`'s image on screen once, and says what it found. */
  testFindImage = (index: number) => {
    const s = this.steps[index];
    if (s?.kind !== "find_image") return;
    // Where the step would click: its point, scaled like the match.
    const [w, h] = pngSize(s.image);
    const click = (m: FoundImage): [number, number] => [
      m.x + Math.round((s.click_x * m.w) / (w || m.w)),
      m.y + Math.round((s.click_y * m.h) / (h || m.h)),
    ];
    return this.testImage(s.items[0], s.image, s.threshold, s.area, click);
  };

  /** Looks for the image trigger's image on screen once, and says what it found. */
  testTriggerImage = () => {
    const t = this.triggers?.image;
    // The trigger doesn't click: the dot marks the middle.
    const middle = (m: FoundImage): [number, number] => [m.x + Math.floor(m.w / 2), m.y + Math.floor(m.h / 2)];
    if (t?.image) return this.testImage(TRIGGER, t.image, t.threshold, t.area, middle);
  };

  private async testImage(
    item: number,
    image: string,
    threshold: number,
    area: Rect | null,
    click: (m: FoundImage) => [number, number],
  ) {
    const id = this.view?.id;
    if (!id) return;
    const test = { id, item };
    this.imageTest = { ...test, text: "Looking…" };
    const m = await this.run(this.backend.testFindImage(image, threshold, area));
    if (this.imageTest?.id !== test.id || this.imageTest.item !== test.item) return;
    this.imageTest = m === undefined ? null : { ...test, text: testResult(m, threshold), found: m, dot: m ? click(m) : undefined };
  }

  /** Marks the last Test's best match on the screen, with a dot where it would be clicked. */
  showImageTest = () => {
    const t = this.imageTest;
    if (!t?.found || !t.dot) return;
    const { x, y, w, h } = t.found;
    return this.run(this.backend.showMatch({ x, y, w, h }, ...t.dot));
  };

  /** Watches for an image from `source`; the trigger turns on with its first image. */
  setTriggerImage = async (source: ImageSource) => {
    const id = this.view?.id;
    if (!id || !this.triggers) return;
    if (!this.editable) return this.fail({ code: "unavailable", message: "Triggers need the Relay app" });
    const image = await this.takeImage(source, TRIGGER);
    const t = this.triggers;
    if (!image || this.view?.id !== id || !t) return;
    return this.setTriggerImageOptions({ image, enabled: t.image.enabled || !t.image.image });
  };

  /** Changes the image trigger's settings (the others stay). */
  setTriggerImageOptions = (patch: Partial<MacroTriggers["image"]>) => {
    const t = this.triggers;
    if (!t) return;
    this.imageTest = null; // it was about the old settings
    return this.setTriggers({ image: { ...t.image, ...patch } });
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

  /** Per macro, the repeat count to go back to when leaving "forever" (remembered when entering it). */
  private lastCounts = $state.raw<Record<string, number>>({});
  /** The repeat count: the saved one, or while looping forever, the one to go back to. */
  repeatCount = $derived(
    this.playback.repeat === "forever" ? ((this.view && this.lastCounts[this.view.id]) ?? 1) : this.playback.repeat.count,
  );

  toggleForever = () => {
    const r = this.playback.repeat;
    if (!this.view) return;
    if (r === "forever") return this.setPlayback({ repeat: { count: this.repeatCount } });
    this.lastCounts = { ...this.lastCounts, [this.view.id]: r.count };
    return this.setPlayback({ repeat: "forever" });
  };

  /** One repeat fewer, down to 1; while looping forever, back to the remembered count. */
  fewerRepeats = () => {
    if (this.playback.repeat === "forever") return this.setPlayback({ repeat: { count: this.repeatCount } });
    if (this.repeatCount > 1) return this.setPlayback({ repeat: { count: this.repeatCount - 1 } });
  };

  /** One repeat more, up to 99 (not while looping forever). */
  moreRepeats = () => {
    if (this.playback.repeat !== "forever" && this.repeatCount < MAX_REPEATS) {
      return this.setPlayback({ repeat: { count: this.repeatCount + 1 } });
    }
  };

  /** The narrow bar's speed button: the next speed (0.5× → 1× → 2× → 4× → 0.5×). */
  cycleSpeed = () => this.setPlayback({ speed: nextSpeed(this.playback.speed) });

  /** The narrow bar's folded repeat: 1 → 2 → 3 → 5 → 10 → forever → 1. */
  cycleRepeat = () => {
    if (!this.view) return;
    const r = this.playback.repeat;
    if (r !== "forever") this.lastCounts = { ...this.lastCounts, [this.view.id]: r.count };
    return this.setPlayback({ repeat: nextRepeat(r) });
  };

  // — data file —

  /** Reads the open macro's data file (again: it may have changed on disk). */
  loadDataFile = async () => {
    const id = this.view?.id;
    if (!id) return;
    const seq = ++this.dataSeq;
    const info = await this.run(this.backend.getDataFile(id));
    if (info !== undefined && this.view?.id === id && seq === this.dataSeq) this.dataFile = info;
  };

  /** Asks for a CSV whose rows drive the open macro's repeats. */
  chooseDataFile = async () => {
    const id = this.view?.id;
    if (!id) return;
    const info = await this.run(this.backend.chooseDataFile(id));
    if (!info || this.view?.id !== id) return;
    ++this.dataSeq;
    this.dataFile = info;
  };

  removeDataFile = async () => {
    const id = this.view?.id;
    if (!id) return;
    try {
      await this.backend.removeDataFile(id);
    } catch (e) {
      return this.fail(e);
    }
    if (this.view?.id !== id) return;
    ++this.dataSeq;
    this.dataFile = null;
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

  /** Sets one of Relay's keys (null: off). Resolves to why it was refused, or null once it's saved. */
  setRelayKey = async (key: RelayKey, combo: string | null): Promise<string | null> => {
    try {
      const saved = await this.backend.setRelayKey(key, combo);
      const saves = this.settingsSaves;
      saves.savedSeq = ++saves.seq;
      saves.saved = this.settings = saved;
      return null;
    } catch (e) {
      return (e as IpcError)?.message ?? String(e);
    }
  };

  /** What to know before using `combo` as a global key, or null. */
  keyWarning = (combo: string): Promise<string | null> => this.backend.keyWarning(combo).catch(() => null);

  /** A key field listens, or stopped: the hotkeys let go of the keys meanwhile. */
  holdHotkeys = (on: boolean) => this.backend.holdHotkeys(on).catch(() => {});

  /** The first run is over (finished or skipped): it doesn't show again. */
  finishIntro = () => this.updateSettings({ intro: "done" });

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
