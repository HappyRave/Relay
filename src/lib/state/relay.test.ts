// The store: what each action sends to Rust, how engine messages change the
// UI state, and the guards (busy sessions, stale responses, failed saves).
import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";
import { core, freshStore, nextFrame, settle } from "../../test/app";
import { browserBackend } from "../ipc/backend";
import type { RelayStore } from "./relay.svelte";
import type { EngineMsg } from "../ipc/bindings/EngineMsg";

const [A, B, C, D] = [1, 2, 3, 4].map((n) => `00000000-0000-0000-0000-00000000000${n}`);
let relay: RelayStore;

const session = (mode: "idle" | "countdown" | "recording" | "playing" | "paused", macro_id: string | null = A): EngineMsg => ({
  type: "session",
  mode,
  macro_id,
});
const key = (k: string, init: KeyboardEventInit = {}, target: EventTarget = window) =>
  target.dispatchEvent(new KeyboardEvent("keydown", { key: k, bubbles: true, cancelable: true, ...init }));

beforeEach(async () => {
  relay = await freshStore();
});
afterEach(() => relay.dispose());

describe("startup", () => {
  test("subscribes, then loads settings, the library, the first macro and its triggers", async () => {
    const r = await freshStore({ init: false });
    await r.init();
    await settle();
    expect(core.commands()).toEqual([
      "window_prefs",
      "subscribe_engine",
      "get_settings",
      "list_macros",
      "load_macro",
      "get_triggers",
      "get_autostart",
    ]);
    expect(core.lastArgs("load_macro")).toEqual({ id: A });
    expect(r.ready).toBe(true);
    expect(r.name).toBe("Export invoice to PDF");
    expect(r.tab).toBe("steps");
  });

  test("opens in the mode the widget was left in", async () => {
    core.reset();
    core.window.expanded = false;
    const r = await freshStore({ init: false });
    core.window.expanded = false;
    await r.init();
    expect(r.expanded).toBe(false);
  });

  test("a failure to read settings is shown, and the rest still loads", async () => {
    const r = await freshStore({ init: false });
    core.fail("get_settings", "settings.json is locked");
    await r.init();
    expect(r.error).toBe("settings.json is locked");
    expect(r.library).toHaveLength(4);
    expect(r.view?.id).toBe(A);
  });

  test("an empty library leaves nothing open", async () => {
    const r = await freshStore({ init: false });
    core.entries = [];
    await r.init();
    expect(r.view).toBeNull();
    expect(r.name).toBe("");
    expect(r.steps).toEqual([]);
    expect(r.duration).toBe(2000);
  });

  test("autostart is read, and a failure to read it counts as off", async () => {
    const r = await freshStore({ init: false });
    core.autostart = true;
    await r.init();
    expect(r.autostart).toBe(true);
    const r2 = await freshStore({ init: false });
    core.fail("get_autostart");
    await r2.init();
    expect(r2.autostart).toBe(false);
    expect(r2.toast).toBeNull();
  });
});

describe("session buttons", () => {
  test("Record sends toggle_record", async () => {
    await relay.toggleRec();
    expect(core.commands()).toEqual(["toggle_record"]);
  });

  test("Record does nothing while playing", async () => {
    core.emit(session("playing"));
    await relay.toggleRec();
    core.emit(session("paused"));
    await relay.toggleRec();
    expect(core.commands()).toEqual([]);
  });

  test("Play sends toggle_play from the playhead", async () => {
    relay.cur = 1234;
    await relay.togglePlay();
    expect(core.argsOf("toggle_play")).toEqual([{ from: 1234 }]);
  });

  test("Play at the end starts from the beginning", async () => {
    relay.cur = relay.duration;
    await relay.togglePlay();
    relay.cur = relay.duration - 0.5;
    await relay.togglePlay();
    expect(core.argsOf("toggle_play")).toEqual([{ from: 0 }, { from: 0 }]);
  });

  test("Play does nothing while recording or with no macro", async () => {
    core.emit(session("countdown"));
    await relay.togglePlay();
    core.emit(session("recording"));
    await relay.togglePlay();
    core.emit(session("idle"));
    relay.view = null;
    await relay.togglePlay();
    expect(core.argsOf("toggle_play")).toEqual([]);
  });

  test("Stop sends stop_session in any mode", async () => {
    await relay.stop();
    core.emit(session("playing"));
    await relay.stop();
    expect(core.commands()).toEqual(["stop_session", "stop_session"]);
  });

  test("a failed command is shown", async () => {
    core.fail("toggle_record", "Couldn't install the input hook");
    await relay.toggleRec();
    expect(relay.toast).toMatchObject({ kind: "error", message: "Couldn't install the input hook" });
  });
});

