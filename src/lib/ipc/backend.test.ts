// The UI's contract with Rust: each Backend method sends one command, with
// the argument names the #[tauri::command] functions take (camelCase in JS,
// matched to snake_case parameters by Tauri).
import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";
import { browserBackend, tauriBackend } from "./backend";
import { core } from "../../test/fake-core";
import type { EngineMsg } from "./bindings/EngineMsg";
import type { MacroTriggers, PlaybackOptions, Settings } from "../types";
import { DEFAULT_PLAYBACK, DEFAULT_SETTINGS } from "../defaults";

const b = tauriBackend;
const id = "00000000-0000-0000-0000-000000000001";

beforeEach(() => core.reset());

describe("tauriBackend", () => {
  test("is editable", () => {
    expect(b.editable).toBe(true);
  });

  test.each([
    ["toggleRecord", () => b.toggleRecord(), "toggle_record", {}],
    ["togglePlay", () => b.togglePlay(1234), "toggle_play", { from: 1234 }],
    ["stop", () => b.stop(), "stop_session", {}],
    ["seek", () => b.seek(50.5), "seek", { t: 50.5 }],
    ["listMacros", () => b.listMacros(), "list_macros", {}],
    ["loadMacro", () => b.loadMacro(id), "load_macro", { id }],
    ["editMacro", () => b.editMacro(id, { op: "rename", name: "N" }), "edit_macro", { id, op: { op: "rename", name: "N" } }],
    ["undoEdit", () => b.undoEdit(id, false), "undo_edit", { id, redo: false }],
    ["redo", () => b.undoEdit(id, true), "undo_edit", { id, redo: true }],
    ["duplicateMacro", () => b.duplicateMacro(id), "duplicate_macro", { id }],
    ["deleteMacro", () => b.deleteMacro(id), "delete_macro", { id }],
    ["getSettings", () => b.getSettings(), "get_settings", {}],
    ["samplePixel", () => b.samplePixel(-5, 7), "sample_pixel", { x: -5, y: 7 }],
    ["pickPixel", () => b.pickPixel(3000), "pick_pixel", { delayMs: 3000 }],
    ["getTriggers", () => b.getTriggers(id), "get_triggers", { id }],
    ["setTriggersPaused", () => b.setTriggersPaused(true), "set_triggers_paused", { paused: true }],
    ["listProcesses", () => b.listProcesses(), "list_processes", {}],
    ["getAutostart", () => b.getAutostart(), "get_autostart", {}],
    ["setAutostart", () => b.setAutostart(true), "set_autostart", { enabled: true }],
  ] as const)("%s sends %s", async (_name, call, cmd, args) => {
    await call();
    expect(core.calls).toEqual([{ cmd, args }]);
  });

  test("restoreMacro sends restore_macro", async () => {
    await b.deleteMacro(id);
    core.clearCalls();
    await b.restoreMacro(id);
    expect(core.calls).toEqual([{ cmd: "restore_macro", args: { id } }]);
  });

  test("setPlaybackOptions sends the whole options object", async () => {
    const options: PlaybackOptions = { ...DEFAULT_PLAYBACK, speed: 2, repeat: "forever" };
    const view = await b.setPlaybackOptions(id, options);
    expect(core.calls).toEqual([{ cmd: "set_playback_options", args: { id, options } }]);
    expect(view.playback).toEqual(options);
  });

  test("updateSettings sends the whole settings object", async () => {
    const settings: Settings = { ...DEFAULT_SETTINGS, countdown: false, keep_on_top: "never" };
    expect(await b.updateSettings(settings)).toEqual(settings);
    expect(core.calls).toEqual([{ cmd: "update_settings", args: { settings } }]);
  });

  test("setTriggers sends all four triggers", async () => {
    const triggers: MacroTriggers = {
      hotkey: { enabled: true, combo: "Ctrl + Alt + 9" },
      schedule: { enabled: true, schedule: { days: [true, false, true, false, true, false, false], time: "07:30" } },
      app_launch: { enabled: true, exe: "excel.exe", delay_ms: 1500 },
      pixel: { enabled: true, x: 10, y: 20, color: "#ABCDEF", tolerance: 4 },
    };
    const status = await b.setTriggers(id, triggers);
    expect(core.calls).toEqual([{ cmd: "set_triggers", args: { id, triggers } }]);
    expect(status.triggers).toEqual(triggers);
    expect(status.next_run).not.toBeNull();
  });

  test("errors arrive as { code, message }", async () => {
    await expect(b.loadMacro("nope")).rejects.toEqual({ code: "not_found", message: "no macro with id nope" });
    core.fail("delete_macro", "Stop the recording or playback first", "busy");
    await expect(b.deleteMacro(id)).rejects.toEqual({ code: "busy", message: "Stop the recording or playback first" });
  });

  test("subscribe hands Rust a channel whose messages reach the UI in order", async () => {
    const got: EngineMsg[] = [];
    await b.subscribe((m) => got.push(m));
    expect(core.calls).toEqual([{ cmd: "subscribe_engine", args: {} }]);
    core.emit({ type: "countdown", left_ms: 3000 });
    core.emit({ type: "session", mode: "recording", macro_id: null });
    expect(got).toEqual([
      { type: "countdown", left_ms: 3000 },
      { type: "session", mode: "recording", macro_id: null },
    ]);
  });

  describe("export", () => {
    test("asks where to save, with the format's filter, then writes there", async () => {
      core.dialog.save = "C:\\Users\\me\\export-invoice.rly";
      expect(await b.exportMacro(id, "rly", "export-invoice.rly")).toBe("C:\\Users\\me\\export-invoice.rly");
      expect(core.calls[0].cmd).toBe("plugin:dialog|save");
      expect(core.calls[0].args.options).toMatchObject({
        defaultPath: "export-invoice.rly",
        filters: [{ name: "Relay macro", extensions: ["rly"] }],
      });
      expect(core.calls[1]).toEqual({
        cmd: "export_macro",
        args: { id, format: "rly", path: "C:\\Users\\me\\export-invoice.rly" },
      });
    });

    test("JSON exports get the JSON filter", async () => {
      core.dialog.save = "C:\\x.json";
      await b.exportMacro(id, "json", "x.json");
      expect(core.calls[0].args.options).toMatchObject({ filters: [{ name: "JSON events", extensions: ["json"] }] });
      expect(core.lastArgs("export_macro")).toMatchObject({ format: "json" });
    });

    test("cancelling the dialog writes nothing", async () => {
      core.dialog.save = null;
      expect(await b.exportMacro(id, "rly", "a.rly")).toBeNull();
      expect(core.commands()).toEqual(["plugin:dialog|save"]);
    });
  });

  describe("import", () => {
    test("asks for several .rly or .json files, then imports them", async () => {
      core.dialog.open = ["C:\\a.rly", "C:\\b.json"];
      const result = await b.importMacros();
      expect(core.calls[0].cmd).toBe("plugin:dialog|open");
      expect(core.calls[0].args.options).toMatchObject({
        multiple: true,
        filters: [{ name: "Relay macros", extensions: ["rly", "json"] }],
      });
      expect(core.calls[1]).toEqual({ cmd: "import_macros", args: { paths: ["C:\\a.rly", "C:\\b.json"] } });
      expect(result?.imported).toHaveLength(2);
    });

    test("a single picked file is sent as a list", async () => {
      core.dialog.open = "C:\\one.rly";
      await b.importMacros();
      expect(core.lastArgs("import_macros")).toEqual({ paths: ["C:\\one.rly"] });
    });

    test("cancelling imports nothing", async () => {
      core.dialog.open = null;
      expect(await b.importMacros()).toBeNull();
      expect(core.commands()).toEqual(["plugin:dialog|open"]);
    });
  });
});

