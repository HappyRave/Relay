// The fake core answers like src-tauri and relay-core, so the UI tests mean
// something: these pin the behaviors the store relies on, with the numbers
// relay-core's own tests use (edit.rs, history.rs, library.rs, hotkeys.rs).
import { beforeEach, describe, expect, test, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { tauriBackend as b } from "../lib/ipc/backend";
import { core, defaultTriggers } from "./fake-core";
import { png } from "./app";
import { DEFAULT_SETTINGS } from "../lib/defaults";
import type { EditOp, MacroView } from "../lib/types";
import type { EngineMsg } from "../lib/ipc/bindings/EngineMsg";
import { smooth, straighten, type Point } from "./fake-path";
import pathCases from "./path-cases.json";

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
  // The invoice sample: each click has the MOVE step that leads to it just before it.
  test("an insert at a step's start goes after the step, pushes the rest back and grows the macro", async () => {
    const before = core.view(A);
    const v = await edit({ op: "insert_wait", at: 850, dur: 500, label: "x" });
    expect(v.steps[2]).toMatchObject({ kind: "wait", t: 911, end: 1411, dur: 500, items: [56] });
    expect(times(v).slice(4, 6)).toEqual([
      [2250, 2310],
      [2500, 3200],
    ]);
    expect(v.duration).toBe(before.duration + 500);
    // Later steps' events come one later.
    expect(v.steps[4].items).toEqual([98, 99]);
    expect(v.moves.find((m) => m.t > 911)!.t).toBeGreaterThanOrEqual(1411);
  });

  test("deleting a wait closes its gap; deleting a step renumbers the later ones", async () => {
    const v = await edit({ op: "delete_step", index: 4 }); // Wait 0.7 s at 2000
    expect(times(v).slice(5, 6)).toEqual([[2800, 2980]]); // was 3500
    expect(v.duration).toBe(10150 - 700);
    expect(v.steps[5].items).toEqual([149, 150, 151, 152]);
    const again = await edit({ op: "delete_step", index: 0 }); // the first move: the click keeps its time
    expect(again.steps[0]).toMatchObject({ kind: "click", t: 850, pause: 850, items: [0, 1] });
    expect(again.steps[1].items[0]).toBe(2);
    expect(again.moves).toHaveLength(247);
  });

  test("deleting a click joins the moves either side of it", async () => {
    const v = await edit({ op: "delete_step", index: 1 });
    expect(v.steps[0]).toMatchObject({ kind: "move", t: 0, end: 1750, samples: 54 + 41, items: [...Array(95).keys()] });
    expect(v.steps[1]).toMatchObject({ kind: "click", t: 1750, items: [95, 96] });
    expect(v.moves).toHaveLength(299);
  });

  test("inserting then deleting a wait gives the macro back", async () => {
    const original = core.view(A);
    const v1 = await edit({ op: "insert_wait", at: 1000, dur: 700, label: "" });
    expect(v1.steps[2]).toMatchObject({ kind: "wait", t: 1000, pause: 90 });
    expect(v1.steps[3]).toMatchObject({ kind: "move", t: 1816, pause: 116 });
    const v = await edit({ op: "delete_step", index: 2 });
    expect(v.steps).toEqual(original.steps);
    expect(v.moves).toEqual(original.moves);
    expect(v.duration).toBe(original.duration);
  });

  test("a pause retimes what follows", async () => {
    const v = await edit({ op: "set_pause", index: 2, dur: 100 }); // the move after the first click: was 206
    expect(times(v).slice(2, 5)).toEqual([
      [1010, 1644],
      [1644, 1704],
      [1894, 2594],
    ]);
    expect(v.duration).toBe(10150 - 106);
    expect(v.steps[2].pause).toBe(100);
  });

  test("a pause added where there was none leaves the move before it alone", async () => {
    // The first click comes as its move ends: both at 850.
    const v = await edit({ op: "set_pause", index: 1, dur: 300 });
    expect(times(v).slice(0, 3)).toEqual([
      [0, 850],
      [1150, 1210],
      [1416, 2050],
    ]);
    expect(v.steps[1].pause).toBe(300);
    expect(v.moves.filter((m) => m.t === 850)).toHaveLength(1);
  });

  test("trimming pauses shortens only the long ones, and the macro", async () => {
    await edit({ op: "set_pause", index: 4, dur: 2500 }); // the wait: was 190
    const v = await edit({ op: "cap_pauses", max: 1000 });
    expect(v.steps.map((s) => s.pause)).toEqual([0, 0, 206, 0, 1000, 16, 0, 70, 220, 261, 0, 190, 0, 336, 0, 206, 0, 210]);
    expect(v.duration).toBe(10150 + 810);
  });

  test("a pixel check's duration moves what follows", async () => {
    const v = await edit({ op: "set_wait_duration", index: 11, dur: 400 }); // was 900
    expect(times(v).slice(11, 13)).toEqual([
      [6270, 6670],
      [6670, 6750],
    ]);
    expect(v.duration).toBe(10150 - 500);
  });

  test("a move's duration retimes it and moves what follows", async () => {
    const v = await edit({ op: "set_move_duration", index: 2, dur: 300 }); // 1116..1750
    expect(times(v).slice(2, 5)).toEqual([
      [1116, 1416],
      [1416, 1476],
      [1666, 2366],
    ]);
    expect(v.duration).toBe(10150 - 334);
    const samples = v.moves.slice(56, 56 + 41); // after the first move and the click's press and release
    expect([samples[0].t, samples[40].t]).toEqual([1116, 1416]);
    expect(samples.every((m, i) => i === 0 || m.t >= samples[i - 1].t)).toBe(true);
  });

  test("smoothing and straightening a move change where its samples are, not when", async () => {
    const original = core.view(A);
    const samples = (v: MacroView) => v.moves.slice(56, 56 + 41);
    const v = await edit({ op: "straighten_move", index: 2 });
    expect(samples(v).map((m) => m.t)).toEqual(samples(original).map((m) => m.t));
    expect(samples(v).at(-1)).toEqual(samples(original).at(-1));
    // On the line from where the cursor was (the click before) to the move's end.
    const s = original.steps[2];
    if (s.kind !== "move") throw new Error("not a move");
    const off = (m: { x: number; y: number }) =>
      Math.abs((m.x - s.x) * (s.to_y - s.y) - (m.y - s.y) * (s.to_x - s.x)) / Math.hypot(s.to_x - s.x, s.to_y - s.y);
    for (const m of samples(v)) expect(off(m)).toBeLessThan(1);
    expect(v.steps).toEqual(original.steps);
    // A straight line stays straight: nothing changed, so it isn't an edit to undo.
    await edit({ op: "smooth_move", index: 2 });
    expect((await b.undoEdit(A, false)).moves).toEqual(original.moves);
    expect(core.view(A).can_undo).toBe(false);
  });

  test("wrong-kind edits and missing steps are refused with Rust's messages", async () => {
    await expect(edit({ op: "set_wait_duration", index: 0, dur: 5 })).rejects.toEqual({
      code: "edit_rejected",
      message: "step 0 can't be edited this way",
    });
    await expect(edit({ op: "update_pixel_wait", index: 4, x: 1, y: 1, color: "#000000", tolerance: 1, timeout_ms: 500 })).rejects.toMatchObject({
      message: "step 4 can't be edited this way",
    });
    await expect(edit({ op: "set_label", index: 7, label: "x" })).rejects.toMatchObject({ message: "step 7 can't be edited this way" });
    await expect(edit({ op: "set_label", index: 0, label: "x" })).rejects.toMatchObject({ message: "step 0 can't be edited this way" });
    await expect(edit({ op: "set_move_duration", index: 1, dur: 5 })).rejects.toMatchObject({ message: "step 1 can't be edited this way" });
    await expect(edit({ op: "smooth_move", index: 1 })).rejects.toMatchObject({ message: "step 1 can't be edited this way" });
    await expect(edit({ op: "straighten_move", index: 4 })).rejects.toMatchObject({ message: "step 4 can't be edited this way" });
    await expect(edit({ op: "delete_step", index: 99 })).rejects.toEqual({ code: "edit_rejected", message: "there is no step 99" });
    expect(core.view(A).can_undo).toBe(false);
  });

  test("labels go on clicks, drags, waits and pixel checks", async () => {
    for (const index of [1, 4, 11]) await edit({ op: "set_label", index, label: `L${index}` });
    const labels = core.view(A).steps.map((s) => ("label" in s ? s.label : null));
    expect([labels[1], labels[4], labels[11]]).toEqual(["L1", "L4", "L11"]);
  });

  test("a Find image step is a wait: inserted, retimed, updated, labeled and deleted like edit.rs", async () => {
    const before = core.view(A);
    const image = png(40, 20);
    const find = { image, click_x: 20, click_y: 10, btn: "Left" as const, threshold: 85, timeout_ms: 5000, area: null };
    let v = await edit({ op: "insert_find_image", at: 850, dur: 700, ...find, label: "" });
    expect(v.steps[2]).toMatchObject({ kind: "find_image", t: 911, end: 1611, dur: 700, items: [56], ...find });
    expect(v.duration).toBe(before.duration + 700);
    v = await edit({ op: "set_wait_duration", index: 2, dur: 200 });
    expect(v.duration).toBe(before.duration + 200);
    const area = { x: -1920, y: 0, w: 1920, h: 1080 };
    v = await edit({ op: "update_find_image", index: 2, ...find, click_x: -4, btn: "Right", threshold: 120, area });
    expect(v.steps[2]).toMatchObject({ click_x: -4, btn: "Right", threshold: 100, area, dur: 200 });
    v = await edit({ op: "update_find_image", index: 2, ...find, threshold: 5 });
    expect(v.steps[2]).toMatchObject({ threshold: 50, area: null });
    v = await edit({ op: "set_label", index: 2, label: "OK" });
    expect(v.steps[2]).toMatchObject({ label: "OK" });
    await expect(edit({ op: "update_find_image", index: 1, ...find })).rejects.toMatchObject({ message: "step 1 can't be edited this way" });
    v = await edit({ op: "delete_step", index: 2 });
    expect(v.steps).toEqual(before.steps);
    expect(v.duration).toBe(before.duration);
  });

  test("a Find image step's area may be left out, like an Option", async () => {
    const op = { op: "insert_find_image", at: 0, dur: 100, image: png(8, 8), click_x: 0, click_y: 0, btn: "Left", threshold: 85, timeout_ms: 5000, label: "" };
    const v = await edit(op as EditOp);
    expect(v.steps.find((s) => s.kind === "find_image")).toMatchObject({ area: null });
  });

  test("a Text step lasts as long as its typing: inserted, lengthened by a longer text, retimed and deleted", async () => {
    const before = core.view(A);
    let v = await edit({ op: "insert_text", at: 850, text: "No. {n}" });
    expect(v.steps[2]).toEqual({ kind: "text", t: 911, end: 961, pause: 1, items: [56], dur: 50, text: "No. {n}" });
    expect(v.steps[3]).toMatchObject({ kind: "move", t: 1166, end: 1800, pause: 205, items: [57, ...v.steps[3].items.slice(1)] });
    expect(v.steps[18]).toMatchObject({ t: 9660, end: 9700, items: [338, 339] });
    expect(v.duration).toBe(10200);
    v = await edit({ op: "update_text", index: 2, text: "x".repeat(70) });
    expect(v.steps[2]).toMatchObject({ t: 911, end: 1611, dur: 700 });
    expect(times(v).slice(3, 5)).toEqual([
      [1816, 2450],
      [2450, 2510],
    ]);
    expect(v.duration).toBe(10850);
    v = await edit({ op: "update_text", index: 2, text: "{date}" });
    expect(v.steps[2]).toMatchObject({ dur: 700, text: "{date}" });
    v = await edit({ op: "set_wait_duration", index: 2, dur: 0 });
    expect(v.steps[2]).toMatchObject({ dur: 100, end: 1011 });
    v = await edit({ op: "delete_step", index: 2 });
    expect(v.steps).toEqual(before.steps);
    expect(v.duration).toBe(before.duration);
  });

  test("a Text step with a mistake is refused with Rust's message", async () => {
    const refused = (text: string, message: string) =>
      expect(edit({ op: "insert_text", at: 0, text })).rejects.toEqual({ code: "edit_rejected", message });
    await refused("{name}", "{name} isn't a placeholder: use {date}, {time}, {clipboard}, {n} or {col:Name}.");
    await refused("{Date}", "{Date} isn't a placeholder: use {date}, {time}, {clipboard}, {n} or {col:Name}.");
    await refused("{col: }", "{col:} needs the name of a column, as in {col:Customer}.");
    await refused("a {date", "A { isn't closed: type {{ for a brace.");
    await refused("{da{te}", "A { isn't closed: type {{ for a brace.");
    await refused("a } b", "A } has no { before it: type }} for a brace.");
    await edit({ op: "insert_text", at: 0, text: "{{n}}" }); // after the first move and click: step 2
    await expect(edit({ op: "update_text", index: 2, text: "}" })).rejects.toMatchObject({
      message: "A } has no { before it: type }} for a brace.",
    });
    await expect(edit({ op: "update_text", index: 1, text: "x" })).rejects.toMatchObject({ message: "step 1 can't be edited this way" });
    await expect(edit({ op: "make_editable", index: 1 })).rejects.toMatchObject({ message: "step 1 can't be edited this way" });
  });

  test("Make editable turns the typing into a Text step over the same time", async () => {
    const before = core.view(A);
    const v = await edit({ op: "make_editable", index: 8 }); // “invoice_0924”, 26 key events
    expect(v.steps[8]).toEqual({ kind: "text", t: 4050, end: 5025, pause: 220, items: [158], dur: 975, text: "invoice_0924" });
    expect(times(v)).toEqual(times(before));
    expect(v.steps[9].items.slice(0, 3)).toEqual([159, 160, 161]);
    expect(v.steps[10].items).toEqual([206, 207]);
    expect(v.steps[17].items).toEqual([312, 313]);
    expect(v.duration).toBe(10150);
  });

  test("preview_text fills in the placeholders as the first repeat would, now", async () => {
    core.clipboardText = "ACME";
    expect(await b.previewText(A, "{date} {time} #{n} {clipboard} {{x}}")).toBe("2026-10-01 09:05:07 #1 ACME {x}");
    core.clipboardText = null;
    expect(await b.previewText(A, "[{clipboard}]")).toBe("[]");
    await expect(b.previewText(A, "{when}")).rejects.toEqual({
      code: "invalid_text",
      message: "{when} isn't a placeholder: use {date}, {time}, {clipboard}, {n} or {col:Name}.",
    });
  });

  test("a data file is read when asked for, and its first row fills the preview", async () => {
    const csv = "C:\\Data\\customers.csv";
    core.csv.set(csv, { columns: ["Customer", "Total"], rows: [["ACME", "12"], ["Globex"]] });
    expect(await b.getDataFile(A)).toBeNull();
    await expect(b.previewText(A, "{col:Customer}")).rejects.toEqual({
      code: "data_file",
      message: "A Text step types {col:Customer}: choose a data file in Settings → Playback.",
    });
    core.dialog.open = null;
    expect(await b.chooseDataFile(A)).toBeNull();
    expect(core.calls.some((c) => c.cmd === "set_data_file")).toBe(false);
    core.dialog.open = csv;
    expect(await b.chooseDataFile(A)).toEqual({ path: csv, columns: ["Customer", "Total"], rows: 2, error: null });
    expect(core.calls.at(-1)).toEqual({ cmd: "set_data_file", args: { id: A, path: csv } });
    expect(await b.previewText(A, "{col:customer} owes {col: Total}")).toBe("ACME owes 12");
    await expect(b.previewText(A, "{col:Due}")).rejects.toEqual({ code: "data_file", message: "customers.csv has no column “Due”." });

    // It's read each time: a change on disk shows.
    core.csv.set(csv, "customers.csv: Row 2 has 3 values, but there are 2 columns.");
    expect(await b.getDataFile(A)).toEqual({
      path: csv,
      columns: [],
      rows: 0,
      error: "customers.csv: Row 2 has 3 values, but there are 2 columns.",
    });
    core.csv.delete(csv);
    expect((await b.getDataFile(A))?.error).toBe("customers.csv isn't there anymore: choose it again in Settings → Playback.");
    core.dialog.open = "C:\\gone.csv";
    await expect(b.chooseDataFile(A)).rejects.toEqual({
      code: "data_file",
      message: "gone.csv isn't there anymore: choose it again in Settings → Playback.",
    });
    expect((await b.getDataFile(A))?.path).toBe(csv);
    await b.removeDataFile(A);
    expect(core.calls.at(-1)).toEqual({ cmd: "set_data_file", args: { id: A, path: null } });
    expect(await b.getDataFile(A)).toBeNull();
  });

  test("a data file is kept by copies, and checked before a program is exported", async () => {
    const csv = "C:\\Data\\customers.csv";
    core.csv.set(csv, { columns: ["Customer"], rows: [] });
    core.dialog.open = csv;
    await b.chooseDataFile(A);
    expect((await b.getDataFile(A))?.error).toBe("customers.csv has no rows to play.");
    core.dialog.save = "C:\\out.exe";
    await expect(b.exportMacro(A, "exe", "x.exe")).rejects.toEqual({ code: "data_file", message: "customers.csv has no rows to play." });
    core.dialog.save = "C:\\out.rly";
    expect(await b.exportMacro(A, "rly", "x.rly")).toBe("C:\\out.rly");
    const copy = await b.duplicateMacro(A);
    expect((await b.getDataFile(copy))?.path).toBe(csv);
  });

  test("arguments Rust can't deserialize are refused", async () => {
    const bad: [EditOp, string][] = [
      [{ op: "set_pause", index: 1, dur: 1.5 }, "invalid type: floating point `1.5`, expected u32"],
      [{ op: "set_pause", index: 1, dur: -1 }, "invalid value: integer `-1`, expected u32"],
      [{ op: "update_pixel_wait", index: 11, x: 1.2, y: 0, color: "#000000", tolerance: 1, timeout_ms: 500 }, "expected i32"],
      [{ op: "update_pixel_wait", index: 11, x: 1, y: 0, color: "#000000", tolerance: 256, timeout_ms: 500 }, "expected u8"],
      [{ op: "update_pixel_wait", index: 11, x: 1, y: 0, color: "red", tolerance: 1, timeout_ms: 500 }, 'invalid color "red"'],
      [{ op: "insert_find_image", at: 0, dur: 1, image: "R0lGOD", click_x: 0, click_y: 0, btn: "Left", threshold: 85, timeout_ms: 1, area: null, label: "" }, "invalid image"],
      [{ op: "insert_find_image", at: 0, dur: 1, image: png(8, 8), click_x: 0, click_y: 0, btn: "Side" as never, threshold: 85, timeout_ms: 1, area: null, label: "" }, 'unknown variant "Side"'],
      [{ op: "insert_find_image", at: 0, dur: 1, image: png(8, 8), click_x: 0, click_y: 0, btn: "Left", threshold: 300, timeout_ms: 1, area: null, label: "" }, "expected u8"],
      [{ op: "insert_find_image", at: 0, dur: 1, image: png(8, 8), click_x: 0, click_y: 0, btn: "Left", threshold: 85, timeout_ms: 1, area: { x: 0, y: 0, w: 1.5, h: 1 }, label: "" }, "expected i32"],
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
    expect((await b.undoEdit(A, true)).steps).toHaveLength(17);
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
    const v = await edit({ op: "set_label", index: 1, label: "x" });
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

  test("an exported program imports like a macro file", async () => {
    core.files.set("C:\\ping.exe", "not a program exported by Relay");
    const result = await invoke<{ imported: string[]; problems: string[] }>("import_macros", {
      paths: ["C:\\Weekly report.exe", "C:\\ping.exe"],
    });
    expect(result.problems).toEqual(["ping.exe: not a program exported by Relay"]);
    expect(core.view(result.imported[0]).name).toBe("Weekly report");
  });

  test("exports take the formats Rust knows", async () => {
    for (const format of ["rly", "json", "exe"]) await invoke("export_macro", { id: A, format, path: "C:\\x" });
    await expect(invoke("export_macro", { id: A, format: "ahk", path: "C:\\x" })).rejects.toMatch(/unknown variant "ahk"/);
  });

  test("a file the test didn't fill in holds the first sample, named after the file", async () => {
    const [id] = (await invoke<{ imported: string[] }>("import_macros", { paths: ["C:\\macros\\Weekly report.rly"] })).imported;
    expect(core.view(id)).toMatchObject({ name: "Weekly report", duration: 10150 });
  });
});

describe("the run history, like run_history.rs", () => {
  test("starts empty, lists what runs.json holds, newest first, and can't fail", async () => {
    expect(await b.listRuns()).toEqual([]);
    const run = {
      at: "2026-09-24T09:12:00Z",
      macro_id: A,
      macro_name: "Export invoice to PDF",
      source: "hotkey" as const,
      outcome: { type: "finished" as const, reason: "completed" as const },
      duration_ms: 1200,
      from_ms: 0,
      loops: 1,
      speed: 1,
      humanize: false,
      checks: [],
      checks_dropped: 0,
    };
    core.runLog = [run, { ...run, at: "2026-09-24T08:00:00Z" }];
    expect((await b.listRuns()).map((r) => r.at)).toEqual(["2026-09-24T09:12:00Z", "2026-09-24T08:00:00Z"]);
    expect(() => core.fail("list_runs")).toThrow("can't fail");
  });
});

describe("the window, like window_ctl.rs", () => {
  test("dividers are saved sanitized, as Panes::sanitized does, and a reset clears them", async () => {
    await invoke("save_panes", { panes: { preview_w: -5, transport_h: 90, timeline_h: 1e9 } });
    expect((await invoke<{ panes: unknown }>("window_prefs")).panes).toEqual({ preview_w: null, transport_h: 90, timeline_h: 10_000 });
    await invoke("save_panes", { panes: { preview_w: 480 } });
    expect(core.window.panes).toEqual({ preview_w: 480, transport_h: null, timeline_h: null });
    await invoke("reset_layout");
    expect(core.window.panes).toEqual({ preview_w: null, transport_h: null, timeline_h: null });
    await expect(invoke("save_panes", { panes: { preview_w: "wide" } })).rejects.toMatch(/invalid args `panes`/);
    // NaN is null over JSON, which Rust drops like any other non-size.
    await invoke("save_panes", { panes: { preview_w: NaN, transport_h: 90, timeline_h: null } });
    expect(core.window.panes).toEqual({ preview_w: null, transport_h: 90, timeline_h: null });
  });

  test("fit_window remembers the mode, as window_ctl::fit does, and checks its arguments", async () => {
    await invoke("fit_window", { width: 604, height: 68, expanded: false });
    expect((await invoke<{ expanded: boolean }>("window_prefs")).expanded).toBe(false);
    await invoke("fit_window", { width: 0, height: 0, expanded: true });
    expect((await invoke<{ expanded: boolean }>("window_prefs")).expanded).toBe(true);
    await expect(invoke("fit_window", { width: 604, height: 68 })).rejects.toMatch(/invalid args `expanded`/);
    expect(() => core.fail("fit_window")).toThrow();
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
    await expect(invoke("update_settings", { settings: { preview_background: "photo" } })).rejects.toContain("unknown variant");
  });

  test("a change that couldn't be written is kept, and the engine stream says so", async () => {
    core.saveError = "disk full";
    const v = await edit({ op: "delete_step", index: 0 });
    expect(v.steps).toHaveLength(17);
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

describe("path reshaping, like path.rs", () => {
  test("smoothing and straightening land on the same pixels as Rust", () => {
    // Written by relay-core's `export_path_cases` test.
    for (const c of pathCases as { points: Point[]; smooth: Point[]; straighten: Point[] }[]) {
      expect(smooth(c.points)).toEqual(c.smooth);
      expect(straighten(c.points)).toEqual(c.straighten);
    }
    expect(pathCases.length).toBeGreaterThan(4);
  });
});

describe("images, like commands.rs", () => {
  test("paste reads the clipboard, and says when there's no picture", async () => {
    await expect(b.pasteImage()).rejects.toEqual({ code: "image", message: "There's no picture on the clipboard. Copy or snip one first." });
    core.clipboard = png(4, 8);
    expect(await b.pasteImage()).toBe(png(4, 8));
  });

  test("a file is read from the path the dialog returns, or nothing when it's cancelled", async () => {
    expect(await b.chooseImage()).toBeNull();
    core.dialog.open = "C:\shots\ok.png";
    await expect(b.chooseImage()).rejects.toMatchObject({ code: "io" });
    core.images.set("C:\shots\ok.png", png(10, 10));
    expect(await b.chooseImage()).toBe(png(10, 10));
    expect(core.argsOf("load_image").at(-1)).toEqual({ path: "C:\shots\ok.png" });
  });

  test("a snip is the image, or null when cancelled; Test answers what it found", async () => {
    expect(await b.snipImage()).toBeNull();
    core.snip = png(6, 6);
    expect(await b.snipImage()).toBe(png(6, 6));
    await b.cancelSnip();
    expect(await b.testFindImage(png(6, 6), 85, null)).toBeNull();
    core.found = { x: 1, y: 2, w: 6, h: 6, score: 93 };
    expect(await b.testFindImage(png(6, 6), 85, { x: 0, y: 0, w: 100, h: 100 })).toEqual(core.found);
    await expect(b.testFindImage("nope", 85, null)).rejects.toContain("invalid image");
  });
});