describe("seeking", () => {
  test("moves the playhead at once and tells the engine once per frame", async () => {
    relay.seek(1000);
    relay.seek(2000);
    relay.seek(3000);
    expect(relay.cur).toBe(3000);
    expect(core.argsOf("seek")).toEqual([]);
    await nextFrame();
    expect(core.argsOf("seek")).toEqual([{ t: 3000 }]);
    relay.seek(4000);
    await nextFrame();
    expect(core.argsOf("seek")).toEqual([{ t: 3000 }, { t: 4000 }]);
  });

  test("is kept within the macro", async () => {
    relay.seek(-50);
    expect(relay.cur).toBe(0);
    relay.seek(1e9);
    expect(relay.cur).toBe(relay.duration);
  });

  test("is ignored while recording", async () => {
    core.emit(session("recording"));
    relay.seek(500);
    await nextFrame();
    expect(relay.cur).toBe(0);
    expect(core.argsOf("seek")).toEqual([]);
  });

  test("previous and next step jump between step starts", async () => {
    relay.seek(0);
    relay.jump(1);
    expect(relay.cur).toBe(850);
    relay.jump(1);
    expect(relay.cur).toBe(1750);
    relay.jump(-1);
    expect(relay.cur).toBe(850);
    relay.jump(-1);
    expect(relay.cur).toBe(0);
    relay.seek(9650);
    relay.jump(1);
    expect(relay.cur).toBe(relay.duration);
  });

  test("the current step follows the playhead", () => {
    relay.seek(0);
    expect(relay.curStepIdx).toBe(-1);
    relay.seek(850);
    expect(relay.curStepIdx).toBe(0);
    relay.seek(2500);
    expect(relay.curStepIdx).toBe(2);
    relay.seek(relay.duration);
    expect(relay.curStepIdx).toBe(11);
  });

  test("the cursor is where the path says, or mid-desktop without one", () => {
    expect(relay.cursorAt(0)).toEqual({ t: 0, x: 960, y: 670 });
    expect(relay.cursorAt(850)).toMatchObject({ x: 134, y: 70 });
    relay.view = { ...relay.view!, moves: [] };
    expect(relay.cursorAt(500)).toEqual({ x: 960, y: 540 });
  });
});

describe("engine messages", () => {
  test("the countdown shows the time left", () => {
    relay.cur = 4000;
    core.emit(session("countdown"));
    core.emit({ type: "countdown", left_ms: 2200 });
    expect(relay.mode).toBe("countdown");
    expect(relay.recording).toBe(true);
    expect(relay.countLeft).toBe(2200);
    expect(relay.cur).toBe(0);
  });

  test("recording shows live steps, path and desktop, then the saved macro opens", async () => {
    relay.start();
    core.emit(session("recording", null));
    expect(relay.steps).toEqual([]);
    expect(relay.moves).toEqual([]);
    expect(relay.frames).toEqual([]);
    const desktop = { x: -1920, y: 0, w: 3840, h: 1080 };
    const click = relay.view!.steps[0];
    core.emit({ type: "rec_progress", elapsed_ms: 100, desktop, moves: [{ t: 50, x: 1, y: 2 }], steps: null });
    core.emit({ type: "rec_progress", elapsed_ms: 200, desktop, moves: [{ t: 150, x: 3, y: 4 }], steps: [click] });
    core.emit({ type: "rec_progress", elapsed_ms: 3300, desktop, moves: [], steps: null });
    expect(relay.moves).toEqual([
      { t: 50, x: 1, y: 2 },
      { t: 150, x: 3, y: 4 },
    ]);
    expect(relay.steps).toEqual([click]);
    expect(relay.desktop).toEqual(desktop);
    await nextFrame();
    expect(relay.cur).toBeGreaterThanOrEqual(3300);
    expect(relay.cur).toBeLessThanOrEqual(3400);
    expect(relay.duration).toBe(4000); // whole seconds while recording

    core.emit({ type: "saved", id: C });
    await settle();
    expect(core.argsOf("load_macro")).toEqual([]); // not before the session is idle
    core.emit(session("idle", null));
    await settle();
    expect(core.argsOf("load_macro")).toEqual([{ id: C }]);
    expect(relay.name).toBe("Batch rename photos");
    expect(relay.steps).toHaveLength(8);
  });

  test("a save arriving when already idle opens the macro at once", async () => {
    core.emit({ type: "saved", id: B });
    await settle();
    expect(relay.view?.id).toBe(B);
  });

  test("play ticks move the playhead, extrapolated between ticks", async () => {
    relay.start();
    core.emit(session("playing"));
    core.emit({ type: "play_tick", t: 2000, advancing: true, speed: 2, loop_idx: 1, loops: 3 });
    expect(relay.loopIdx).toBe(1);
    expect(relay.playing).toBe(true);
    await nextFrame();
    expect(relay.cur).toBeGreaterThan(2000);
    expect(relay.cur).toBeLessThanOrEqual(2200); // at most 100 ms ahead, at 2×
  });

  test("a tick that isn't advancing (paused, a pixel wait) sets the playhead exactly", async () => {
    relay.start();
    core.emit(session("paused"));
    core.emit({ type: "play_tick", t: 3456, advancing: false, speed: 1, loop_idx: 0, loops: 1 });
    await nextFrame();
    expect(relay.cur).toBe(3456);
  });

  test("a playback started by a trigger opens its macro", async () => {
    core.emit(session("playing", D));
    await settle();
    expect(core.argsOf("load_macro")).toEqual([{ id: D }]);
    expect(relay.view?.id).toBe(D);
  });

  test.each([
    ["stopped", 0],
    ["key_pressed", 0],
    ["killed", 0],
    ["error", 0],
    ["completed", 5000],
    ["pixel_timeout", 5000],
  ] as const)("finishing as %s leaves the playhead at %d", (reason, at) => {
    relay.cur = 5000;
    relay.loopIdx = 2;
    core.emit({ type: "finished", reason, timing: null });
    expect(relay.cur).toBe(at);
    expect(relay.lastFinish).toBe(reason);
    expect(relay.loopIdx).toBe(0);
  });

  test("the last playback's timing is kept", () => {
    const timing = { events: 120, p50_ms: 0.2, p99_ms: 1.1, max_ms: 2 };
    core.emit({ type: "finished", reason: "completed", timing });
    core.emit({ type: "finished", reason: "stopped", timing: null });
    expect(relay.lastTiming).toEqual(timing);
  });

  test("library changes reload the list", async () => {
    core.entries[1].runs = 999;
    core.emit({ type: "library_changed" });
    await settle();
    expect(core.commands()).toEqual(["list_macros"]);
    expect(relay.library[1].runs).toBe(999);
  });

  test("the kill switch pauses triggers, and resuming shows it", () => {
    core.emit({ type: "triggers_paused", paused: true });
    expect(relay.triggersPaused).toBe(true);
    core.emit({ type: "triggers_paused", paused: false });
    expect(relay.triggersPaused).toBe(false);
  });

  test("Ctrl + Shift + M switches between compact and expanded", () => {
    core.emit({ type: "toggle_compact" });
    expect(relay.expanded).toBe(false);
    core.emit({ type: "toggle_compact" });
    expect(relay.expanded).toBe(true);
  });

  test("errors and notices become toasts", () => {
    core.emit({ type: "error", message: "Couldn't register the Play hotkey" });
    expect(relay.toast).toEqual({ kind: "error", message: "Couldn't register the Play hotkey" });
    expect(relay.error).toBe("Couldn't register the Play hotkey");
    core.emit({ type: "notice", message: "Skipped “Export”: Relay was busy" });
    expect(relay.toast).toMatchObject({ kind: "info", message: "Skipped “Export”: Relay was busy" });
    expect(relay.error).toBeNull();
  });
});