describe("browserBackend (npm run dev)", () => {
  let got: EngineMsg[];
  const modes = () => got.filter((m) => m.type === "session").map((m) => (m as { mode: string }).mode);

  beforeEach(async () => {
    vi.useFakeTimers();
    got = [];
  });
  afterEach(() => vi.useRealTimers());

  async function ready() {
    const bb = browserBackend();
    await bb.subscribe((m) => got.push(m));
    await vi.advanceTimersByTimeAsync(0);
    return bb;
  }

  test("is read-only and never calls Rust", async () => {
    const bb = await ready();
    expect(bb.editable).toBe(false);
    await bb.listMacros();
    expect(core.calls).toEqual([]);
  });

  test("lists the design's sample macros", async () => {
    const bb = await ready();
    const list = await bb.listMacros();
    expect(list.map((m) => m.name)).toEqual([
      "Export invoice to PDF",
      "Fill weekly timesheet",
      "Batch rename photos",
      "Open standup tools",
    ]);
    expect(list[0]).toMatchObject({ step_count: 12, runs: 148, hotkey: "Ctrl + Alt + 1" });
    expect(list[2].hotkey).toBeNull();
    expect(new Date(list[0].last_run!).getTime()).toBeLessThan(Date.now());
  });

  test("loads a macro, and rejects unknown ones", async () => {
    const bb = await ready();
    expect((await bb.loadMacro(id)).name).toBe("Export invoice to PDF");
    await expect(bb.loadMacro("missing")).rejects.toMatchObject({ code: "not_found" });
  });

  test("edits, files and pixel picking need the app", async () => {
    const bb = await ready();
    for (const p of [
      bb.editMacro(id, { op: "rename", name: "x" }),
      bb.undoEdit(id, false),
      bb.exportMacro(id, "rly", "x.rly"),
      bb.importMacros(),
      bb.duplicateMacro(id),
      bb.deleteMacro(id),
      bb.restoreMacro(id),
      bb.pickPixel(0),
    ]) {
      await expect(p).rejects.toMatchObject({ code: "unavailable" });
    }
    expect(await bb.samplePixel(1, 2)).toBeNull();
    expect(await bb.getAutostart()).toBe(false);
    expect(await bb.setAutostart(true)).toBe(false);
    expect(await bb.listProcesses()).toContain("excel.exe");
  });

  test("recording counts down three seconds, records, and F9 again stops", async () => {
    const bb = await ready();
    await bb.toggleRecord();
    expect(modes()).toEqual(["countdown"]);
    await vi.advanceTimersByTimeAsync(1000);
    const left = got.filter((m) => m.type === "countdown").map((m) => (m as { left_ms: number }).left_ms);
    expect(left.at(-1)).toBeGreaterThan(1900);
    expect(left.at(-1)).toBeLessThanOrEqual(2000);
    await vi.advanceTimersByTimeAsync(2100);
    expect(modes()).toEqual(["countdown", "recording"]);
    await vi.advanceTimersByTimeAsync(500);
    const progress = got.filter((m) => m.type === "rec_progress");
    expect(progress.length).toBeGreaterThanOrEqual(4);
    expect(progress.at(-1)).toMatchObject({ moves: [], steps: null, desktop: { w: 1920, h: 1080 } });
    await bb.toggleRecord();
    expect(modes()).toEqual(["countdown", "recording", "idle"]);
  });

  test("F9 during the countdown cancels it", async () => {
    const bb = await ready();
    await bb.toggleRecord();
    await vi.advanceTimersByTimeAsync(1000);
    await bb.toggleRecord();
    await vi.advanceTimersByTimeAsync(5000);
    expect(modes()).toEqual(["countdown", "idle"]);
  });

  test("without the countdown, recording starts at once", async () => {
    const bb = await ready();
    await bb.updateSettings({ ...DEFAULT_SETTINGS, countdown: false });
    await bb.toggleRecord();
    expect(modes()).toEqual(["recording"]);
    await bb.stop();
    expect(modes()).toEqual(["recording", "idle"]);
    expect(got.some((m) => m.type === "finished")).toBe(false);
  });

  test("playback runs every loop to the end, then finishes", async () => {
    const bb = await ready();
    const view = await bb.loadMacro(id); // 3 loops at 1×
    await bb.togglePlay(0);
    expect(modes()).toEqual(["playing"]);
    await vi.advanceTimersByTimeAsync(view.duration * 3 + 500);
    const ticks = got.filter((m) => m.type === "play_tick") as Extract<EngineMsg, { type: "play_tick" }>[];
    expect(new Set(ticks.map((t) => t.loop_idx))).toEqual(new Set([0, 1, 2]));
    expect(ticks.every((t) => t.loops === 3 && t.speed === 1)).toBe(true);
    expect(ticks.at(-1)).toMatchObject({ t: view.duration, advancing: false, loop_idx: 2 });
    expect(got.at(-2)).toEqual({ type: "finished", reason: "completed", timing: null });
    expect(modes()).toEqual(["playing", "idle"]);
  });

  test("F9 is ignored while playing", async () => {
    const bb = await ready();
    await bb.loadMacro(id);
    await bb.togglePlay(0);
    await bb.toggleRecord();
    expect(modes()).toEqual(["playing"]);
    await bb.stop();
  });

  test("pause holds the playhead and resume continues from it", async () => {
    const bb = await ready();
    await bb.loadMacro(id);
    await bb.togglePlay(0);
    await vi.advanceTimersByTimeAsync(1000);
    await bb.togglePlay(0);
    expect(modes()).toEqual(["playing", "paused"]);
    const lastTick = () => got.filter((m) => m.type === "play_tick").at(-1) as Extract<EngineMsg, { type: "play_tick" }>;
    await vi.advanceTimersByTimeAsync(100);
    const held = lastTick().t;
    await vi.advanceTimersByTimeAsync(1000);
    expect(lastTick().t).toBe(held);
    expect(lastTick().advancing).toBe(false);
    await bb.togglePlay(0);
    await vi.advanceTimersByTimeAsync(500);
    expect(lastTick().t).toBeGreaterThan(held + 400);
    expect(modes()).toEqual(["playing", "paused", "playing"]);
    await bb.stop();
  });

  test("stop ends playback as stopped", async () => {
    const bb = await ready();
    await bb.loadMacro(id);
    await bb.togglePlay(500);
    await vi.advanceTimersByTimeAsync(100);
    await bb.stop();
    expect(got.at(-2)).toEqual({ type: "finished", reason: "stopped", timing: null });
    expect(modes()).toEqual(["playing", "idle"]);
    const n = got.length;
    await bb.stop();
    expect(got.length).toBe(n); // already idle: nothing
  });

  test("playing from the end starts over; with nothing loaded, play does nothing", async () => {
    const bb = await ready();
    await bb.togglePlay(0);
    expect(modes()).toEqual([]);
    const view = await bb.loadMacro(id);
    await bb.togglePlay(view.duration);
    await vi.advanceTimersByTimeAsync(100);
    const t = (got.filter((m) => m.type === "play_tick").at(-1) as { t: number }).t;
    expect(t).toBeLessThan(500);
    await bb.stop();
  });

  test("seek moves the simulated playhead", async () => {
    const bb = await ready();
    await bb.loadMacro(id);
    await bb.togglePlay(0);
    await bb.seek(5000);
    await vi.advanceTimersByTimeAsync(100);
    const t = (got.filter((m) => m.type === "play_tick").at(-1) as { t: number }).t;
    expect(t).toBeGreaterThanOrEqual(5000);
    expect(t).toBeLessThan(5300);
    await bb.stop();
  });

  test("speed changes apply mid-playback without a jump", async () => {
    const bb = await ready();
    const view = await bb.loadMacro(id);
    await bb.togglePlay(0);
    await vi.advanceTimersByTimeAsync(1000);
    const before = (got.filter((m) => m.type === "play_tick").at(-1) as { t: number }).t;
    const saved = await bb.setPlaybackOptions(id, { ...view.playback, speed: 4 });
    expect(saved.playback.speed).toBe(4);
    await vi.advanceTimersByTimeAsync(100);
    const after = got.filter((m) => m.type === "play_tick").at(-1) as { t: number; speed: number };
    expect(after.speed).toBe(4);
    expect(after.t - before).toBeGreaterThan(200);
    expect(after.t - before).toBeLessThan(700);
    await bb.stop();
  });

  test("forever loops never finish on their own", async () => {
    const bb = await ready();
    const view = await bb.loadMacro(id);
    await bb.setPlaybackOptions(id, { ...view.playback, repeat: "forever" });
    await bb.togglePlay(0);
    await vi.advanceTimersByTimeAsync(view.duration * 5);
    expect(got.some((m) => m.type === "finished")).toBe(false);
    const last = got.filter((m) => m.type === "play_tick").at(-1) as { loops: number | null; loop_idx: number };
    expect(last.loops).toBeNull();
    expect(last.loop_idx).toBeGreaterThanOrEqual(4);
    await bb.stop();
  });

  test("settings and triggers live in memory", async () => {
    const bb = await ready();
    expect(await bb.getSettings()).toEqual(DEFAULT_SETTINGS);
    await bb.updateSettings({ ...DEFAULT_SETTINGS, path_mode: "trail" });
    expect((await bb.getSettings()).path_mode).toBe("trail");

    const status = await bb.getTriggers(id);
    expect(status.triggers.hotkey).toEqual({ enabled: false, combo: "Ctrl + Alt + 1" });
    expect(status.triggers.schedule.schedule.time).toBe("09:00");
    const t = { ...status.triggers, pixel: { ...status.triggers.pixel, enabled: true } };
    await bb.setTriggers(id, t);
    expect((await bb.getTriggers(id)).triggers.pixel.enabled).toBe(true);
    await bb.setTriggersPaused(true);
    expect((await bb.getTriggers(id)).paused).toBe(true);
  });
});
