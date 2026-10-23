// A stand-in for the Rust side, behind Tauri's own IPC mock: the UI runs its
// real `tauriBackend`, every `invoke` lands here, and tests check exactly
// which commands were sent with which arguments. It keeps a small in-memory
// library (the design's sample macros) and answers like src-tauri does:
// edits and their history (fake-edit.ts), duplicate, trash and import,
// triggers and their hotkey rules, settings, the busy guard, the arguments
// Rust would refuse, and failed saves reported on the engine stream.
//
// The engine stream is driven by the test: `core.emit(...)` delivers an
// EngineMsg to the UI the way the coordinator would, `core.emitLater(...)`
// delivers messages one at a time with animation frames in between.
import { clearMocks, mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import type { Channel } from "@tauri-apps/api/core";
import samples from "../lib/dev/sample-views.json";
import { DEFAULT_SETTINGS } from "../lib/defaults";
import type { EditOp, MacroListItem, MacroTriggers, MacroView, Mode, PlaybackOptions, Settings } from "../lib/types";
import type { EngineMsg } from "../lib/ipc/bindings/EngineMsg";
import type { Panes } from "../lib/ipc/bindings/Panes";
import type { FoundImage } from "../lib/ipc/bindings/FoundImage";
import type { IpcError } from "../lib/ipc/backend";
import type { RunEntry } from "../lib/ipc/bindings/RunEntry";
import type { DataFileInfo } from "../lib/ipc/bindings/DataFileInfo";
import { applyEdit, columnsUsed, fillTemplate, History, parseTemplate } from "./fake-edit";

/** A data file's contents, as relay-core reads them. */
export interface DataTable {
  columns: string[];
  rows: string[][];
}

export interface Call {
  cmd: string;
  args: Record<string, unknown>;
}

interface Entry {
  view: MacroView;
  runs: number;
  last_run: string | null;
  history: History;
  /** The data file's path (library.json's `data_file`). */
  dataFile?: string;
}

type Handler = (args: Record<string, unknown>) => unknown;

/** A deferred response: the command waits until the test resolves it. */
export interface Held {
  cmd: string;
  args: Record<string, unknown>;
  resolve: (value?: unknown) => void;
  reject: (e: IpcError) => void;
}

/** "Now" for the fake: Thursday 24 September 2026, 14:00 at +02:00. */
const NOW = new Date("2026-09-24T12:00:00Z").getTime();
const OFFSET = "+02:00";

export const defaultTriggers = (): MacroTriggers => ({
  hotkey: { enabled: false, combo: "" },
  schedule: { enabled: false, schedule: { days: [true, true, true, true, true, false, false], time: "09:00" } },
  app_launch: { enabled: false, exe: "", delay_ms: 2000 },
  pixel: { enabled: false, x: 0, y: 0, color: "#EC3013", tolerance: 8 },
  image: { enabled: false, image: null, threshold: 85, area: null },
});

interface Sample {
  view: MacroView;
  runs: number;
  last_run_ago_ms: number | null;
  hotkey: string | null;
}
const SAMPLES = samples as unknown as Sample[];

/** Commands that return no `Result` in Rust: they can't fail, so tests mustn't pretend they do. */
const INFALLIBLE = new Set([
  "subscribe_engine",
  "toggle_record",
  "toggle_play",
  "stop_session",
  "seek",
  "list_macros",
  "list_runs",
  "import_macros",
  "get_settings",
  "list_processes",
  "get_autostart",
  "window_prefs",
  "fit_window",
  "hide_to_tray",
  "quit",
  "screenshot",
  "save_panes",
  "reset_layout",
  "cancel_snip",
  "show_match",
]);

const BUSY: IpcError = { code: "busy", message: "Stop the recording or playback first" };

export class FakeCore {
  calls: Call[] = [];
  entries: Entry[] = [];
  trash: { entry: Entry; position: number }[] = [];
  settings: Settings = { ...DEFAULT_SETTINGS };
  triggers = new Map<string, MacroTriggers>();
  /** Why a macro's hotkey didn't register (e.g. another app owns it), as the hotkey thread would report. */
  hotkeyErrors = new Map<string, string>();
  triggersPaused = false;
  /** runs.json, newest first (Rust starts with none; tests add what they need). */
  runLog: RunEntry[] = [];
  autostart = false;
  processes = ["chrome.exe", "EXCEL.EXE", "notepad.exe"];
  /**
   * The session mode, as the coordinator holds it: follows the `session`
   * messages the test emits (or set it directly). Edits and deletes are
   * refused as busy unless it's idle.
   */
  mode: Mode = "idle";
  /** What the save and open dialogs return (null: the user cancelled). */
  dialog: { save: string | null; open: string[] | string | null } = { save: null, open: null };
  /**
   * What each file holds for `import_macros`: a macro, or why it can't be
   * read. A file not listed holds a copy of the first sample, named after the file.
   */
  files = new Map<string, MacroView | string>();
  /** The screenshots taken when macros were recorded (JPEG bytes), by macro id; like Rust's `screens\<id>.jpg`. */
  screens = new Map<string, Uint8Array>();
  /** When set, saving to disk fails with this: changes are kept and an `error` message says so. */
  saveError: string | null = null;
  /** The pixel color `sample_pixel` reads, or null when unreadable. */
  pixel: string | null = "#123456";
  picked = { x: 640, y: 360, color: "#00FF00" };
  /** The picture on the clipboard (a base64 PNG, as `paste_image` makes it), or null. */
  clipboard: string | null = null;
  /** The text on the clipboard, for `{clipboard}`, or null. */
  clipboardText: string | null = null;
  /**
   * Data files by path: what they hold, or why they can't be read (data_file.rs's
   * message). A path not listed isn't there.
   */
  csv = new Map<string, DataTable | string>();
  /** The local time `preview_text` fills in. */
  localNow = "2026-10-01T09:05:07";
  /** What `snip_image` returns: the snip, or null when the user cancelled. */
  snip: string | null = null;
  /** Image files for `load_image`, by path: the image, or why it can't be used. */
  images = new Map<string, string | IpcError>();
  /** What `test_find_image` finds. */
  found: FoundImage | null = null;
  /** window.json as Rust holds it: the mode and the editor's dividers (the size isn't the page's to see). */
  window: { expanded: boolean; panes: Panes } = { expanded: true, panes: { preview_w: null, transport_h: null, timeline_h: null } };
  private channel: Channel<EngineMsg> | null = null;
  /** Errors and notices sent before the UI subscribed, delivered when it does. */
  private queued: EngineMsg[] = [];
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
    this.screens = new Map();
    this.entries = SAMPLES.map((s) => ({
      view: structuredClone(s.view),
      runs: s.runs,
      last_run: s.last_run_ago_ms == null ? null : new Date(NOW - s.last_run_ago_ms).toISOString(),
      history: new History(),
    }));
    this.trash = [];
    this.runLog = [];
    this.settings = { ...DEFAULT_SETTINGS };
    this.triggers = new Map();
    // Like library.rs seed_samples: the design's hotkeys are shown but off.
    SAMPLES.forEach((s) => {
      const t = defaultTriggers();
      if (s.hotkey) t.hotkey = { enabled: false, combo: s.hotkey };
      this.triggers.set(s.view.id, t);
    });
    this.hotkeyErrors = new Map();
    this.triggersPaused = false;
    this.autostart = false;
    this.mode = "idle";
    this.dialog = { save: null, open: null };
    this.files = new Map();
    this.saveError = null;
    this.pixel = "#123456";
    this.picked = { x: 640, y: 360, color: "#00FF00" };
    this.clipboard = null;
    this.clipboardText = null;
    this.localNow = "2026-10-01T09:05:07";
    this.snip = null;
    this.images = new Map();
    this.found = null;
    this.window = { expanded: true, panes: { preview_w: null, transport_h: null, timeline_h: null } };
    this.channel = null;
    this.queued = [];
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
    if (INFALLIBLE.has(cmd)) throw new Error(`${cmd} can't fail in Rust: emit an error message instead`);
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

  /**
   * Delivers an engine message to the UI now. Before the UI subscribes,
   * errors and notices are queued (Rust delivers them on subscribe); anything
   * else is a mistake in the test.
   */
  emit(msg: EngineMsg) {
    if (msg.type === "session") this.mode = msg.mode;
    if (!this.channel) {
      if (msg.type === "error" || msg.type === "notice") return void this.queued.push(msg);
      throw new Error("the UI hasn't subscribed to the engine");
    }
    (this.channel.onmessage as (m: EngineMsg) => void)(msg);
  }

  /**
   * Delivers messages one by one, each after a pause long enough for an
   * animation frame to run, as they arrive from the coordinator's thread.
   * With fake timers, advance them (20 ms per message) for it to finish.
   */
  async emitLater(...msgs: EngineMsg[]) {
    for (const msg of msgs) {
      await new Promise((r) => setTimeout(r, 20));
      this.emit(msg);
    }
  }

  get subscribed(): boolean {
    return this.channel != null;
  }

  /** The arguments of every call to `cmd`, in order. */
  argsOf(cmd: string): Record<string, unknown>[] {
    return this.calls.filter((c) => c.cmd === cmd).map((c) => c.args);
  }

  /** The commands sent, in order. */
  commands(): string[] {
    return this.calls.map((c) => c.cmd);
  }

  lastArgs(cmd: string): Record<string, unknown> | undefined {
    return this.argsOf(cmd).at(-1);
  }

  clearCalls() {
    this.calls = [];
  }

  /** A copy of the macro as the fake holds it now. */
  view(id: string): MacroView {
    return structuredClone(this.entry(id).view);
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
    const check = (arg: string, value: unknown, spec: Spec) => validate(cmd, arg, value, spec);
    switch (cmd) {
      case "subscribe_engine":
        this.channel = a.channel as Channel<EngineMsg>;
        for (const msg of this.queued.splice(0)) this.emit(msg);
        return null;
      case "toggle_play":
        check("from", a.from, "f64");
        return null;
      case "seek":
        check("t", a.t, "f64");
        return null;
      case "toggle_record":
      case "stop_session":
      case "hide_to_tray":
      case "quit":
      case "plugin:window|start_dragging":
        return null;
      case "fit_window":
        check("width", a.width, "f64");
        check("height", a.height, "f64");
        check("expanded", a.expanded, "bool");
        // window_ctl::fit remembers the mode, for the next start.
        this.window.expanded = a.expanded as boolean;
        return null;
      case "list_runs":
        return structuredClone(this.runLog);
      case "window_prefs":
        return structuredClone(this.window);
      case "save_panes": {
        check("panes", a.panes, PANES);
        // As Panes::sanitized: what isn't a size is dropped, a huge one capped.
        const ok = (v: number | null | undefined) => (v != null && Number.isFinite(v) && v > 0 ? Math.min(v, 10_000) : null);
        const p = a.panes as Partial<Panes>;
        this.window.panes = { preview_w: ok(p.preview_w), transport_h: ok(p.transport_h), timeline_h: ok(p.timeline_h) };
        return null;
      }
      case "reset_layout":
        this.window.panes = { preview_w: null, transport_h: null, timeline_h: null };
        return null;
      case "list_macros":
        return this.list();
      case "load_macro":
        return this.withHistory(this.entry(id));
      case "screenshot": {
        // Raw bytes, as a Tauri `Response` arrives: empty when there's none (never an error).
        check("id", id, "string");
        const bytes = this.screens.get(id) ?? new Uint8Array();
        return bytes.slice().buffer;
      }
      case "edit_macro": {
        const e = this.entry(id);
        const op = a.op as EditOp;
        check("op", op, EDIT_OPS[op?.op] ?? `unknown variant \`${op?.op}\``);
        if (this.mode !== "idle") throw BUSY;
        const before = History.before(e.view, op);
        applyEdit(e.view, op);
        e.history.record(before, op, e.view);
        this.saved();
        return this.withHistory(e);
      }
      case "undo_edit": {
        const e = this.entry(id);
        if (this.mode !== "idle") throw BUSY;
        if (e.history.step(e.view, a.redo as boolean)) this.saved();
        return this.withHistory(e);
      }
      case "set_playback_options": {
        const e = this.entry(id);
        check("options", a.options, PLAYBACK);
        e.view = { ...e.view, playback: structuredClone(a.options as PlaybackOptions) };
        this.saved();
        return this.withHistory(e);
      }
      case "duplicate_macro": {
        const e = this.entry(id);
        const copyId = this.newId();
        const view = { ...structuredClone(e.view), id: copyId, name: this.uniqueName(`${e.view.name} (copy)`) };
        // A copy starts with no triggers (two macros on one hotkey would collide) and no history.
        const copy = { view, runs: 0, last_run: null, history: new History(), dataFile: e.dataFile };
        this.entries.splice(this.entries.indexOf(e) + 1, 0, copy);
        this.triggers.set(copyId, defaultTriggers());
        const screen = this.screens.get(id);
        if (screen) this.screens.set(copyId, screen.slice());
        return copyId;
      }
      case "delete_macro": {
        if (this.mode !== "idle") throw BUSY;
        const e = this.entry(id);
        const position = this.entries.indexOf(e);
        this.entries.splice(position, 1);
        this.trash.push({ entry: e, position });
        return null;
      }
      case "restore_macro": {
        const i = this.trash.findIndex((t) => t.entry.view.id === id);
        if (i < 0) throw notFound(id);
        const [{ entry, position }] = this.trash.splice(i, 1);
        this.entries.splice(Math.min(position, this.entries.length), 0, entry);
        // Its hotkey may have gone to another macro meanwhile: it comes back off.
        const t = this.triggers.get(id)!;
        const taken = t.hotkey.enabled ? this.takenBy(id, t.hotkey.combo) : null;
        if (taken) {
          this.triggers.set(id, { ...t, hotkey: { ...t.hotkey, enabled: false } });
          this.emit({
            type: "notice",
            message: `“${entry.view.name}” is back; its hotkey ${t.hotkey.combo} is now used by “${taken}”, so it's off`,
          });
        }
        return null;
      }
      case "export_macro": {
        check("format", a.format, oneOf("rly", "json", "exe"));
        const e = this.entry(id);
        // A program carries the data file's rows, so it needs them now.
        if (a.format === "exe") this.forPlaying(texts(e.view), e.dataFile);
        return null;
      }
      case "import_macros":
        return this.import(a.paths as string[]);
      case "plugin:dialog|save":
        return this.dialog.save;
      case "plugin:dialog|open":
        return this.dialog.open;
      case "get_settings":
        return this.settings;
      case "update_settings":
        // Missing fields fall back to the defaults (settings.rs has #[serde(default)]).
        check("settings", a.settings, SETTINGS);
        this.settings = { ...DEFAULT_SETTINGS, ...(a.settings as Partial<Settings>) };
        this.saved();
        return this.settings;
      case "sample_pixel":
        check("x", a.x, "i32");
        check("y", a.y, "i32");
        return this.pixel;
      case "pick_pixel":
        check("delayMs", a.delayMs, "u32");
        return this.picked;
      case "paste_image":
        if (!this.clipboard) throw { code: "image", message: "There's no picture on the clipboard. Copy or snip one first." } satisfies IpcError;
        return this.clipboard;
      case "load_image": {
        check("path", a.path, "string");
        const image = this.images.get(a.path as string);
        if (image == null) {
          throw { code: "io", message: "Couldn't read that file: The system cannot find the file specified. (os error 2)" } satisfies IpcError;
        }
        if (typeof image !== "string") throw image;
        return image;
      }
      case "snip_image":
        return this.snip;
      case "cancel_snip":
        return null;
      case "show_match":
        check("area", a.area, RECT);
        check("dotX", a.dotX, "i32");
        check("dotY", a.dotY, "i32");
        return null;
      case "preview_text": {
        check("text", a.text, "string");
        const parsed = parseTemplate(a.text as string);
        if ("error" in parsed) throw { code: "invalid_text", message: parsed.error } satisfies IpcError;
        const data = parsed.parts.some((p) => "column" in p) ? this.forPlaying([a.text as string], this.entry(id).dataFile) : null;
        return fillTemplate(a.text as string, 1, this.localNow, this.clipboardText, (name) => value(data, 0, name));
      }
      case "get_data_file":
        return this.dataInfo(this.entry(id));
      case "set_data_file": {
        const e = this.entry(id);
        check("path", a.path, nullable("string"));
        const path = (a.path as string | null) ?? undefined;
        if (path) this.readData(path);
        e.dataFile = path;
        this.saved();
        return this.dataInfo(e);
      }
      case "test_find_image":
        check("image", a.image, "png");
        check("threshold", a.threshold, "u8");
        check("area", a.area, nullable(RECT));
        return this.found;
      case "get_triggers":
        this.entry(id);
        return this.status(id);
      case "set_triggers": {
        this.entry(id);
        check("triggers", a.triggers, TRIGGERS);
        const t = withDefaults(a.triggers as Partial<MacroTriggers>);
        if (t.hotkey.enabled) {
          const why = this.conflict(id, t.hotkey.combo);
          if (why) throw { code: "hotkey", message: why } satisfies IpcError;
        }
        this.triggers.set(id, t);
        return this.status(id);
      }
      case "set_triggers_paused":
        this.triggersPaused = a.paused as boolean;
        // The coordinator applies it and tells the UI, after the command returned.
        {
          // (Not to a store a later test subscribed.)
          const channel = this.channel;
          setTimeout(() => channel && channel === this.channel && this.emit({ type: "triggers_paused", paused: a.paused as boolean }));
        }
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

  /** data_file.rs `read`: the table, or its message as a `data_file` error. */
  private readData(path: string): DataTable {
    const found = this.csv.get(path);
    const name = fileName(path);
    if (found === undefined) throw dataError(`${name} isn't there anymore: choose it again in Settings → Playback.`);
    if (typeof found === "string") throw dataError(found);
    return found;
  }

  /** data_file.rs `for_playing`, with relay-core's `check`. */
  private forPlaying(templates: string[], path: string | undefined): DataTable | null {
    const table = path ? this.readData(path) : null;
    const why = checkData(templates, table, path ? fileName(path) : "");
    if (why) throw dataError(why);
    return table;
  }

  private dataInfo(e: Entry): DataFileInfo | null {
    if (!e.dataFile) return null;
    try {
      const t = this.readData(e.dataFile);
      const error = checkData(texts(e.view), t, fileName(e.dataFile));
      return { path: e.dataFile, columns: t.columns, rows: t.rows.length, error };
    } catch (err) {
      return { path: e.dataFile, columns: [], rows: 0, error: (err as IpcError).message };
    }
  }

  private entry(id: string): Entry {
    const e = this.entries.find((e) => e.view.id === id);
    if (!e) throw notFound(id);
    return e;
  }

  /** A change was saved: a failed write keeps it in memory and says so (commands.rs report_unsaved). */
  private saved() {
    if (this.saveError) this.emit({ type: "error", message: `Couldn't save the change: ${this.saveError}. It's kept until you quit.` });
  }

  private withHistory(e: Entry): MacroView {
    return structuredClone({ ...e.view, can_undo: e.history.undo.length > 0, can_redo: e.history.redo.length > 0 });
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

  /** `name`, or `name 2`, `name 3`… if a macro already has it. */
  private uniqueName(name: string): string {
    const taken = (n: string) => this.entries.some((e) => e.view.name === n);
    if (!taken(name)) return name;
    for (let i = 2; ; i++) if (!taken(`${name} ${i}`)) return `${name} ${i}`;
  }

  /** Like library.rs import: each file on its own, added at the top in order. */
  private import(paths: string[]) {
    const imported: string[] = [];
    const problems: string[] = [];
    for (const path of paths) {
      const file = path.split(/[\\/]/).pop()!;
      const held = this.files.get(path) ?? { ...structuredClone(SAMPLES[0].view), name: file.replace(/\.(rly|json|exe)$/, "") };
      if (typeof held === "string") {
        problems.push(`${file}: ${held}`);
        continue;
      }
      const view = structuredClone(held);
      if (this.entries.some((e) => e.view.id === view.id) || this.trash.some((t) => t.entry.view.id === view.id)) {
        view.id = this.newId();
      }
      view.name = this.uniqueName(view.name);
      this.entries.splice(imported.length, 0, { view, runs: 0, last_run: null, history: new History() });
      this.triggers.set(view.id, defaultTriggers());
      imported.push(view.id);
    }
    return { imported, problems };
  }

  /** The macro (other than `id`) whose enabled hotkey is `combo`, written either way. */
  private takenBy(id: string, combo: string): string | null {
    const s = parseCombo(combo);
    if (typeof s === "string") return null;
    for (const e of this.entries) {
      const t = this.triggers.get(e.view.id);
      if (e.view.id === id || !t?.hotkey.enabled) continue;
      const o = parseCombo(t.hotkey.combo);
      if (typeof o !== "string" && sameShortcut(o, s)) return e.view.name;
    }
    return null;
  }

  /** Why `combo` can't be `id`'s hotkey (hotkeys.rs conflict). */
  private conflict(id: string, combo: string): string | null {
    const s = parseCombo(combo);
    if (typeof s === "string") return s;
    if (RELAYS_OWN.some((r) => sameShortcut(r, s))) return `${combo} is one of Relay's own hotkeys`;
    const other = this.takenBy(id, combo);
    return other ? `${combo} already runs “${other}”` : null;
  }

  private status(id: string) {
    const t = this.triggers.get(id)!;
    return {
      triggers: structuredClone(t),
      next_run: t.schedule.enabled ? nextRun(t.schedule.schedule.days, t.schedule.schedule.time) : null,
      hotkey_error: t.hotkey.enabled ? (this.hotkeyErrors.get(id) ?? null) : null,
      paused: this.triggersPaused,
    };
  }

  private newId(): string {
    return `00000000-0000-0000-0000-${String(this.nextId++).padStart(12, "0")}`;
  }
}

const notFound = (id: string): IpcError => ({ code: "not_found", message: `no macro with id ${id}` });

function stripChannel(args: Record<string, unknown>): Record<string, unknown> {
  const { channel: _, ...rest } = args;
  return rest;
}

function withDefaults(t: Partial<MacroTriggers>): MacroTriggers {
  const d = defaultTriggers();
  const out: MacroTriggers = {
    hotkey: { ...d.hotkey, ...t.hotkey },
    schedule: { ...d.schedule, ...t.schedule },
    app_launch: { ...d.app_launch, ...t.app_launch },
    pixel: { ...d.pixel, ...t.pixel },
    image: { ...d.image, ...t.image },
  };
  out.pixel.color = out.pixel.color.toUpperCase();
  return structuredClone(out);
}

/** The next scheduled run from NOW, as RFC 3339 at +02:00 (relay-core's next_run). */
function nextRun(days: boolean[], time: string): string | null {
  const [h, m] = time.split(":").map(Number);
  const local = new Date(NOW + 2 * 3_600_000); // wall clock at +02:00, read with the UTC getters
  const nowMinutes = local.getUTCHours() * 60 + local.getUTCMinutes();
  for (let add = 0; add <= 7; add++) {
    const d = new Date(Date.UTC(local.getUTCFullYear(), local.getUTCMonth(), local.getUTCDate() + add));
    if (!days[(d.getUTCDay() + 6) % 7] || (add === 0 && h * 60 + m <= nowMinutes)) continue;
    const pad = (n: number) => String(n).padStart(2, "0");
    return `${d.getUTCFullYear()}-${pad(d.getUTCMonth() + 1)}-${pad(d.getUTCDate())}T${pad(h)}:${pad(m)}:00${OFFSET}`;
  }
  return null;
}

// — hotkeys.rs —

interface Shortcut {
  mods: string;
  code: string;
}

const MODIFIERS = ["Ctrl", "Alt", "Shift", "Win"];
const PUNCTUATION: Record<string, string> = {
  "-": "Minus",
  "=": "Equal",
  "[": "BracketLeft",
  "]": "BracketRight",
  "\\": "Backslash",
  ";": "Semicolon",
  "'": "Quote",
  ",": "Comma",
  ".": "Period",
  "/": "Slash",
  "`": "Backquote",
};
const NAMED: Record<string, string> = {
  Ctrl: "ControlLeft",
  Alt: "AltLeft",
  Shift: "ShiftLeft",
  Win: "MetaLeft",
  Esc: "Escape",
  PgUp: "PageUp",
  PgDn: "PageDown",
  Del: "Delete",
  Ins: "Insert",
  Left: "ArrowLeft",
  Right: "ArrowRight",
  Up: "ArrowUp",
  Down: "ArrowDown",
};
const OTHER_CODES = ["Enter", "Tab", "Space", "Home", "End", "Backspace", "Escape", "PageUp", "PageDown", "Delete", "Insert"];

/** relay-core's code_for_label, then whether the global-shortcut plugin knows the code. */
function codeFor(label: string): string | null {
  let code: string;
  if (NAMED[label]) code = NAMED[label];
  else if (label.startsWith("Num ")) code = "Numpad" + label.slice(4);
  else if (/^[a-z]$/i.test(label)) code = "Key" + label.toUpperCase();
  else if (/^\d$/.test(label)) code = "Digit" + label;
  else if (label.length === 1) code = PUNCTUATION[label] ?? label;
  else code = label;
  const known =
    /^(Key[A-Z]|Digit\d|Numpad\d|F([1-9]|1\d|2[0-4])|Arrow(Left|Right|Up|Down)|(Control|Alt|Shift|Meta)Left)$/.test(code) ||
    OTHER_CODES.includes(code) ||
    Object.values(PUNCTUATION).includes(code);
  return known ? code : null;
}

/** Keys that type a character, so Shift alone doesn't make them a hotkey. */
const typesCharacter = (code: string) => /^(Key[A-Z]|Digit\d|Space)$/.test(code) || Object.values(PUNCTUATION).includes(code);

/** Parses a combo as shown in the UI ("Ctrl + Alt + 1"), or says why it can't be a hotkey. */
export function parseCombo(combo: string): Shortcut | string {
  // Like relay_core::keys::split_combo: a trailing "+ +" is the plus key, any other empty part a typo.
  let parts = combo.split("+").map((p) => p.trim());
  if (parts.length >= 2 && parts.at(-1) === "" && parts.at(-2) === "") parts = [...parts.slice(0, -2), "+"];
  if (parts.some((p) => p === "")) return "The hotkey is empty or incomplete";
  const key = parts.pop()!;
  const mods = new Set<string>();
  for (const p of parts) {
    if (!MODIFIERS.includes(p)) return `“${p}” isn't a modifier (use Ctrl, Alt, Shift or Win)`;
    mods.add(p);
  }
  const code = codeFor(key);
  if (!code) return `“${key}” isn't a key Relay can use`;
  if (MODIFIERS.includes(key)) return `Add a key after ${key}: a hotkey can't end with a modifier`;
  const fkey = /^F([1-9]|1\d|2[0-4])$/.test(key);
  if (!mods.size && !fkey) return "Add Ctrl, Alt, Shift or Win, so the key still types normally";
  if (mods.size === 1 && mods.has("Shift") && typesCharacter(code)) {
    return `Add Ctrl, Alt or Win: Shift + ${key} is ordinary typing`;
  }
  return { mods: MODIFIERS.filter((m) => mods.has(m)).join("+"), code };
}

const sameShortcut = (a: Shortcut, b: Shortcut) => a.mods === b.mods && a.code === b.code;

const RELAYS_OWN: Shortcut[] = [
  { mods: "", code: "F9" },
  { mods: "", code: "F10" },
  { mods: "Ctrl+Shift", code: "KeyM" },
  { mods: "Ctrl+Alt", code: "End" },
];

// — argument types: what serde would refuse before a command runs —

type Kind = "u32" | "i32" | "u8" | "f64" | "bool" | "string" | "color" | "time" | "days" | "png";
type Spec = Kind | { [field: string]: Spec | [Spec, "optional"] } | ((v: unknown) => string | null) | string;

const INT: Record<string, [number, number]> = { u32: [0, 2 ** 32 - 1], i32: [-(2 ** 31), 2 ** 31 - 1], u8: [0, 255] };

function why(v: unknown, spec: Spec): string | null {
  if (typeof spec === "function") return spec(v);
  if (typeof spec === "object") {
    if (typeof v !== "object" || v == null) return `invalid type: ${JSON.stringify(v)}, expected a struct`;
    for (const [field, s] of Object.entries(spec)) {
      const [fieldSpec, optional] = Array.isArray(s) ? [s[0], true] : [s, false];
      const value = (v as Record<string, unknown>)[field];
      if (value === undefined) {
        if (optional) continue;
        return `missing field \`${field}\``;
      }
      const w = why(value, fieldSpec);
      if (w) return w;
    }
    return null;
  }
  switch (spec) {
    case "u32":
    case "i32":
    case "u8": {
      if (typeof v !== "number") return `invalid type: ${JSON.stringify(v)}, expected ${spec}`;
      if (!Number.isInteger(v)) return `invalid type: floating point \`${v}\`, expected ${spec}`;
      const [min, max] = INT[spec];
      return v < min || v > max ? `invalid value: integer \`${v}\`, expected ${spec}` : null;
    }
    case "f64":
      return typeof v === "number" && Number.isFinite(v) ? null : `invalid type: ${JSON.stringify(v)}, expected f64`;
    case "bool":
      return typeof v === "boolean" ? null : `invalid type: ${JSON.stringify(v)}, expected a boolean`;
    case "string":
      return typeof v === "string" ? null : `invalid type: ${JSON.stringify(v)}, expected a string`;
    case "color":
      return typeof v === "string" && /^#[0-9a-fA-F]{6}$/.test(v) ? null : `invalid color ${JSON.stringify(v)}`;
    case "time":
      return typeof v === "string" && /^([01]\d|2[0-3]):[0-5]\d$/.test(v) ? null : `invalid time ${JSON.stringify(v)}`;
    case "days":
      return Array.isArray(v) && v.length === 7 && v.every((d) => typeof d === "boolean") ? null : "expected 7 days";
    case "png":
      // ImagePng: base64 of bytes that start like a PNG.
      return typeof v === "string" && v.startsWith("iVBORw0KGgo") ? null : "invalid image";
    default:
      return spec; // a fixed refusal, e.g. an unknown variant
  }
}

/** Rejects like Tauri does when an argument doesn't deserialize: with a plain string. */
function validate(cmd: string, arg: string, value: unknown, spec: Spec) {
  const w = why(value, spec);
  if (w) throw `invalid args \`${arg}\` for command \`${cmd}\`: ${w}`;
}

const oneOf =
  (...values: string[]) =>
  (v: unknown) =>
    values.includes(v as string) ? null : `unknown variant ${JSON.stringify(v)}, expected one of ${values.join(", ")}`;


const PLAYBACK: Spec = {
  speed: "f64",
  repeat: (v: unknown) => (v === "forever" ? null : why(v, { count: "u32" })),
  humanize: "bool",
  jitter_ms: "u32",
  stop_on_key: "bool",
  coord_mode: oneOf("screen", "window"),
};

const optional = (s: Spec): [Spec, "optional"] => [s, "optional"];
/** An `Option<T>`: null, or a T. (JSON has no NaN or Infinity: over IPC they arrive as null.) */
const fileName = (path: string) => path.split(/[\\/]/).pop() ?? path;
const dataError = (message: string): IpcError => ({ code: "data_file", message });
const texts = (v: MacroView) => v.steps.flatMap((s) => (s.kind === "text" ? [s.text] : []));
const sameName = (a: string, b: string) => a.trim().toLowerCase() === b.trim().toLowerCase();

/** relay-core `DataTable::value`: a column of row `row`, empty past the row's end. */
function value(t: DataTable | null, row: number, name: string): string | null {
  const col = t?.columns.findIndex((c) => sameName(c, name)) ?? -1;
  if (!t || col < 0 || row >= t.rows.length) return null;
  return t.rows[row][col] ?? "";
}

/** relay-core `data::check`: why a macro typing `templates` can't play with `table`. */
function checkData(templates: string[], table: DataTable | null, file: string): string | null {
  const used = columnsUsed(templates);
  if (!table) return used.length ? `A Text step types {col:${used[0]}}: choose a data file in Settings → Playback.` : null;
  const missing = used.find((u) => !table.columns.some((c) => sameName(c, u)));
  if (missing) return `${file} has no column “${missing}”.`;
  if (!table.rows.length) return `${file} has no rows to play.`;
  return null;
}

const nullable = (s: Spec) => (v: unknown) => (v === null || (typeof v === "number" && !Number.isFinite(v)) ? null : why(v, s));
const RECT: Spec = { x: "i32", y: "i32", w: "i32", h: "i32" };
const BUTTON = oneOf("Left", "Right", "Middle", "X1", "X2");
/** A Find image step's settings; `area` is an `Option`, so it may be left out. */
const FIND_IMAGE = {
  image: "png",
  click_x: "i32",
  click_y: "i32",
  btn: BUTTON,
  threshold: "u8",
  timeout_ms: "u32",
  area: optional(nullable(RECT)),
} satisfies Record<string, Spec | [Spec, "optional"]>;

const EDIT_OPS: Record<string, Spec> = {
  rename: { name: "string" },
  delete_step: { index: "u32" },
  insert_wait: { at: "u32", dur: "u32", label: "string" },
  insert_pixel_wait: { at: "u32", dur: "u32", x: "i32", y: "i32", color: "color", tolerance: "u8", timeout_ms: "u32", label: "string" },
  set_wait_duration: { index: "u32", dur: "u32" },
  update_pixel_wait: { index: "u32", x: "i32", y: "i32", color: "color", tolerance: "u8", timeout_ms: "u32" },
  insert_find_image: { at: "u32", dur: "u32", ...FIND_IMAGE, label: "string" },
  update_find_image: { index: "u32", ...FIND_IMAGE },
  set_label: { index: "u32", label: "string" },
  set_pause: { index: "u32", dur: "u32" },
  cap_pauses: { max: "u32" },
  set_move_duration: { index: "u32", dur: "u32" },
  smooth_move: { index: "u32" },
  straighten_move: { index: "u32" },
  insert_text: { at: "u32", text: "string" },
  update_text: { index: "u32", text: "string" },
  make_editable: { index: "u32" },
};

const PANES: Spec = {
  preview_w: optional(nullable("f64")),
  transport_h: optional(nullable("f64")),
  timeline_h: optional(nullable("f64")),
};
const SETTINGS: Spec = {
  capture_moves: optional("bool"),
  capture_keys: optional("bool"),
  countdown: optional("bool"),
  esc_stops_recording: optional("bool"),
  ignore_injected: optional("bool"),
  capture_screen: optional("bool"),
  path_mode: optional(oneOf("full", "trail")),
  show_click_labels: optional("bool"),
  preview_background: optional(oneOf("screen", "sketch")),
  close_to_tray: optional("bool"),
  keep_on_top: optional(oneOf("always", "sessions", "never")),
};

const TRIGGERS: Spec = {
  hotkey: optional({ enabled: optional("bool"), combo: optional("string") }),
  schedule: optional({ enabled: optional("bool"), schedule: optional({ days: "days", time: "time" }) }),
  app_launch: optional({ enabled: optional("bool"), exe: optional("string"), delay_ms: optional("u32") }),
  pixel: optional({ enabled: optional("bool"), x: optional("i32"), y: optional("i32"), color: optional("color"), tolerance: optional("u8") }),
  image: optional({ enabled: optional("bool"), image: optional(nullable("png")), threshold: optional("u8"), area: optional(nullable(RECT)) }),
};

export const core = new FakeCore();