describe("toasts", () => {
  beforeEach(() => vi.useFakeTimers());

  test("errors go away after 5 s, notices from the engine after 6 s", async () => {
    core.emit({ type: "error", message: "x" });
    await vi.advanceTimersByTimeAsync(4900);
    expect(relay.toast).not.toBeNull();
    await vi.advanceTimersByTimeAsync(200);
    expect(relay.toast).toBeNull();
    core.emit({ type: "notice", message: "y" });
    await vi.advanceTimersByTimeAsync(5900);
    expect(relay.toast).not.toBeNull();
    await vi.advanceTimersByTimeAsync(200);
    expect(relay.toast).toBeNull();
  });

  test("a toast with an Undo stays at least 8 s", async () => {
    relay.notify("Moved it", { label: "Undo", run: () => {} });
    await vi.advanceTimersByTimeAsync(7900);
    expect(relay.toast?.action?.label).toBe("Undo");
    await vi.advanceTimersByTimeAsync(200);
    expect(relay.toast).toBeNull();
  });

  test("a newer toast replaces the older one and restarts the clock", async () => {
    relay.notify("one");
    await vi.advanceTimersByTimeAsync(4000);
    relay.notify("two");
    await vi.advanceTimersByTimeAsync(4000);
    expect(relay.toast?.message).toBe("two");
  });

  test("dismissing closes it", () => {
    relay.notify("one");
    relay.dismissToast();
    expect(relay.toast).toBeNull();
  });
});

