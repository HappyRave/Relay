// The fake core answers like src-tauri and relay-core, so the UI tests mean
// something: these pin the behaviors the store relies on, with the numbers
// relay-core's own tests use (edit.rs, history.rs, library.rs, hotkeys.rs).
import { beforeEach, describe, expect, test, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { tauriBackend as b } from "../lib/ipc/backend";
import { core, defaultTriggers } from "./fake-core";
import { DEFAULT_SETTINGS } from "../lib/defaults";
import type { EditOp, MacroView } from "../lib/types";
import type { EngineMsg } from "../lib/ipc/bindings/EngineMsg";

const [A, B, C] = [1, 2, 3].map((n) => `00000000-0000-0000-0000-00000000000${n}`);
const edit = (op: EditOp, id = A) => b.editMacro(id, op);
const times = (v: MacroView) => v.steps.map((s) => [s.t, s.end]);

let got: EngineMsg[];
beforeEach(async () => {
  core.reset();
  got = [];
  await b.subscribe((m) => got.push(m));
});

describe("edits, like edit.rs", () => {
  test("an insert at a step's start goes after the step, pushes the rest back and grows the macro", async () => {
    const before = core.view(A);
    const v = await edit({ op: "insert_wait", at: 850, dur: 500, label: "x" });
    expect(v.steps[1]).toMatchObject({ kind: "wait", t: 911, end: 1411, dur: 500, items: [56] });
    expect(times(v).slice(2, 4)).toEqual([
      [2250, 2310],
      [2500, 3200],
    ]);
    expect(v.duration).toBe(before.duration + 500);
    // Later steps' events come one later.
    expect(v.steps[2].items).toEqual([98, 99]);
    expect(v.moves.find((m) => m.t > 911)!.t).toBeGreaterThanOrEqual(1411);
  });

  test("deleting a wait closes its gap; deleting a step renumbers the later ones", async () => {
    const v = await edit({ op: "delete_step", index: 2 }); // Wait 0.7 s at 2000
    expect(times(v).slice(2, 3)).toEqual([[2800, 2980]]); // was 3500
    expect(v.duration).toBe(10150 - 700);
    expect(v.steps[2].items).toEqual([149, 150, 151, 152]);
    const again = await edit({ op: "delete_step", index: 0 }); // a click: its time stays
    expect(again.steps[0].t).toBe(1750);
    expect(again.steps[0].items).toEqual([95, 96]);
  });

  test("inserting then deleting a wait gives the macro back", async () => {
    const original = core.view(A);
    await edit({ op: "insert_wait", at: 1500, dur: 700, label: "" });
    const v = await edit({ op: "delete_step", index: 1 });
    expect(v.steps).toEqual(original.steps);
    expect(v.moves).toEqual(original.moves);
    expect(v.duration).toBe(original.duration);
  });

  test("a pause retimes what follows", async () => {
    const v = await edit({ op: "set_pause", index: 1, dur: 340 }); // was 840
    expect(times(v).slice(1, 3)).toEqual([
      [1250, 1310],
      [1500, 2200],
    ]);
    expect(v.duration).toBe(10150 - 500);
    expect(v.steps[1].pause).toBe(340);
  });

  test("trimming pauses shortens only the long ones, and the macro", async () => {
    const v = await edit({ op: "cap_pauses", max: 1000 });
    expect(v.steps.map((s) => s.pause)).toEqual([850, 840, 190, 800, 70, 220, 995, 190, 0, 1000, 610, 210]);
    expect(v.duration).toBe(10150 - 420);
  });

  test("a pixel check's duration moves what follows", async () => {
    const v = await edit({ op: "set_wait_duration", index: 7, dur: 400 }); // was 900
    expect(times(v).slice(7, 9)).toEqual([
      [6270, 6670],
      [6670, 6750],
    ]);
    expect(v.duration).toBe(10150 - 500);
  });

  test("wrong-kind edits and missing steps are refused with Rust's messages", async () => {
    await expect(edit({ op: "set_wait_duration", index: 0, dur: 5 })).rejects.toEqual({
      code: "edit_rejected",
      message: "step 0 can't be edited this way",
    });
    await expect(edit({ op: "update_pixel_wait", index: 2, x: 1, y: 1, color: "#000000", tolerance: 1, timeout_ms: 500 })).rejects.toMatchObject({
      message: "step 2 can't be edited this way",
    });
    await expect(edit({ op: "set_label", index: 4, label: "x" })).rejects.toMatchObject({ message: "step 4 can't be edited this way" });
    await expect(edit({ op: "delete_step", index: 99 })).rejects.toEqual({ code: "edit_rejected", message: "there is no step 99" });
    expect(core.view(A).can_undo).toBe(false);
  });

  test("labels go on clicks, drags, waits and pixel checks", async () => {
    for (const index of [0, 2, 7]) await edit({ op: "set_label", index, label: `L${index}` });
    const labels = core.view(A).steps.map((s) => ("label" in s ? s.label : null));
    expect([labels[0], labels[2], labels[7]]).toEqual(["L0", "L2", "L7"]);
  });

  test("arguments Rust can't deserialize are refused", async () => {
    const bad: [EditOp, string][] = [
      [{ op: "set_pause", index: 1, dur: 1.5 }, "invalid type: floating point `1.5`, expected u32"],
      [{ op: "set_pause", index: 1, dur: -1 }, "invalid value: integer `-1`, expected u32"],
      [{ op: "update_pixel_wait", index: 7, x: 1.2, y: 0, color: "#000000", tolerance: 1, timeout_ms: 500 }, "expected i32"],
      [{ op: "update_pixel_wait", index: 7, x: 1, y: 0, color: "#000000", tolerance: 256, timeout_ms: 500 }, "expected u8"],
      [{ op: "update_pixel_wait", index: 7, x: 1, y: 0, color: "red", tolerance: 1, timeout_ms: 500 }, 'invalid color "red"'],
    ];
    for (const [op, why] of bad) await expect(edit(op)).rejects.toContain(why);
    expect(core.view(A).can_undo).toBe(false);
  });
});

describe("history, like history.rs", () => {
  test("undo brings back the name and the events, not the playback options", async () => {
    const original = core.view(A);
    await edit({ op: "delete_step", index: 0 });
    await b.setPlaybackOptions(A, { ...original.playback, speed: 4 });
    const v = await b.undoEdit(A, false);
    expect(v.steps).toEqual(original.steps);
    expect(v.playback.speed).toBe(4);
    expect([v.can_undo, v.can_redo]).toEqual([false, true]);
    expect((await b.undoEdit(A, true)).steps).toHaveLength(11);
  });

  test("renames less than 2 s apart are one undo step", async () => {
    vi.useFakeTimers();
    for (const name of ["R", "Re", "Rel"]) {
      await edit({ op: "rename", name });
      await vi.advanceTimersByTimeAsync(1500);
    }
    await vi.advanceTimersByTimeAsync(1000);
    await edit({ op: "rename", name: "Relay" });
    expect((await b.undoEdit(A, false)).name).toBe("Rel");
    const v = await b.undoEdit(A, false);
    expect(v.name).toBe("Export invoice to PDF");
    expect(v.can_undo).toBe(false);
  });

  test("a new edit drops the redo", async () => {
    await edit({ op: "delete_step", index: 0 });
    await b.undoEdit(A, false);
    const v = await edit({ op: "set_label", index: 0, label: "x" });
    expect(v.can_redo).toBe(false);
  });
});

describe("the library, like library.rs", () => {
  test("the samples' hotkeys are seeded off, and the list only shows hotkeys that are on", async () => {
    expect((await b.getTriggers(A)).triggers.hotkey).toEqual({ enabled: false, combo: "Ctrl + Alt + 1" });
    expect((await b.listMacros()).map((m) => m.hotkey)).toEqual([null, null, null, null]);
    await b.setTriggers(A, { ...defaultTriggers(), hotkey: { enabled: true, combo: "Ctrl + Alt + 1" } });
    expect((await b.listMacros())[0].hotkey).toBe("Ctrl + Alt + 1");
  });

  test("a copy is named “(copy)”, then “(copy) 2”, right after the original, with no triggers", async () => {
    await b.setTriggers(A, { ...defaultTriggers(), hotkey: { enabled: true, combo: "Ctrl + Alt + 1" } });
    const one = await b.duplicateMacro(A);
    const two = await b.duplicateMacro(A);
    expect((await b.listMacros()).map((m) => m.name).slice(0, 3)).toEqual([
      "Export invoice to PDF",
      "Export invoice to PDF (copy) 2",
      "Export invoice to PDF (copy)",
    ]);
    expect(core.ids.slice(1, 3)).toEqual([two, one]);
    expect((await b.getTriggers(one)).triggers).toEqual(defaultTriggers());
  });

  test("screenshots come back as raw bytes, empty without one, and a copy gets its original's", async () => {
    expect(new Uint8Array(await b.screenshot(A))).toEqual(new Uint8Array());
    core.screens.set(A, new Uint8Array([1, 2, 3]));
    expect(new Uint8Array(await b.screenshot(A))).toEqual(new Uint8Array([1, 2, 3]));
    const copy = await b.duplicateMacro(A);
    expect(new Uint8Array(await b.screenshot(copy))).toEqual(new Uint8Array([1, 2, 3]));
    await expect(invoke("screenshot", {})).rejects.toMatch(/invalid args `id` for command `screenshot`/);
  });

  test("a restored macro goes back where it was", async () => {
    await b.deleteMacro(A);
    await b.deleteMacro(B);
    await b.restoreMacro(B);
    expect(core.ids.slice(0, 2)).toEqual([B, C]);
    await b.restoreMacro(A);
    expect(core.ids).toEqual([A, B, C, "00000000-0000-0000-0000-000000000004"]);
  });

  test("a restored macro whose hotkey was taken meanwhile comes back with it off, and says so", async () => {
    await b.setTriggers(A, { ...defaultTriggers(), hotkey: { enabled: true, combo: "Ctrl + Alt + 1" } });
    await b.deleteMacro(A);
    await b.setTriggers(B, { ...defaultTriggers(), hotkey: { enabled: true, combo: "Alt + Ctrl + 1" } });
    await b.restoreMacro(A);
    expect((await b.getTriggers(A)).triggers.hotkey).toEqual({ enabled: false, combo: "Ctrl + Alt + 1" });
    expect(got).toContainEqual({
      type: "notice",
      message: "“Export invoice to PDF” is back; its hotkey Ctrl + Alt + 1 is now used by “Fill weekly timesheet”, so it's off",
    });
  });

  test("imports keep their own name, made unique, at the top in order, with a new id if theirs is taken", async () => {
    const own = { ...structuredClone(core.view(C)), id: "00000000-0000-0000-0000-0000000000aa", name: "Weekly report" };
    core.files.set("C:\\a.rly", own);
    core.files.set("C:\\b.rly", structuredClone(core.view(A))); // an export of a macro still here
    core.files.set("C:\\c.rly", "expected value at line 1");
    const result = await invoke<{ imported: string[]; problems: string[] }>("import_macros", { paths: ["C:\\a.rly", "C:\\b.rly", "C:\\c.rly"] });
    expect(result.problems).toEqual(["c.rly: expected value at line 1"]);
    expect(result.imported[0]).toBe(own.id);
    expect(result.imported[1]).not.toBe(A);
    const list = await b.listMacros();
    expect(list.slice(0, 3).map((m) => m.name)).toEqual(["Weekly report", "Export invoice to PDF 2", "Export invoice to PDF"]);
    expect(list.slice(0, 2).map((m) => m.id)).toEqual(result.imported);
  });

  test("a file the test didn't fill in holds the first sample, named after the file", async () => {
    const [id] = (await invoke<{ imported: string[] }>("import_macros", { paths: ["C:\\macros\\Weekly report.rly"] })).imported;
    expect(core.view(id)).toMatchObject({ name: "Weekly report", duration: 10150 });
  });
});

describe("the busy guard", () => {
  test("deletes, edits and undo are refused during a session, as the session messages say", async () => {
    await edit({ op: "delete_step", index: 0 });
    core.emit({ type: "session", mode: "playing", macro_id: A });
    const busy = { code: "busy", message: "Stop the recording or playback first" };
    await expect(b.deleteMacro(B)).rejects.toEqual(busy);
    await expect(edit({ op: "delete_step", index: 0 })).rejects.toEqual(busy);
    await expect(b.undoEdit(A, false)).rejects.toEqual(busy);
    core.mode = "idle";
    await expect(b.undoEdit(A, false)).resolves.toMatchObject({ can_redo: true });
  });
});

describe("triggers, like hotkeys.rs", () => {
  const withHotkey = (combo: string, enabled = true) => ({ ...defaultTriggers(), hotkey: { enabled, combo } });

  test.each([
    ["F9", "F9 is one of Relay's own hotkeys"],
    ["F10", "F10 is one of Relay's own hotkeys"],
    ["Shift + Ctrl + M", "Shift + Ctrl + M is one of Relay's own hotkeys"],
    ["Ctrl+Alt+End", "Ctrl+Alt+End is one of Relay's own hotkeys"],
    ["Q", "Add Ctrl, Alt, Shift or Win, so the key still types normally"],
    ["Ctrl + Banana", "“Banana” isn't a key Relay can use"],
    ["Hyper + A", "“Hyper” isn't a modifier (use Ctrl, Alt, Shift or Win)"],
    ["Shift + A", "Add Ctrl, Alt or Win: Shift + A is ordinary typing"],
    ["Ctrl + Shift", "Add a key after Shift: a hotkey can't end with a modifier"],
    ["Ctrl + + K", "The hotkey is empty or incomplete"],
  ])("%s is refused", async (combo, message) => {
    await expect(b.setTriggers(A, withHotkey(combo))).rejects.toEqual({ code: "hotkey", message });
  });

  test("another macro's hotkey is refused while it's on, written either way", async () => {
    await b.setTriggers(B, withHotkey("Alt + Ctrl + 7"));
    await expect(b.setTriggers(A, withHotkey("Ctrl + Alt + 7"))).rejects.toEqual({
      code: "hotkey",
      message: "Ctrl + Alt + 7 already runs “Fill weekly timesheet”",
    });
    await b.setTriggers(B, withHotkey("Alt + Ctrl + 7", false));
    await expect(b.setTriggers(A, withHotkey("Ctrl + Alt + 7"))).resolves.toBeTruthy();
  });

  test("a hotkey that's off isn't checked, and Shift with a function key is fine", async () => {
    await expect(b.setTriggers(A, withHotkey("F9", false))).resolves.toBeTruthy();
    await expect(b.setTriggers(A, withHotkey("Shift + F7"))).resolves.toBeTruthy();
  });

  test("the hotkey error only shows while the hotkey is on", async () => {
    core.hotkeyErrors.set(A, "Ctrl + Alt + 1 is taken by another app");
    expect((await b.getTriggers(A)).hotkey_error).toBeNull();
    expect((await b.setTriggers(A, withHotkey("Ctrl + Alt + 1"))).hotkey_error).toBe("Ctrl + Alt + 1 is taken by another app");
  });

  test("the next run: none while off or with no days, else the next matching day", async () => {
    const schedule = (enabled: boolean, days: boolean[], time = "09:00") => ({
      ...defaultTriggers(),
      schedule: { enabled, schedule: { days: days as [boolean, boolean, boolean, boolean, boolean, boolean, boolean], time } },
    });
    const weekdays = [true, true, true, true, true, false, false];
    expect((await b.setTriggers(A, schedule(false, weekdays))).next_run).toBeNull();
    expect((await b.setTriggers(A, schedule(true, Array(7).fill(false)))).next_run).toBeNull();
    // "Now" is Thursday 14:00.
    expect((await b.setTriggers(A, schedule(true, weekdays))).next_run).toBe("2026-09-25T09:00:00+02:00");
    expect((await b.setTriggers(A, schedule(true, weekdays, "15:30"))).next_run).toBe("2026-09-24T15:30:00+02:00");
    expect((await b.setTriggers(A, schedule(true, [true, false, false, false, false, false, false]))).next_run).toBe(
      "2026-09-28T09:00:00+02:00",
    );
  });

  test("bad trigger values are refused", async () => {
    const t = defaultTriggers();
    await expect(b.setTriggers(A, { ...t, pixel: { ...t.pixel, x: 1.5 } })).rejects.toContain("expected i32");
    await expect(b.setTriggers(A, { ...t, pixel: { ...t.pixel, tolerance: 300 } })).rejects.toContain("expected u8");
    await expect(b.setTriggers(A, { ...t, pixel: { ...t.pixel, color: "#12345" } })).rejects.toContain("invalid color");
    await expect(b.setTriggers(A, { ...t, app_launch: { ...t.app_launch, delay_ms: 2.5 } })).rejects.toContain("expected u32");
  });

  test("pausing triggers is confirmed on the engine stream, after the command returns", async () => {
    await b.setTriggersPaused(true);
    expect(got).toEqual([]);
    await new Promise((r) => setTimeout(r, 0));
    expect(got).toEqual([{ type: "triggers_paused", paused: true }]);
  });
});

describe("settings and saving", () => {
  test("missing settings fall back to the defaults; unknown values are refused", async () => {
    expect(await invoke("update_settings", { settings: { countdown: false } })).toEqual({ ...DEFAULT_SETTINGS, countdown: false });
    await expect(invoke("update_settings", { settings: { keep_on_top: "sometimes" } })).rejects.toContain("unknown variant");
  });

  test("a change that couldn't be written is kept, and the engine stream says so", async () => {
    core.saveError = "disk full";
    const v = await edit({ op: "delete_step", index: 0 });
    expect(v.steps).toHaveLength(11);
    expect(got).toEqual([{ type: "error", message: "Couldn't save the change: disk full. It's kept until you quit." }]);
  });
});

describe("the engine stream", () => {
  test("errors and notices sent before the UI subscribes arrive when it does", async () => {
    core.reset();
    core.emit({ type: "error", message: "one" });
    core.emit({ type: "notice", message: "two" });
    expect(() => core.emit({ type: "library_changed" })).toThrow("hasn't subscribed");
    const late: EngineMsg[] = [];
    await b.subscribe((m) => late.push(m));
    expect(late).toEqual([
      { type: "error", message: "one" },
      { type: "notice", message: "two" },
    ]);
  });

  test("emitLater lets a frame run between messages", async () => {
    const frames: number[] = [];
    let raf = 0;
    const frame = () => (raf = requestAnimationFrame(() => (frames.push(got.length), frame())));
    frame();
    await core.emitLater({ type: "countdown", left_ms: 2 }, { type: "countdown", left_ms: 1 });
    cancelAnimationFrame(raf);
    expect(got).toHaveLength(2);
    expect(frames).toContain(1);
  });

  test("commands that can't fail in Rust can't be made to fail", () => {
    for (const cmd of ["toggle_record", "get_settings", "list_macros", "seek"]) expect(() => core.fail(cmd)).toThrow("can't fail");
  });
});
