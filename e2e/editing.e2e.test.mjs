// Editing a macro through the UI, with the real relay-core applying each
// edit: the result is checked in the macro's file on disk.
import { after, before, describe, test } from "node:test";
import assert from "node:assert/strict";
import { App, until } from "./harness.mjs";

describe("editing", () => {
  const app = new App();
  let page;
  let id;
  before(async () => {
    page = await app.start();
    id = await page.store("view.id");
  });
  after(() => app.dispose());

  const disk = () => app.macro(id);
  const steps = () => page.store("view.steps");
  /** Waits until the macro's file changes from `before`. */
  const saved = (before, what = "the save") =>
    until(() => JSON.stringify(disk()) !== JSON.stringify(before) && disk(), { what });
  /** Opens a step's editor by clicking its row. */
  const openStep = (i) =>
    page.run((i) => {
      document.querySelectorAll(".list .row")[i].click();
      return true;
    }, i);
  /** Sets the step editor's field whose label starts with `label`. */
  const editField = (label, value) =>
    page.run(
      (label, value) => {
        const l = [...document.querySelectorAll(".editor label")].find((l) => l.textContent.trim().startsWith(label));
        if (!l) throw new Error(`no field ${label}`);
        const input = l.querySelector("input");
        input.value = value;
        input.dispatchEvent(new Event("change", { bubbles: true }));
        return true;
      },
      label,
      value,
    );

  test("× on a step deletes its events; Undo puts them back exactly; Redo deletes again", async () => {
    const original = disk();
    const n = (await steps()).length;
    await page.click("Delete step");
    const afterDelete = await saved(original, "the delete");
    assert.equal((await steps()).length, n - 1);
    assert.ok(afterDelete.events.length < original.events.length);
    assert.equal(await page.store("canUndo"), true);

    await page.click("Undo");
    await until(() => JSON.stringify(disk().events) === JSON.stringify(original.events), { what: "the undo on disk" });
    assert.equal((await steps()).length, n);
    assert.equal(await page.store("canRedo"), true);

    await page.click("Redo");
    await until(() => JSON.stringify(disk().events) === JSON.stringify(afterDelete.events), { what: "the redo on disk" });
    await page.click("Undo");
    await until(() => disk().events.length === original.events.length, { what: "the undo again" });
  });

  test("Ctrl + Z and Ctrl + Y work too", async () => {
    const original = disk();
    await page.click("Delete step");
    await saved(original);
    const key = (k) => page.run((k) => window.dispatchEvent(new KeyboardEvent("keydown", { key: k, ctrlKey: true, bubbles: true })), k);
    await key("z");
    await until(() => disk().events.length === original.events.length, { what: "Ctrl + Z" });
    await key("y");
    await until(() => disk().events.length < original.events.length, { what: "Ctrl + Y" });
    await key("z");
    await until(() => disk().events.length === original.events.length, { what: "Ctrl + Z again" });
  });

  test("+ Wait inserts half a second at the playhead", async () => {
    const before = disk();
    await page.run(() => window.__relay.seek(1000));
    await page.idle();
    await page.click("+ Wait");
    const after = await saved(before);
    const wait = after.events.find((e) => e.type === "wait" && e.label === "Inserted");
    assert.deepEqual(wait, { type: "wait", t: 1000, dur: 500, label: "Inserted" });
    // Everything after the playhead moved half a second later.
    const later = (evs) => evs.filter((e) => e.t > 1000 && e.type === "button").map((e) => e.t);
    assert.deepEqual(later(after.events), later(before.events).map((t) => t + 500));
  });

  test("+ Pixel check inserts a check for the pixel under the macro's cursor, in its current color", async () => {
    const before = disk();
    await page.run(() => window.__relay.seek(850));
    await page.idle();
    await page.click("+ Pixel check");
    const after = await saved(before);
    const added = after.events.filter((e) => e.type === "pixel_wait").slice(before.events.filter((e) => e.type === "pixel_wait").length);
    const check = after.events.find((e) => e.type === "pixel_wait" && !before.events.some((b) => JSON.stringify(b) === JSON.stringify(e)));
    assert.ok(check && added.length === 1, "one new pixel check");
    // Inserts go just after the step under the playhead: the click at 850–910 ms.
    assert.equal(check.t, 911);
    assert.deepEqual([check.x, check.y, check.tolerance, check.timeout_ms], [134, 70, 8, 5000]);
    assert.match(check.color, /^#[0-9A-F]{6}$/);
  });

  test("the step editor: a click's label", async () => {
    const before = disk();
    await openStep(0);
    await editField("Label", "The File menu");
    const after = await saved(before);
    const first = (await steps())[0];
    assert.equal(first.label, "The File menu");
    assert.ok(after.events.some((e) => e.type === "button" && e.down && e.label === "The File menu"));
  });

  test("the step editor: the pause before a step", async () => {
    const before = disk();
    const i = (await steps()).findIndex((s) => s.kind === "keys");
    await openStep(i);
    await editField("Pause before", "2.5");
    await saved(before);
    assert.equal((await steps())[i].pause, 2500);
  });

  test("the step editor: a wait's duration", async () => {
    const before = disk();
    const i = (await steps()).findIndex((s) => s.kind === "wait" && s.label === "Dialog opens");
    await openStep(i);
    await editField("Duration", "1.25");
    const after = await saved(before);
    assert.equal(after.events.find((e) => e.type === "wait" && e.label === "Dialog opens").dur, 1250);
  });

  test("the step editor: a pixel check's position, color, tolerance and timeout", async () => {
    const i = (await steps()).findIndex((s) => s.kind === "pixel_wait" && s.label === "Save button turns grey");
    await openStep(i);
    for (const [label, value] of [
      ["X", "1200"],
      ["Y", "600"],
      ["Color", "#aabbcc"],
      ["Tolerance", "20"],
      ["Timeout", "2.5"],
    ]) {
      const before = disk();
      await editField(label, value);
      await saved(before, label);
    }
    const check = disk().events.find((e) => e.type === "pixel_wait" && e.label === "Save button turns grey");
    assert.deepEqual([check.x, check.y, check.color, check.tolerance, check.timeout_ms], [1200, 600, "#AABBCC", 20, 2500]);
  });

  test("Trim pauses shortens every pause over a second", async () => {
    // The keys step now has a 2.5 s pause.
    assert.ok((await page.store("longPauses")) > 0);
    const before = disk();
    await page.click("Trim pauses");
    await saved(before);
    const pauses = (await steps()).map((s) => s.pause);
    assert.ok(pauses.every((p) => p <= 1000), JSON.stringify(pauses));
    assert.match(await page.store("toast.message"), /^Shortened \d+ pauses? to 1 s$/);
  });

  test("a rejected edit is explained and changes nothing", async () => {
    const before = disk();
    await assert.rejects(page.invoke("edit_macro", { id, op: { op: "set_wait_duration", index: 999, dur: 1 } }), (e) => e.code === "edit_rejected");
    assert.deepEqual(disk(), before);
  });

  describe("playback options", () => {
    const pb = () => disk().playback;

    for (const [label, speed] of [
      ["2×", 2],
      ["4×", 4],
      ["0.5×", 0.5],
      ["1×", 1],
    ]) {
      test(`speed ${label}`, async () => {
        await page.click(label, { role: "radio" });
        await until(() => pb().speed === speed, { what: `speed ${speed}` });
      });
    }

    test("repeat: + and −, and Loop forever and back", async () => {
      const count = pb().repeat.count;
      await page.click("More repeats");
      await until(() => pb().repeat.count === count + 1, { what: "+" });
      await page.click("Fewer repeats");
      await until(() => pb().repeat.count === count, { what: "−" });
      await page.click("Loop forever");
      await until(() => pb().repeat === "forever", { what: "forever" });
      await page.click("Loop forever");
      await until(() => pb().repeat.count === count, { what: "back to the count" });
    });

    test("the Settings tab: humanize, jitter, coordinates, stop on key press", async () => {
      await page.click("Settings", { role: "tab" });
      await page.idle();
      await page.click("Humanize", { role: "switch" });
      await until(() => pb().humanize === false, { what: "humanize off" });
      await page.fill('input[aria-label="Jitter"]', "85");
      await until(() => pb().jitter_ms === 85, { what: "jitter" });
      await page.click("Window", { role: "radio" });
      await until(() => pb().coord_mode === "window", { what: "window coordinates" });
      await page.click("Stop on key press", { role: "switch" });
      await until(() => pb().stop_on_key === false, { what: "stop on key off" });
      await page.click("Screen", { role: "radio" });
      await until(() => pb().coord_mode === "screen", { what: "screen coordinates" });
    });
  });

  test("the undo history is per macro, and doesn't outlive the app", async () => {
    assert.equal(await page.store("canUndo"), true);
    await page.run(() => window.__relay.loadMacro(window.__relay.library[1].id));
    await page.idle();
    assert.equal(await page.store("canUndo"), false);
    await page.run((id) => window.__relay.loadMacro(id), id);
    await page.idle();
    assert.equal(await page.store("canUndo"), true);
    page = await app.restart();
    assert.equal(await page.store("canUndo"), false);
    // The edits themselves are kept.
    assert.equal((await steps())[0].label, "The File menu");
  });
});