describe("library", () => {
  test("opening a macro loads it and its triggers, on the Steps tab", async () => {
    relay.tab = "library";
    relay.cur = 3000;
    await relay.loadMacro(B);
    expect(core.calls).toEqual([
      { cmd: "load_macro", args: { id: B } },
      { cmd: "get_triggers", args: { id: B } },
    ]);
    expect(relay.view?.id).toBe(B);
    expect(relay.triggers?.hotkey.combo).toBe("Ctrl + Alt + 2");
    expect(relay.tab).toBe("steps");
    expect(relay.cur).toBe(0);
  });

  test("macros can't be switched during a session", async () => {
    core.emit(session("recording"));
    await relay.loadMacro(B);
    core.emit(session("playing"));
    await relay.loadMacro(B);
    expect(core.argsOf("load_macro")).toEqual([]);
    expect(relay.view?.id).toBe(A);
  });

  test("a slow response for a macro the user left is dropped", async () => {
    core.hold("load_macro");
    const first = relay.loadMacro(B);
    await settle();
    core.release("load_macro");
    await relay.loadMacro(C);
    core.held[0].resolve(core.view(B));
    await first;
    await settle();
    expect(relay.view?.id).toBe(C);
  });

  test("the old macro's triggers are never shown for the new one", async () => {
    core.hold("get_triggers");
    const opening = relay.loadMacro(B);
    await settle();
    expect(relay.view?.id).toBe(B);
    expect(relay.triggerStatus).toBeNull();
    core.release("get_triggers");
    core.held[0].resolve({ triggers: core.triggers.get(B), next_run: null, hotkey_error: null, paused: true });
    await opening;
    expect(relay.triggers?.hotkey.combo).toBe("Ctrl + Alt + 2");
    expect(relay.triggersPaused).toBe(true);
  });

  test("Duplicate opens the copy in the Library tab", async () => {
    await relay.duplicateMacro(A);
    expect(core.commands()).toEqual(["duplicate_macro", "list_macros", "load_macro", "get_triggers"]);
    expect(relay.library).toHaveLength(5);
    expect(relay.name).toBe("Export invoice to PDF (copy)");
    expect(relay.tab).toBe("library");
  });

  test("a failed duplicate changes nothing", async () => {
    core.fail("duplicate_macro", "disk full");
    await relay.duplicateMacro(A);
    expect(core.commands()).toEqual(["duplicate_macro"]);
    expect(relay.error).toBe("disk full");
  });

  test("Delete moves the open macro to the trash, opens its neighbour and offers Undo", async () => {
    await relay.deleteMacro(A);
    expect(core.commands()).toEqual(["delete_macro", "list_macros", "load_macro", "get_triggers"]);
    expect(core.argsOf("delete_macro")).toEqual([{ id: A }]);
    expect(relay.view?.id).toBe(B);
    expect(relay.tab).toBe("library");
    expect(relay.toast).toMatchObject({ kind: "info", message: "Moved “Export invoice to PDF” to the trash" });

    core.clearCalls();
    relay.toast!.action!.run();
    await settle();
    expect(core.commands()).toEqual(["restore_macro", "list_macros", "load_macro", "get_triggers"]);
    expect(core.argsOf("restore_macro")).toEqual([{ id: A }]);
    expect(relay.view?.id).toBe(A);
    expect(relay.toast).toBeNull();
    expect(relay.library.map((m) => m.id)).toContain(A);
  });

  test("deleting another macro keeps the open one", async () => {
    await relay.deleteMacro(C);
    expect(core.commands()).toEqual(["delete_macro", "list_macros"]);
    expect(relay.view?.id).toBe(A);
    expect(relay.toast?.message).toBe("Moved “Batch rename photos” to the trash");
  });

  test("deleting the last macro in the list opens the one before it", async () => {
    await relay.loadMacro(D);
    await relay.deleteMacro(D);
    expect(relay.view?.id).toBe(C);
  });

  test("deleting the only macro leaves nothing open", async () => {
    for (const id of [B, C, D]) await relay.deleteMacro(id);
    await relay.deleteMacro(A);
    expect(relay.library).toEqual([]);
    expect(relay.view).toBeNull();
  });

  test("a refused delete (a session is running) is explained and nothing moves", async () => {
    core.fail("delete_macro", "Stop the recording or playback first", "busy");
    await relay.deleteMacro(A);
    expect(core.commands()).toEqual(["delete_macro"]);
    expect(relay.toast).toMatchObject({ kind: "error", message: "Stop the recording or playback first" });
    expect(relay.view?.id).toBe(A);
  });

  test("a failed restore is shown", async () => {
    await relay.deleteMacro(A);
    core.fail("restore_macro", "no macro with id x", "not_found");
    await relay.restoreMacro(A);
    expect(relay.error).toBe("no macro with id x");
  });

  describe("import", () => {
    test("cancelling the file picker does nothing", async () => {
      await relay.importMacros();
      expect(core.commands()).toEqual(["plugin:dialog|open"]);
      expect(relay.toast).toBeNull();
    });

    test("opens the first imported macro and says how many", async () => {
      core.dialog.open = ["C:\\one.rly", "C:\\two.rly"];
      await relay.importMacros();
      expect(core.commands()).toEqual(["plugin:dialog|open", "import_macros", "list_macros", "load_macro", "get_triggers"]);
      expect(relay.library).toHaveLength(6);
      expect(relay.name).toBe("one");
      expect(relay.tab).toBe("library");
      expect(relay.toast).toMatchObject({ kind: "info", message: "Imported 2 macros" });
    });

    test("files that couldn't be imported are listed", async () => {
      core.dialog.open = ["C:\\a.rly", "C:\\bad.rly"];
      core.importResult = { imported: [A], problems: ["bad.rly: expected value at line 1"] };
      await relay.importMacros();
      expect(relay.toast).toMatchObject({ kind: "error", message: "Imported 1 macro. bad.rly: expected value at line 1" });
    });

    test("nothing imported says so", async () => {
      core.dialog.open = ["C:\\bad.rly"];
      core.importResult = { imported: [], problems: [] };
      await relay.importMacros();
      expect(relay.toast?.message).toBe("Nothing imported");
      expect(core.argsOf("load_macro")).toEqual([]);
    });
  });
});

