// The UI talks to Relay through this interface: the Tauri commands and the
// engine's message stream in the app, or a read-only simulation in a plain
// browser (`npm run dev`).
import { Channel, invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import type { EditOp, ExportFormat, MacroListItem, MacroView, PlaybackOptions } from "../types";
import type { EngineMsg } from "./bindings/EngineMsg";
import type { Mode } from "./bindings/Mode";
import type { Settings } from "./bindings/Settings";
import type { PickedPixel } from "./bindings/PickedPixel";
import type { FoundImage } from "./bindings/FoundImage";
import type { Rect } from "./bindings/Rect";
import type { ImportResult } from "./bindings/ImportResult";
import type { MacroTriggers } from "./bindings/MacroTriggers";
import type { TriggerStatus } from "./bindings/TriggerStatus";
import type { RunEntry } from "./bindings/RunEntry";
import type { FinishReason } from "./bindings/FinishReason";
import { DEFAULT_SETTINGS } from "../defaults";
import { isTauri } from "../platform/window";

export interface Backend {
  /** False in the browser preview, where macros can't be recorded or edited. */
  readonly editable: boolean;
  /** Receives session updates (mode, countdown, recording progress, playback ticks). */
  subscribe(onMessage: (msg: EngineMsg) => void): Promise<void>;
  toggleRecord(): Promise<void>;
  togglePlay(from: number): Promise<void>;
  stop(): Promise<void>;
  seek(t: number): Promise<void>;
  listMacros(): Promise<MacroListItem[]>;
  /** The run history, newest first. */
  listRuns(): Promise<RunEntry[]>;
  loadMacro(id: string): Promise<MacroView>;
  /** The screenshot taken when the macro was recorded (JPEG bytes), empty if it has none. */
  screenshot(id: string): Promise<ArrayBuffer>;
  editMacro(id: string, op: EditOp): Promise<MacroView>;
  /** Reverts the last edit, or with `redo` re-applies the last undone one. */
  undoEdit(id: string, redo: boolean): Promise<MacroView>;
  setPlaybackOptions(id: string, options: PlaybackOptions): Promise<MacroView>;
  /** Asks where to save, then writes the export. Returns the path, or null if cancelled. */
  exportMacro(id: string, format: ExportFormat, defaultName: string): Promise<string | null>;
  /** Asks for files, then imports them. Returns null if cancelled. */
  importMacros(): Promise<ImportResult | null>;
  /** Returns the copy's id. */
  duplicateMacro(id: string): Promise<string>;
  /** Moves a macro to the trash. */
  deleteMacro(id: string): Promise<void>;
  restoreMacro(id: string): Promise<void>;
  getSettings(): Promise<Settings>;
  updateSettings(settings: Settings): Promise<Settings>;
  /** The color of a screen pixel as "#RRGGBB", or null if unreadable. */
  samplePixel(x: number, y: number): Promise<string | null>;
  /** After `delayMs`, the position and color under the cursor. */
  pickPixel(delayMs: number): Promise<PickedPixel>;
  /** The picture on the clipboard, for a Find image step (a base64 PNG). */
  pasteImage(): Promise<string>;
  /** Asks for a PNG or JPEG file and reads it. Returns null if cancelled. */
  chooseImage(): Promise<string | null>;
  /** Opens Windows' snipping overlay and waits for the snip. Returns null if cancelled. */
  snipImage(): Promise<string | null>;
  /** Stops waiting for a snip. */
  cancelSnip(): Promise<void>;
  /** Looks for an image on screen once: the best match (its score in percent), or null. */
  testFindImage(image: string, threshold: number, area: Rect | null): Promise<FoundImage | null>;
  /** Marks a match on the screen for a few seconds: an outline around `area`, a dot where it's clicked. */
  showMatch(area: Rect, dotX: number, dotY: number): Promise<void>;
  getTriggers(id: string): Promise<TriggerStatus>;
  /** Rejects a hotkey that clashes with Relay's own or another macro's. */
  setTriggers(id: string, triggers: MacroTriggers): Promise<TriggerStatus>;
  setTriggersPaused(paused: boolean): Promise<void>;
  /** Running executables, for the "When app launches" suggestions. */
  listProcesses(): Promise<string[]>;
  getAutostart(): Promise<boolean>;
  setAutostart(enabled: boolean): Promise<boolean>;
}

/** Errors from Rust commands arrive as `{ code, message }`. */
export interface IpcError {
  code: string;
  message: string;
}

export const tauriBackend: Backend = {
  editable: true,
  subscribe: async (onMessage) => {
    const channel = new Channel<EngineMsg>();
    channel.onmessage = onMessage;
    await invoke("subscribe_engine", { channel });
  },
  toggleRecord: () => invoke("toggle_record"),
  togglePlay: (from) => invoke("toggle_play", { from }),
  stop: () => invoke("stop_session"),
  seek: (t) => invoke("seek", { t }),
  listMacros: () => invoke("list_macros"),
  listRuns: () => invoke("list_runs"),
  loadMacro: (id) => invoke("load_macro", { id }),
  screenshot: (id) => invoke("screenshot", { id }),
  editMacro: (id, op) => invoke("edit_macro", { id, op }),
  undoEdit: (id, redo) => invoke("undo_edit", { id, redo }),
  setPlaybackOptions: (id, options) => invoke("set_playback_options", { id, options }),
  exportMacro: async (id, format, defaultName) => {
    const path = await save({
      defaultPath: defaultName,
      filters: [{ name: format === "rly" ? "Relay macro" : "JSON events", extensions: [format] }],
    });
    if (!path) return null;
    await invoke("export_macro", { id, format, path });
    return path;
  },
  importMacros: async () => {
    const picked = await open({ multiple: true, filters: [{ name: "Relay macros", extensions: ["rly", "json"] }] });
    if (!picked) return null;
    return invoke("import_macros", { paths: Array.isArray(picked) ? picked : [picked] });
  },
  duplicateMacro: (id) => invoke("duplicate_macro", { id }),
  deleteMacro: (id) => invoke("delete_macro", { id }),
  restoreMacro: (id) => invoke("restore_macro", { id }),
  getSettings: () => invoke("get_settings"),
  updateSettings: (settings) => invoke("update_settings", { settings }),
  samplePixel: (x, y) => invoke("sample_pixel", { x, y }),
  pickPixel: (delayMs) => invoke("pick_pixel", { delayMs }),
  pasteImage: () => invoke("paste_image"),
  chooseImage: async () => {
    const picked = await open({ filters: [{ name: "Images", extensions: ["png", "jpg", "jpeg"] }] });
    if (!picked || Array.isArray(picked)) return null;
    return invoke("load_image", { path: picked });
  },
  snipImage: () => invoke("snip_image"),
  cancelSnip: () => invoke("cancel_snip"),
  testFindImage: (image, threshold, area) => invoke("test_find_image", { image, threshold, area }),
  showMatch: (area, dotX, dotY) => invoke("show_match", { area, dotX, dotY }),
  getTriggers: (id) => invoke("get_triggers", { id }),
  setTriggers: (id, triggers) => invoke("set_triggers", { id, triggers }),
  setTriggersPaused: (paused) => invoke("set_triggers_paused", { paused }),
  listProcesses: () => invoke("list_processes"),
  getAutostart: () => invoke("get_autostart"),
  setAutostart: (enabled) => invoke("set_autostart", { enabled }),
};

interface FixtureItem {
  view: MacroView;
  runs: number;
  last_run_ago_ms: number | null;
  hotkey: string | null;
}

interface FixtureRun {
  ago_ms: number;
  entry: RunEntry;
}

/**
 * The design's sample macros, generated by relay-core's tests, with a small
 * session simulation: the countdown and playback run, recording captures
 * nothing, and step edits need the Rust core.
 */
export function browserBackend(): Backend {
  // Loaded lazily so the fixture stays out of the app bundle.
  let items: FixtureItem[] = [];
  let runs: RunEntry[] = [];
  const loadedAt = Date.now();
  const ready = Promise.all([import("../dev/sample-views.json"), import("../dev/sample-runs.json")]).then(([v, r]) => {
    items = structuredClone(v.default as unknown as FixtureItem[]);
    runs = (r.default as unknown as FixtureRun[]).map((f) => ({
      ...f.entry,
      at: new Date(loadedAt - f.ago_ms).toISOString(),
    }));
  });
  let settings: Settings = DEFAULT_SETTINGS;
  let triggersPaused = false;
  const find = async (id: string) => {
    await ready;
    const item = items.find((i) => i.view.id === id);
    if (!item) throw { code: "not_found", message: `no macro with id ${id}` } satisfies IpcError;
    return item;
  };
  const browserTriggers = new Map<string, MacroTriggers>();
  const triggersFor = (id: string): MacroTriggers =>
    browserTriggers.get(id) ?? {
      hotkey: { enabled: false, combo: items.find((i) => i.view.id === id)?.hotkey ?? "" },
      schedule: { enabled: false, schedule: { days: [true, true, true, true, true, false, false], time: "09:00" } },
      app_launch: { enabled: false, exe: "", delay_ms: 2000 },
      pixel: { enabled: false, x: 0, y: 0, color: "#EC3013", tolerance: 8 },
      image: { enabled: false, image: null, threshold: 85, area: null },
    };
  // Like the app, the Library shows a hotkey only while it's on (the samples' are off).
  const hotkeyOf = (t: MacroTriggers) => (t.hotkey.enabled && t.hotkey.combo ? t.hotkey.combo : null);
  const unavailable = (): never => {
    throw { code: "unavailable", message: "This needs the Relay app (npm run tauri dev)" } satisfies IpcError;
  };

  // — session simulation —
  let emit: (m: EngineMsg) => void = () => {};
  let mode: EngineMsg & { type: "session" } = { type: "session", mode: "idle", macro_id: null };
  let timer: ReturnType<typeof setInterval> | undefined;
  let play = { t: 0, loop: 0, anchorT: 0, anchorWall: 0, paused: false };
  let started = { at: "", wall: 0, from: 0 };
  let current: MacroView | null = null;
  // Like the app, every playback that ends goes into the run history.
  const logRun = (reason: FinishReason) => {
    if (!current) return;
    const pb = current.playback;
    const entry: RunEntry = {
      at: started.at,
      macro_id: current.id,
      macro_name: current.name,
      source: "manual",
      outcome: { type: "finished", reason },
      duration_ms: Math.round(performance.now() - started.wall),
      from_ms: started.from,
      loops: play.loop + 1,
      speed: pb.speed,
      humanize: pb.humanize,
      checks: [],
      checks_dropped: 0,
    };
    runs = [entry, ...runs].slice(0, 200);
    emit({ type: "runs_changed" });
  };
  const setMode = (m: Mode) => {
    mode = { ...mode, mode: m };
    emit(mode);
  };
  const halt = () => clearInterval(timer);
  const playTick = () => {
    if (!current) return;
    const pb = current.playback;
    const loops = pb.repeat === "forever" ? null : pb.repeat.count;
    const now = performance.now();
    play.t = play.paused ? play.anchorT : play.anchorT + (now - play.anchorWall) * pb.speed;
    if (play.t >= current.duration) {
      if (loops == null || play.loop + 1 < loops) {
        play = { ...play, loop: play.loop + 1, anchorT: 0, anchorWall: now, t: 0 };
      } else {
        halt();
        emit({ type: "play_tick", t: current.duration, advancing: false, speed: pb.speed, loop_idx: play.loop, loops });
        logRun("completed");
        emit({ type: "finished", reason: "completed", timing: null });
        return setMode("idle");
      }
    }
    emit({ type: "play_tick", t: play.t, advancing: !play.paused, speed: pb.speed, loop_idx: play.loop, loops });
  };

  return {
    editable: false,
    subscribe: async (onMessage) => {
      emit = onMessage;
    },
    toggleRecord: async () => {
      // Like the app: F9 does nothing while a macro plays.
      if (mode.mode === "playing" || mode.mode === "paused") return;
      halt();
      if (mode.mode === "countdown" || mode.mode === "recording") return setMode("idle");
      const startRecording = () => {
        setMode("recording");
        const recStart = performance.now();
        timer = setInterval(() => {
          const desktop = current?.recording.virtual_desktop ?? { x: 0, y: 0, w: 1920, h: 1080 };
          emit({ type: "rec_progress", elapsed_ms: performance.now() - recStart, desktop, moves: [], steps: null });
        }, 100);
      };
      if (!settings.countdown) return startRecording();
      const start = performance.now();
      setMode("countdown");
      timer = setInterval(() => {
        const left = 3000 - (performance.now() - start);
        if (left > 0) return emit({ type: "countdown", left_ms: left });
        halt();
        startRecording();
      }, 50);
    },
    togglePlay: async (from) => {
      if (mode.mode === "playing") {
        play = { ...play, anchorT: play.t, paused: true };
        return setMode("paused");
      }
      if (mode.mode === "paused") {
        play = { ...play, anchorWall: performance.now(), paused: false };
        return setMode("playing");
      }
      if (mode.mode !== "idle" || !current) return;
      play = { t: from, loop: 0, anchorT: from >= current.duration - 1 ? 0 : from, anchorWall: performance.now(), paused: false };
      started = { at: new Date().toISOString(), wall: performance.now(), from: Math.round(play.anchorT) };
      setMode("playing");
      timer = setInterval(playTick, 33);
    },
    stop: async () => {
      halt();
      if (mode.mode === "playing" || mode.mode === "paused") {
        logRun("stopped");
        emit({ type: "finished", reason: "stopped", timing: null });
      }
      if (mode.mode !== "idle") setMode("idle");
    },
    seek: async (t) => {
      play = { ...play, t, anchorT: t, anchorWall: performance.now() };
    },
    listMacros: async () => {
      await ready;
      return items.map((i) => ({
        id: i.view.id,
        name: i.view.name,
        duration: i.view.duration,
        step_count: i.view.steps.length,
        runs: i.runs,
        last_run: i.last_run_ago_ms == null ? null : new Date(loadedAt - i.last_run_ago_ms).toISOString(),
        hotkey: hotkeyOf(triggersFor(i.view.id)),
      }));
    },
    listRuns: async () => {
      await ready;
      return runs;
    },
    loadMacro: async (id) => {
      current = (await find(id)).view;
      mode = { ...mode, macro_id: id };
      return current;
    },
    // The samples were never recorded here, so they have no screenshot.
    screenshot: async () => new ArrayBuffer(0),
    editMacro: async () => unavailable(),
    undoEdit: async () => unavailable(),
    setPlaybackOptions: async (id, options) => {
      const item = await find(id);
      item.view = { ...item.view, playback: options };
      if (current?.id === id) {
        // Keep the simulated playhead continuous across speed changes.
        play = { ...play, anchorT: play.t, anchorWall: performance.now() };
        current = item.view;
      }
      return item.view;
    },
    // The fixture has views, not the events a .rly holds.
    exportMacro: async () => unavailable(),
    importMacros: async () => unavailable(),
    duplicateMacro: async () => unavailable(),
    deleteMacro: async () => unavailable(),
    restoreMacro: async () => unavailable(),
    getSettings: async () => settings,
    updateSettings: async (s) => (settings = s),
    samplePixel: async () => null,
    pickPixel: async () => unavailable(),
    pasteImage: async () => unavailable(),
    chooseImage: async () => unavailable(),
    snipImage: async () => unavailable(),
    cancelSnip: async () => {},
    testFindImage: async () => unavailable(),
    showMatch: async () => {},
    // Triggers only live in memory here, so the tab can be tried out.
    getTriggers: async (id) => {
      await ready; // the sample hotkeys come from the fixture
      return { triggers: triggersFor(id), next_run: null, hotkey_error: null, paused: triggersPaused };
    },
    setTriggers: async (id, t) => {
      browserTriggers.set(id, t);
      return { triggers: t, next_run: null, hotkey_error: null, paused: triggersPaused };
    },
    setTriggersPaused: async (paused) => {
      triggersPaused = paused;
    },
    listProcesses: async () => ["chrome.exe", "excel.exe", "notepad.exe"],
    getAutostart: async () => false,
    setAutostart: async () => false,
  };
}

export const backend: Backend = isTauri() ? tauriBackend : browserBackend();
