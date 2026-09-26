// A stand-in for the Rust side, behind Tauri's own IPC mock: the UI runs its
// real `tauriBackend`, every `invoke` lands here, and tests check exactly
// which commands were sent with which arguments. It keeps a small in-memory
// library (the design's sample macros) and applies edits roughly the way
// relay-core does; the real edit semantics are tested in Rust.
//
// The engine stream is driven by the test: `core.emit(...)` delivers an
// EngineMsg to the UI the way the coordinator would.
import { clearMocks, mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import type { Channel } from "@tauri-apps/api/core";
import samples from "../lib/dev/sample-views.json";
import { DEFAULT_SETTINGS } from "../lib/defaults";
import type { EditOp, MacroListItem, MacroTriggers, MacroView, Settings, Step } from "../lib/types";
import type { EngineMsg } from "../lib/ipc/bindings/EngineMsg";
import type { IpcError } from "../lib/ipc/backend";

export interface Call {
  cmd: string;
  args: Record<string, unknown>;
}

interface Entry {
  view: MacroView;
  runs: number;
  last_run: string | null;
  undo: MacroView[];
  redo: MacroView[];
}

type Handler = (args: Record<string, unknown>) => unknown;

/** A deferred response: the command waits until the test resolves it. */
export interface Held {
  cmd: string;
  args: Record<string, unknown>;
  resolve: (value?: unknown) => void;
  reject: (e: IpcError) => void;
}

const NOW = new Date("2026-09-24T12:00:00Z").getTime();

export const defaultTriggers = (): MacroTriggers => ({
  hotkey: { enabled: false, combo: "" },
  schedule: { enabled: false, schedule: { days: [true, true, true, true, true, false, false], time: "09:00" } },
  app_launch: { enabled: false, exe: "", delay_ms: 2000 },
  pixel: { enabled: false, x: 0, y: 0, color: "#EC3013", tolerance: 8 },
});

export class FakeCore {
  calls: Call[] = [];
  entries: Entry[] = [];
  trash: Entry[] = [];
  settings: Settings = { ...DEFAULT_SETTINGS };
  triggers = new Map<string, MacroTriggers>();
  hotkeyErrors = new Map<string, string>();
  triggersPaused = false;
  autostart = false;
  processes = ["chrome.exe", "EXCEL.EXE", "notepad.exe"];
  /** What the save and open dialogs return (null: the user cancelled). */
  dialog: { save: string | null; open: string[] | string | null } = { save: null, open: null };
  /** The pixel color `sample_pixel` reads, or null when unreadable. */
  pixel: string | null = "#123456";
  picked = { x: 640, y: 360, color: "#00FF00" };
  importResult: { imported: string[]; problems: string[] } | null = null;
  window = { expanded: true };
  private channel: Channel<EngineMsg> | null = null;
  private failures = new Map<string, IpcError>();
  private overrides = new Map<string, Handler>();
  private holding = new Set<string>();
  held: Held[] = [];
  private nextId = 100;

  constructor() {
    this.reset();
  }

  reset() {
    this.calls = [];
    this.entries = (samples as unknown as { view: MacroView; runs: number; last_run_ago_ms: number | null }[]).map(
      (s) => ({
        view: structuredClone(s.view),
        runs: s.runs,
        last_run: s.last_run_ago_ms == null ? null : new Date(NOW - s.last_run_ago_ms).toISOString(),
        undo: [],
        redo: [],
      }),
    );
    const hotkeys = (samples as unknown as { hotkey: string | null }[]).map((s) => s.hotkey);
    this.trash = [];
    this.settings = { ...DEFAULT_SETTINGS };
    this.triggers = new Map();
    this.entries.forEach((e, i) => {
      const t = defaultTriggers();
      if (hotkeys[i]) t.hotkey = { enabled: true, combo: hotkeys[i]! };
      this.triggers.set(e.view.id, t);
    });
    this.hotkeyErrors = new Map();
    this.triggersPaused = false;
    this.autostart = false;
    this.dialog = { save: null, open: null };
    this.pixel = "#123456";
    this.picked = { x: 640, y: 360, color: "#00FF00" };
    this.importResult = null;
    this.window = { expanded: true };
    this.channel = null;
    this.failures = new Map();
    this.overrides = new Map();
    this.holding = new Set();
    this.held = [];
  }

  /** Routes the webview's IPC here (the app's window is "main"). */
  install() {
    mockWindows("main");
    mockIPC((cmd, args) => this.handle(cmd, (args ?? {}) as Record<string, unknown>));
  }

  /** Back to a plain browser, as with `npm run dev`. */
  uninstall() {
    clearMocks();
    delete (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
  }

  // — test controls —

  /** Makes `cmd` fail with this error until `succeed(cmd)`. */
  fail(cmd: string, message = `${cmd} failed`, code = "io") {
    this.failures.set(cmd, { code, message });
  }

  succeed(cmd: string) {
    this.failures.delete(cmd);
  }

  /** Answers `cmd` with `handler` instead of the built-in behavior. */
  on(cmd: string, handler: Handler) {
    this.overrides.set(cmd, handler);
  }

  /** Keeps `cmd`'s responses pending until the test settles them (see `held`). */
  hold(cmd: string) {
    this.holding.add(cmd);
  }

  release(cmd: string) {
    this.holding.delete(cmd);
  }

  /** Delivers an engine message to the UI. */
  emit(msg: EngineMsg) {
    if (!this.channel) throw new Error("the UI hasn't subscribed to the engine");
    (this.channel.onmessage as (m: EngineMsg) => void)(msg);
  }

  get subscribed(): boolean {
    return this.channel != null;
  }

  /** The arguments of every call to `cmd`, in order. */
  argsOf(cmd: string): Record<string, unknown>[] {
    return this.calls.filter((c) => c.cmd === cmd).map((c) => c.args);
  }

  /** The commands sent, ignoring the ones every screen sends while loading. */
  commands(): string[] {
    return this.calls.map((c) => c.cmd);
  }

  lastArgs(cmd: string): Record<string, unknown> | undefined {
    return this.argsOf(cmd).at(-1);
  }

  clearCalls() {
    this.calls = [];
  }

  view(id: string): MacroView {
    return this.entry(id).view;
  }

  get ids(): string[] {
    return this.entries.map((e) => e.view.id);
  }

  // — the commands —

  private handle(cmd: string, args: Record<string, unknown>): unknown {
    // The channel itself isn't interesting to compare.
    const logged = cmd === "subscribe_engine" ? {} : structuredClone(stripChannel(args));
    this.calls.push({ cmd, args: logged });
    if (this.holding.has(cmd)) {
      return new Promise((resolve, reject) => this.held.push({ cmd, args, resolve, reject }));
    }
    const failure = this.failures.get(cmd);
    if (failure) return Promise.reject(failure);
    const override = this.overrides.get(cmd);
    if (override) return override(args);
    try {
      return this.builtin(cmd, args);
    } catch (e) {
      return Promise.reject(e);
    }
  }

  private builtin(cmd: string, a: Record<string, unknown>): unknown {
    const id = a.id as string;
    switch (cmd) {
      case "subscribe_engine":
        this.channel = a.channel as Channel<EngineMsg>;
        return null;
      case "toggle_record":
      case "toggle_play":
      case "stop_session":
      case "seek":
      case "fit_window":
      case "hide_to_tray":
      case "quit":
      case "plugin:window|start_dragging":
        return null;
      case "window_prefs":
        return this.window;
      case "list_macros":
        return this.list();
      case "load_macro":
        return this.withHistory(this.entry(id));
      case "edit_macro": {
        const e = this.entry(id);
        const before = structuredClone(e.view);
        applyEdit(e.view, a.op as EditOp);
        e.undo.push(before);
        e.redo = [];
        return this.withHistory(e);
      }
      case "undo_edit": {
        const e = this.entry(id);
        const [from, to] = a.redo ? [e.redo, e.undo] : [e.undo, e.redo];
        const prev = from.pop();
        if (prev) {
          to.push(e.view);
          e.view = prev;
        }
        return this.withHistory(e);
      }
      case "set_playback_options": {
        const e = this.entry(id);
        e.view = { ...e.view, playback: a.options as MacroView["playback"] };
        return this.withHistory(e);
      }
      case "duplicate_macro": {
        const e = this.entry(id);
        const copyId = this.newId();
        const view = { ...structuredClone(e.view), id: copyId, name: `${e.view.name} (copy)` };
        this.entries.splice(this.entries.indexOf(e) + 1, 0, { view, runs: 0, last_run: null, undo: [], redo: [] });
        this.triggers.set(copyId, defaultTriggers());
        return copyId;
      }
      case "delete_macro": {
        const e = this.entry(id);
        this.entries.splice(this.entries.indexOf(e), 1);
        this.trash.push(e);
        return null;
      }
      case "restore_macro": {
        const i = this.trash.findIndex((e) => e.view.id === id);
        if (i < 0) throw notFound(id);
        this.entries.push(...this.trash.splice(i, 1));
        return null;
      }
      case "export_macro":
        this.entry(id);
        return null;
      case "import_macros": {
        if (this.importResult) return this.importResult;
        const paths = a.paths as string[];
        const imported = paths.map((p) => {
          const view = { ...structuredClone(this.entries[0].view), id: this.newId(), name: basename(p) };
          this.entries.push({ view, runs: 0, last_run: null, undo: [], redo: [] });
          this.triggers.set(view.id, defaultTriggers());
          return view.id;
        });
        return { imported, problems: [] };
      }
      case "plugin:dialog|save":
        return this.dialog.save;
      case "plugin:dialog|open":
        return this.dialog.open;
      case "get_settings":
        return this.settings;
      case "update_settings":
        this.settings = { ...(a.settings as Settings) };
        return this.settings;
      case "sample_pixel":
        return this.pixel;
      case "pick_pixel":
        return this.picked;
      case "get_triggers":
        this.entry(id);
        return this.status(id);
      case "set_triggers":
        this.entry(id);
        this.triggers.set(id, structuredClone(a.triggers as MacroTriggers));
        return this.status(id);
      case "set_triggers_paused":
        this.triggersPaused = a.paused as boolean;
        return null;
      case "list_processes":
        return this.processes;
      case "get_autostart":
        return this.autostart;
      case "set_autostart":
        this.autostart = a.enabled as boolean;
        return this.autostart;
      default:
        throw { code: "unknown_command", message: `no command ${cmd}` } satisfies IpcError;
    }
  }

  private entry(id: string): Entry {
    const e = this.entries.find((e) => e.view.id === id);
    if (!e) throw notFound(id);
    return e;
  }

  private withHistory(e: Entry): MacroView {
    return structuredClone({ ...e.view, can_undo: e.undo.length > 0, can_redo: e.redo.length > 0 });
  }

  private list(): MacroListItem[] {
    return this.entries.map((e) => {
      const t = this.triggers.get(e.view.id);
      return {
        id: e.view.id,
        name: e.view.name,
        duration: e.view.duration,
        step_count: e.view.steps.length,
        runs: e.runs,
        last_run: e.last_run,
        hotkey: t?.hotkey.enabled && t.hotkey.combo ? t.hotkey.combo : null,
      };
    });
  }

  private status(id: string) {
    const next = this.triggers.get(id)!.schedule.enabled ? "2026-09-25T09:00:00+02:00" : null;
    return {
      triggers: structuredClone(this.triggers.get(id)!),
      next_run: next,
      hotkey_error: this.hotkeyErrors.get(id) ?? null,
      paused: this.triggersPaused,
    };
  }

  private newId(): string {
    return `00000000-0000-0000-0000-${String(this.nextId++).padStart(12, "0")}`;
  }
}

const notFound = (id: string): IpcError => ({ code: "not_found", message: `no macro with id ${id}` });
const basename = (p: string) => p.split(/[\\/]/).pop()!.replace(/\.(rly|json)$/, "");

function stripChannel(args: Record<string, unknown>): Record<string, unknown> {
  const { channel: _, ...rest } = args;
  return rest;
}

/** A rough relay-core: enough for the UI to show the result of each edit. */
function applyEdit(v: MacroView, op: EditOp) {
  const step = (i: number) => {
    const s = v.steps[i];
    if (!s) throw { code: "edit_rejected", message: `no step ${i}` } satisfies IpcError;
    return s;
  };
  const insert = (s: Step) => {
    const i = v.steps.findIndex((x) => x.t > s.t);
    v.steps.splice(i < 0 ? v.steps.length : i, 0, s);
    v.duration = Math.max(v.duration, s.end);
  };
  const nextItem = () => Math.max(0, ...v.steps.flatMap((s) => s.items)) + 1;
  switch (op.op) {
    case "rename":
      v.name = op.name;
      break;
    case "delete_step":
      step(op.index);
      v.steps.splice(op.index, 1);
      break;
    case "insert_wait":
      insert({ kind: "wait", t: op.at, end: op.at + op.dur, pause: 0, items: [nextItem()], dur: op.dur, label: op.label });
      break;
    case "insert_pixel_wait": {
      const { at, dur, x, y, color, tolerance, timeout_ms, label } = op;
      insert({ kind: "pixel_wait", t: at, end: at + dur, pause: 0, items: [nextItem()], dur, x, y, color, tolerance, timeout_ms, label });
      break;
    }
    case "set_wait_duration": {
      const s = step(op.index);
      if (s.kind === "wait") Object.assign(s, { dur: op.dur, end: s.t + op.dur });
      break;
    }
    case "update_pixel_wait": {
      const s = step(op.index);
      const { x, y, color, tolerance, timeout_ms } = op;
      if (s.kind === "pixel_wait") Object.assign(s, { x, y, color, tolerance, timeout_ms });
      break;
    }
    case "set_label": {
      const s = step(op.index);
      if ("label" in s) s.label = op.label;
      break;
    }
    case "set_pause":
      step(op.index).pause = op.dur;
      break;
    case "cap_pauses":
      for (const s of v.steps) s.pause = Math.min(s.pause, op.max);
      break;
  }
}

export const core = new FakeCore();