describe("step edits", () => {
  test("each edit sends its op for the open macro and shows the result", async () => {
    await relay.edit({ op: "set_label", index: 0, label: "File" });
    expect(core.calls).toEqual([{ cmd: "edit_macro", args: { id: A, op: { op: "set_label", index: 0, label: "File" } } }]);
    expect((relay.steps[0] as { label: string }).label).toBe("File");
    expect(relay.canUndo).toBe(true);
  });

  test("edits that change the step count refresh the library", async () => {
    await relay.edit({ op: "delete_step", index: 0 });
    expect(core.commands()).toEqual(["edit_macro", "list_macros"]);
    expect(relay.library[0].step_count).toBe(11);
  });

  test("a rejected edit is explained and the view is kept", async () => {
    const before = relay.view;
    await relay.edit({ op: "delete_step", index: 99 });
    expect(relay.error).toBe("no step 99");
    expect(relay.view).toBe(before);
  });

  test("editing is off while recording", async () => {
    core.emit(session("recording"));
    await relay.edit({ op: "delete_step", index: 0 });
    expect(core.commands()).toEqual([]);
  });

  test("an edit response for a macro the user left is dropped", async () => {
    core.hold("edit_macro");
    const editing = relay.edit({ op: "delete_step", index: 0 });
    await settle();
    core.release("edit_macro");
    await relay.loadMacro(B);
    core.held[0].resolve({ ...core.view(A), steps: [] });
    await editing;
    expect(relay.view?.id).toBe(B);
    expect(relay.steps).toHaveLength(13);
  });

  test("of two quick edits, only the last response is shown", async () => {
    core.hold("edit_macro");
    const one = relay.edit({ op: "set_label", index: 0, label: "one" });
    const two = relay.edit({ op: "set_label", index: 0, label: "two" });
    await settle();
    const [h1, h2] = core.held;
    h2.resolve({ ...core.view(A), name: "second" });
    h1.resolve({ ...core.view(A), name: "first" });
    await Promise.all([one, two]);
    expect(relay.name).toBe("second");
  });

  test("an edit dismisses an Undo offered for an earlier change", async () => {
    await relay.deleteStep(0);
    expect(relay.toast?.action).toBeDefined();
    await relay.edit({ op: "set_label", index: 0, label: "x" });
    expect(relay.toast).toBeNull();
  });

  test("Delete step offers Undo, which undoes it", async () => {
    await relay.deleteStep(2);
    expect(core.argsOf("edit_macro")).toEqual([{ id: A, op: { op: "delete_step", index: 2 } }]);
    expect(relay.toast).toMatchObject({ kind: "info", message: "Deleted the step" });
    core.clearCalls();
    relay.toast!.action!.run();
    await settle();
    expect(core.argsOf("undo_edit")).toEqual([{ id: A, redo: false }]);
    expect(relay.steps).toHaveLength(12);
  });

  test("a failed step delete offers no Undo", async () => {
    await relay.deleteStep(50);
    expect(relay.toast?.kind).toBe("error");
  });

  test("the pause before a step is sent in whole ms, never negative", async () => {
    await relay.setPause(3, 1234.6);
    await relay.setPause(3, -20);
    expect(core.argsOf("edit_macro").map((a) => a.op)).toEqual([
      { op: "set_pause", index: 3, dur: 1235 },
      { op: "set_pause", index: 3, dur: 0 },
    ]);
  });

  test("Trim pauses caps pauses at 1 s and offers Undo", async () => {
    expect(relay.longPauses).toBe(1);
    await relay.trimPauses();
    expect(core.argsOf("edit_macro")).toEqual([{ id: A, op: { op: "cap_pauses", max: 1000 } }]);
    expect(relay.longPauses).toBe(0);
    expect(relay.toast).toMatchObject({ message: "Shortened 1 pause to 1 s", action: { label: "Undo" } });
  });

  test("Trim pauses with nothing to trim sends nothing", async () => {
    await relay.loadMacro(C);
    core.clearCalls();
    expect(relay.longPauses).toBe(0);
    await relay.trimPauses();
    expect(core.commands()).toEqual([]);
  });

  test("+ Wait inserts half a second at the playhead", async () => {
    relay.seek(1500.4);
    await relay.insertWait();
    expect(core.argsOf("edit_macro")).toEqual([{ id: A, op: { op: "insert_wait", at: 1500, dur: 500, label: "Inserted" } }]);
    expect(relay.steps).toHaveLength(13);
  });

  test("+ Pixel check reads the pixel under the macro's cursor", async () => {
    relay.seek(850);
    core.pixel = "#ABCDEF";
    await relay.insertPixelCheck();
    expect(core.calls.slice(0, 2)).toEqual([
      { cmd: "sample_pixel", args: { x: 134, y: 70 } },
      {
        cmd: "edit_macro",
        args: {
          id: A,
          op: { op: "insert_pixel_wait", at: 850, dur: 800, x: 134, y: 70, color: "#ABCDEF", tolerance: 8, timeout_ms: 5000, label: "" },
        },
      },
    ]);
  });

  test("+ Pixel check falls back to the accent color when the screen can't be read", async () => {
    core.pixel = null;
    await relay.insertPixelCheck();
    expect((core.lastArgs("edit_macro")!.op as { color: string }).color).toBe("#EC3013");
    core.fail("sample_pixel");
    await relay.insertPixelCheck();
    expect((core.lastArgs("edit_macro")!.op as { color: string }).color).toBe("#EC3013");
  });
});

describe("rename", () => {
  beforeEach(() => vi.useFakeTimers());

  test("shows the name at once and saves it after a pause in typing", async () => {
    relay.rename("E");
    relay.rename("Ex");
    relay.rename("Export");
    expect(relay.name).toBe("Export");
    await vi.advanceTimersByTimeAsync(200);
    expect(core.argsOf("edit_macro")).toEqual([]);
    await vi.advanceTimersByTimeAsync(100);
    await settle();
    expect(core.argsOf("edit_macro")).toEqual([{ id: A, op: { op: "rename", name: "Export" } }]);
    expect(core.commands()).toContain("list_macros");
    expect(relay.library[0].name).toBe("Export");
  });

  test("a pending rename is saved before opening another macro", async () => {
    relay.rename("Renamed");
    await relay.loadMacro(B);
    expect(core.commands().slice(0, 3)).toEqual(["edit_macro", "list_macros", "load_macro"]);
    expect(core.view(A).name).toBe("Renamed");
    expect(relay.name).toBe("Fill weekly timesheet");
  });

  test("a pending rename is saved before undo", async () => {
    await relay.edit({ op: "set_label", index: 0, label: "x" });
    core.clearCalls();
    relay.rename("Typed");
    await relay.undo();
    expect(core.commands()[0]).toBe("edit_macro");
    expect(core.commands()).toContain("undo_edit");
  });

  test("a name still being typed wins over the one in an edit's response", async () => {
    relay.rename("Typing…");
    await relay.edit({ op: "set_label", index: 0, label: "x" });
    expect(relay.name).toBe("Typing…");
  });

  test("the browser preview renames only in memory", async () => {
    core.uninstall();
    try {
      const r = await freshStore({ backend: browserBackend() });
      r.rename("Local");
      await vi.advanceTimersByTimeAsync(1000);
      expect(r.name).toBe("Local");
      expect(core.calls).toEqual([]);
      r.dispose();
    } finally {
      core.install();
    }
  });

  test("with nothing open, rename does nothing", () => {
    relay.view = null;
    relay.rename("x");
    expect(relay.view).toBeNull();
  });
});

describe("undo and redo", () => {
  test("are off until there is something to undo or redo", async () => {
    expect(relay.canUndo).toBe(false);
    expect(relay.canRedo).toBe(false);
    await relay.undo();
    await relay.redo();
    expect(core.commands()).toEqual([]);
  });

  test("undo then redo", async () => {
    await relay.edit({ op: "delete_step", index: 0 });
    await relay.undo();
    expect(core.argsOf("undo_edit")).toEqual([{ id: A, redo: false }]);
    expect(relay.steps).toHaveLength(12);
    expect(relay.canRedo).toBe(true);
    expect(relay.canUndo).toBe(false);
    await relay.redo();
    expect(core.argsOf("undo_edit")).toEqual([
      { id: A, redo: false },
      { id: A, redo: true },
    ]);
    expect(relay.steps).toHaveLength(11);
  });

  test("are off during a session", async () => {
    await relay.edit({ op: "delete_step", index: 0 });
    core.emit(session("playing"));
    expect(relay.canUndo).toBe(false);
    await relay.undo();
    expect(core.argsOf("undo_edit")).toEqual([]);
  });

  test("undo dismisses the toast", async () => {
    await relay.deleteStep(0);
    await relay.undo();
    expect(relay.toast).toBeNull();
  });
});

describe("Pick (pixel under the real cursor)", () => {
  beforeEach(() => vi.useFakeTimers());
  const pixelStep = 7;

  test("counts down 3 s, then points the pixel check there", async () => {
    core.hold("pick_pixel");
    const picking = relay.pickPixel(pixelStep);
    await settle();
    expect(core.argsOf("pick_pixel")).toEqual([{ delayMs: 3000 }]);
    expect(relay.picking).toBe(3);
    await vi.advanceTimersByTimeAsync(1000);
    expect(relay.picking).toBe(2);
    await vi.advanceTimersByTimeAsync(1000);
    expect(relay.picking).toBe(1);
    await vi.advanceTimersByTimeAsync(1000);
    expect(relay.picking).toBe(1); // never shows 0 while waiting
    core.held[0].resolve({ x: 5, y: 6, color: "#0000FF" });
    await picking;
    expect(relay.picking).toBe(0);
    expect(core.lastArgs("edit_macro")).toEqual({
      id: A,
      op: { op: "update_pixel_wait", index: pixelStep, x: 5, y: 6, color: "#0000FF", tolerance: 8, timeout_ms: 5000 },
    });
  });

  test("a second Pick while one is running is ignored", async () => {
    core.hold("pick_pixel");
    const p = relay.pickPixel(pixelStep);
    await relay.pickPixel(pixelStep);
    await relay.pickTriggerPixel();
    expect(core.argsOf("pick_pixel")).toHaveLength(1);
    core.held[0].resolve(core.picked);
    await p;
  });

  test("the result is dropped if the user opened another macro meanwhile", async () => {
    core.hold("pick_pixel");
    const p = relay.pickPixel(pixelStep);
    await settle();
    await relay.loadMacro(B);
    core.held[0].resolve(core.picked);
    await p;
    expect(core.argsOf("edit_macro")).toEqual([]);
    expect(relay.picking).toBe(0);
  });

  test("if the step is no longer a pixel check, nothing is changed", async () => {
    await relay.pickPixel(0);
    expect(core.argsOf("edit_macro")).toEqual([]);
  });

  test("a failed pick is shown and the countdown ends", async () => {
    core.fail("pick_pixel", "The screen can't be read");
    await relay.pickPixel(pixelStep);
    expect(relay.picking).toBe(0);
    expect(relay.error).toBe("The screen can't be read");
  });

  test("the pixel trigger's Pick watches the picked pixel", async () => {
    await relay.pickTriggerPixel();
    expect(core.lastArgs("set_triggers")).toMatchObject({
      id: A,
      triggers: { pixel: { enabled: false, x: 640, y: 360, color: "#00FF00", tolerance: 8 } },
    });
  });
});

describe("playback options", () => {
  test("a change is shown at once and saved with the other options", async () => {
    const saving = relay.setPlayback({ speed: 2 });
    expect(relay.playback.speed).toBe(2);
    await saving;
    expect(core.calls).toEqual([
      {
        cmd: "set_playback_options",
        args: { id: A, options: { speed: 2, repeat: { count: 3 }, humanize: true, jitter_ms: 40, stop_on_key: true, coord_mode: "screen" } },
      },
    ]);
  });

  test("previewing (a slider being dragged) saves nothing", async () => {
    relay.previewPlayback({ jitter_ms: 120 });
    expect(relay.playback.jitter_ms).toBe(120);
    await settle();
    expect(core.commands()).toEqual([]);
  });

  test("repeat counts and forever", async () => {
    await relay.setPlayback({ repeat: "forever" });
    expect(relay.loops).toBe(Infinity);
    await relay.setPlayback({ repeat: { count: 7 } });
    expect(relay.loops).toBe(7);
  });

  test("with nothing open, nothing is sent", async () => {
    relay.view = null;
    await relay.setPlayback({ speed: 4 });
    relay.previewPlayback({ speed: 4 });
    expect(core.commands()).toEqual([]);
    expect(relay.playback.speed).toBe(1);
  });
});

describe("triggers", () => {
  test("a change is saved with the other triggers, then the list refreshes (for the hotkey column)", async () => {
    await relay.setTriggers({ hotkey: { enabled: true, combo: "Ctrl + Alt + 9" } });
    expect(core.commands()).toEqual(["set_triggers", "list_macros"]);
    expect(core.lastArgs("set_triggers")).toEqual({
      id: A,
      triggers: { ...core.triggers.get(A), hotkey: { enabled: true, combo: "Ctrl + Alt + 9" } },
    });
    expect(relay.library[0].hotkey).toBe("Ctrl + Alt + 9");
  });

  test("the next scheduled run comes back from Rust", async () => {
    await relay.setTriggers({ schedule: { ...relay.triggers!.schedule, enabled: true } });
    expect(relay.triggerStatus?.next_run).toBe("2026-09-25T09:00:00+02:00");
  });

  test("a refused hotkey puts the old triggers back and explains why", async () => {
    core.fail("set_triggers", "Ctrl + Alt + 2 already runs “Fill weekly timesheet”", "hotkey");
    const before = relay.triggerStatus;
    const saving = relay.setTriggers({ hotkey: { enabled: true, combo: "Ctrl + Alt + 2" } });
    expect(relay.triggers?.hotkey.combo).toBe("Ctrl + Alt + 2"); // optimistic
    await saving;
    expect(relay.triggerStatus).toBe(before);
    expect(relay.error).toBe("Ctrl + Alt + 2 already runs “Fill weekly timesheet”");
  });

  test("a response for a macro the user left is dropped", async () => {
    core.hold("set_triggers");
    const saving = relay.setTriggers({ pixel: { ...relay.triggers!.pixel, enabled: true } });
    await settle();
    core.release("set_triggers");
    await relay.loadMacro(B);
    core.held[0].resolve({ triggers: core.triggers.get(A), next_run: null, hotkey_error: null, paused: false });
    await saving;
    expect(relay.triggers?.hotkey.combo).toBe("Ctrl + Alt + 2");
  });

  test("with no triggers loaded, nothing is sent", async () => {
    relay.triggerStatus = null;
    await relay.setTriggers({ hotkey: { enabled: false, combo: "" } });
    expect(core.commands()).toEqual([]);
  });

  test("Resume unpauses triggers", async () => {
    relay.triggersPaused = true;
    await relay.setTriggersPaused(false);
    expect(relay.triggersPaused).toBe(false);
    expect(core.calls).toEqual([{ cmd: "set_triggers_paused", args: { paused: false } }]);
  });

  test("running programs are listed for suggestions; a failure keeps the old list", async () => {
    await relay.loadProcesses();
    expect(relay.processes).toEqual(["chrome.exe", "EXCEL.EXE", "notepad.exe"]);
    core.fail("list_processes");
    await relay.loadProcesses();
    expect(relay.processes).toHaveLength(3);
    expect(relay.toast).toBeNull();
  });
});

describe("settings", () => {
  test("a change is shown and saved with the other settings", async () => {
    await relay.updateSettings({ countdown: false });
    expect(core.calls).toEqual([{ cmd: "update_settings", args: { settings: { ...core.settings, countdown: false } } }]);
    expect(relay.settings.countdown).toBe(false);
  });

  test("a failed save puts the old settings back", async () => {
    core.fail("update_settings", "Couldn't save settings");
    await relay.updateSettings({ keep_on_top: "never" });
    expect(relay.settings.keep_on_top).toBe("always");
    expect(relay.error).toBe("Couldn't save settings");
  });

  test("Start with Windows reports what Windows actually has", async () => {
    await relay.setAutostart(true);
    expect(core.calls).toEqual([{ cmd: "set_autostart", args: { enabled: true } }]);
    expect(relay.autostart).toBe(true);
    core.on("set_autostart", () => false); // e.g. blocked by policy
    await relay.setAutostart(true);
    expect(relay.autostart).toBe(false);
    core.fail("set_autostart");
    await relay.setAutostart(true);
    expect(relay.autostart).toBe(false);
  });
});

describe("export", () => {
  test("the file name comes from the macro name and format", () => {
    expect(relay.exportName).toBe("export-invoice-to-pdf.rly");
    relay.exportFmt = "json";
    expect(relay.exportName).toBe("export-invoice-to-pdf.json");
    relay.view = { ...relay.view!, name: "  ***  " };
    expect(relay.exportName).toBe("macro.json");
  });

  test("Save closes the dialog and says where it went", async () => {
    relay.exportOpen = true;
    core.dialog.save = "C:\\Users\\me\\Desktop\\invoice.rly";
    await relay.doExport();
    expect(core.lastArgs("export_macro")).toEqual({ id: A, format: "rly", path: "C:\\Users\\me\\Desktop\\invoice.rly" });
    expect(relay.exportOpen).toBe(false);
    expect(relay.toast).toMatchObject({ kind: "info", message: "Saved invoice.rly" });
  });

  test("cancelling or failing keeps the dialog open", async () => {
    relay.exportOpen = true;
    await relay.doExport();
    expect(relay.exportOpen).toBe(true);
    core.dialog.save = "C:\\x.rly";
    core.fail("export_macro", "Couldn't save: access denied");
    await relay.doExport();
    expect(relay.exportOpen).toBe(true);
    expect(relay.error).toBe("Couldn't save: access denied");
  });
});

describe("keyboard", () => {
  beforeEach(() => relay.start());

  test("Ctrl + Z undoes, Ctrl + Y and Ctrl + Shift + Z redo", async () => {
    await relay.edit({ op: "delete_step", index: 0 });
    key("z", { ctrlKey: true });
    await settle();
    key("y", { ctrlKey: true });
    await settle();
    key("z", { ctrlKey: true });
    await settle();
    key("Z", { ctrlKey: true, shiftKey: true });
    await settle();
    expect(core.argsOf("undo_edit").map((a) => a.redo)).toEqual([false, true, false, true]);
  });

  test("text fields keep their own undo", async () => {
    await relay.edit({ op: "delete_step", index: 0 });
    const input = document.createElement("input");
    document.body.append(input);
    key("z", { ctrlKey: true }, input);
    await settle();
    expect(core.argsOf("undo_edit")).toEqual([]);
    input.remove();
  });

  test("Ctrl + Alt + Z isn't undo", async () => {
    await relay.edit({ op: "delete_step", index: 0 });
    key("z", { ctrlKey: true, altKey: true });
    await settle();
    expect(core.argsOf("undo_edit")).toEqual([]);
  });

  test("the export dialog and a hotkey being set own the keyboard", async () => {
    await relay.edit({ op: "delete_step", index: 0 });
    relay.exportOpen = true;
    key("z", { ctrlKey: true });
    relay.exportOpen = false;
    const capture = document.createElement("button");
    capture.setAttribute("data-captures-keys", "");
    document.body.append(capture);
    key("z", { ctrlKey: true }, capture);
    capture.remove();
    await settle();
    expect(core.argsOf("undo_edit")).toEqual([]);
  });

  test("in the app, F9, F10 and Esc are Rust's global hotkeys, not the page's", async () => {
    core.emit(session("playing"));
    key("F9");
    key("F10");
    key("Escape");
    key("m", { ctrlKey: true, shiftKey: true });
    await settle();
    expect(core.commands()).toEqual([]);
    expect(relay.expanded).toBe(true);
  });

  test("dispose stops listening", async () => {
    await relay.edit({ op: "delete_step", index: 0 });
    relay.dispose();
    key("z", { ctrlKey: true });
    await settle();
    expect(core.argsOf("undo_edit")).toEqual([]);
  });
});

describe("the browser preview (npm run dev)", () => {
  let r: RelayStore;
  beforeEach(async () => {
    core.uninstall();
    r = await freshStore({ backend: browserBackend() });
    r.start();
  });
  afterEach(() => {
    r.dispose();
    core.install();
  });

  test("loads the samples without Rust", () => {
    expect(r.editable).toBe(false);
    expect(r.library).toHaveLength(4);
    expect(r.name).toBe("Export invoice to PDF");
    expect(core.calls).toEqual([]);
  });

  test("F10 plays and Esc stops", async () => {
    key("F10");
    await settle();
    expect(r.mode).toBe("playing");
    key("Escape");
    await settle();
    expect(r.mode).toBe("idle");
  });

  test("F9 starts the countdown and Ctrl + Shift + M toggles the compact player", async () => {
    key("F9");
    await settle();
    expect(r.mode).toBe("countdown");
    key("Escape");
    await settle();
    expect(r.mode).toBe("idle");
    key("M", { ctrlKey: true, shiftKey: true });
    expect(r.expanded).toBe(false);
  });

  test("edits explain that they need the app", async () => {
    await r.edit({ op: "delete_step", index: 0 });
    expect(r.error).toBe("Editing needs the Relay app");
    expect(r.canUndo).toBe(false);
  });

  test("Esc while idle does nothing", async () => {
    key("Escape");
    await settle();
    expect(r.mode).toBe("idle");
  });
});

test("the store is on window.__relay for debugging and end-to-end tests", () => {
  expect((window as unknown as { __relay: RelayStore }).__relay).toBe(relay);
});
