// Editing a macro through the UI, with the real relay-core applying each
// edit: the result is checked in the macro's file on disk.
import { after, afterEach, before, describe, test } from "node:test";
import assert from "node:assert/strict";
import { App, until, waitingMacro, writeRly } from "./harness.mjs";

describe("editing", () => {
  const app = new App();
  let page;
  let id;
  before(async () => {
    page = await app.start();
    id = await page.store("view.id");
  });
  after(() => app.dispose());
  afterEach(() => app.page?.reset());

  const disk = () => app.macro(id);
  const steps = () => page.store("view.steps");
  /**
   * Waits until the macro's file changes from `before`, and the UI shows that
   * same version (Rust writes the file just before the UI gets its answer).
   */
  const saved = async (before, what = "the save") => {
    const after = await until(() => JSON.stringify(disk()) !== JSON.stringify(before) && disk(), { what });
    await synced(what);
    return after;
  };
  /** Waits until the UI shows the version of the macro that's on disk. */
  const synced = (what = "the UI") =>
    until(async () => (await page.store("view.modified_at")) === disk().modified_at, { what: `${what} in the UI` });
  /** Opens a step's editor by clicking its row (in the Steps tab). */
  const openStep = async (i) => {
    await page.tab("Steps");
    await page.run((i) => {
      document.querySelectorAll('[role="tabpanel"] .list .row')[i].click();
      return true;
    }, i);
  };
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
    await page.click("Delete step", { nth: 0 });
    const afterDelete = await saved(original, "the delete");
    assert.equal((await steps()).length, n - 1);
    assert.ok(afterDelete.events.length < original.events.length);
    assert.equal(await page.store("canUndo"), true);

    await page.click("Undo", { within: ".header" });
    await until(() => JSON.stringify(disk().events) === JSON.stringify(original.events), { what: "the undo on disk" });
    await synced("the undo");
    assert.equal((await steps()).length, n);
    assert.equal(await page.store("canRedo"), true);

    await page.click("Redo", { within: ".header" });
    await until(() => JSON.stringify(disk().events) === JSON.stringify(afterDelete.events), { what: "the redo on disk" });
    await synced("the redo");
    await page.click("Undo", { within: ".header" });
    await until(() => disk().events.length === original.events.length, { what: "the undo again" });
    await synced("the undo");
  });

  test("Ctrl + Z and Ctrl + Y work too", async () => {
    const original = disk();
    await page.click("Delete step", { nth: 0 });
    await saved(original);
    // Each shortcut only works once the UI knows there's something to undo or redo.
    const key = async (k, ready) => {
      await until(() => page.store(ready), { what: `${ready} before Ctrl + ${k.toUpperCase()}` });
      await page.run((k) => window.dispatchEvent(new KeyboardEvent("keydown", { key: k, ctrlKey: true, bubbles: true })), k);
    };
    await key("z", "canUndo");
    await until(() => disk().events.length === original.events.length, { what: "Ctrl + Z" });
    await synced("Ctrl + Z");
    await key("y", "canRedo");
    await until(() => disk().events.length < original.events.length, { what: "Ctrl + Y" });
    await synced("Ctrl + Y");
    await key("z", "canUndo");
    await until(() => disk().events.length === original.events.length, { what: "Ctrl + Z again" });
    await synced("Ctrl + Z");
  });

  test("+ Wait inserts half a second at the playhead", async () => {
    const before = disk();
    await page.run(() => window.__relay.seek(1000));
    await until(async () => (await page.store("cur")) === 1000, { what: "the playhead" });
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
    await until(async () => (await page.store("cur")) === 850, { what: "the playhead" });
    await page.click("+ Pixel check");
    const after = await saved(before);
    const checks = (m) => m.events.filter((e) => e.type === "pixel_wait");
    assert.equal(checks(after).length, checks(before).length + 1, "one new pixel check");
    // The new one has no label (the macro's own checks do).
    const check = checks(after).find((e) => !e.label);
    // It goes after the step under the playhead (the click at 850–910 ms), before the next one…
    const nextPress = before.events.find((e) => e.type === "button" && e.down && e.t > 910);
    assert.ok(check.t > 910 && check.t < nextPress.t + check.dur, `at ${check.t}`);
    // …and everything after it moves later by its length.
    const later = (evs, shift) => evs.filter((e) => e.type === "button" && e.t > 910).map((e) => e.t - shift);
    assert.deepEqual(later(after.events, check.dur), later(before.events, 0));
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

  test("Trim pauses shortens exactly the pauses over a second, to a second, with an Undo", async () => {
    const pausesBefore = (await steps()).map((s) => s.pause);
    const long = pausesBefore.filter((p) => p > 1000).length;
    assert.ok(long > 0, "the keys step has a 2.5 s pause by now");
    assert.equal(await page.store("longPauses"), long);
    const before = disk();
    await page.click("Trim pauses");
    await saved(before);
    const pausesAfter = (await steps()).map((s) => s.pause);
    assert.deepEqual(pausesAfter, pausesBefore.map((p) => Math.min(p, 1000)));
    assert.equal(await page.store("toast.message"), `Shortened ${long} pause${long === 1 ? "" : "s"} to 1 s`);

    const trimmed = disk();
    await page.click("Undo", { within: ".toast" });
    await saved(trimmed, "the undo");
    assert.deepEqual((await steps()).map((s) => s.pause), pausesBefore);
    await page.click("Redo", { within: ".header" });
    await until(async () => JSON.stringify((await steps()).map((s) => s.pause)) === JSON.stringify(pausesAfter), { what: "the redo" });
  });

  test("a rejected edit is explained and changes nothing", async () => {
    const before = disk();
    await assert.rejects(page.invoke("edit_macro", { id, op: { op: "set_wait_duration", index: 999, dur: 1 } }), (e) => e.code === "edit_rejected");
    assert.deepEqual(disk(), before);
  });

  describe("playback options", () => {
    const pb = () => disk().playback;

    // At the default size the control bar is in its narrow mode: one Speed button that cycles.
    test("speed: the button steps through 2×, 4×, 0.5× and back to 1×", async () => {
      for (const [from, to] of [
        [1, 2],
        [2, 4],
        [4, 0.5],
        [0.5, 1],
      ]) {
        await page.click(`Speed ${from}×`);
        await until(() => pb().speed === to, { what: `speed ${to}` });
      }
    });

    test("repeat, folded at the default size: the button steps through the counts and forever", async () => {
      const start = pb().repeat;
      await page.invoke("set_playback_options", { id: await page.store("view.id"), options: { ...pb(), repeat: { count: 3 } } });
      await page.run(async () => (await window.__relay.loadMacro(window.__relay.view.id), true));
      await page.click("Repeat 3 times");
      await until(() => pb().repeat.count === 5, { what: "5" });
      await page.click("Repeat 5 times");
      await page.click("Repeat 10 times");
      await until(() => pb().repeat === "forever", { what: "forever" });
      await page.click("Repeat forever");
      await until(() => pb().repeat.count === 1, { what: "back to 1" });
      await page.invoke("set_playback_options", { id: await page.store("view.id"), options: { ...pb(), repeat: start } });
      await page.run(async () => (await window.__relay.loadMacro(window.__relay.view.id), true));
    });

    test("repeat in a wider bar: + and −, and Loop forever and back", async () => {
      // 1000 px: the bar's mid mode (and it fits a 1024 px screen).
      await page.invoke("plugin:window|set_size", { label: "main", value: { Logical: { width: 1000, height: 612 } } });
      try {
        await until(() => page.run(() => !!document.querySelector('[aria-label="More repeats"]')), { what: "the mid bar" });
        const count = pb().repeat.count;
        await page.click("More repeats");
        await until(() => pb().repeat.count === count + 1, { what: "+" });
        await page.click("Fewer repeats");
        await until(() => pb().repeat.count === count, { what: "−" });
        await page.click("Loop forever");
        await until(() => pb().repeat === "forever", { what: "forever" });
        await page.click("Loop forever");
        await until(() => pb().repeat.count === count, { what: "back to the count" });
      } finally {
        await page.invoke("reset_layout");
      }
    });

    test("the Settings tab: humanize, jitter, coordinates, stop on key press", async () => {
      await page.tab("Settings");
      await page.click("Humanize", { role: "switch" });
      await until(() => pb().humanize === false, { what: "humanize off" });
      await page.fill('input[aria-label="Jitter"]', "85");
      await until(() => pb().jitter_ms === 85, { what: "jitter" });
      await page.click("Window", { role: "radio", within: '[aria-label="Coordinates"]' });
      await until(() => pb().coord_mode === "window", { what: "window coordinates" });
      await page.click("Stop on key press", { role: "switch" });
      await until(() => pb().stop_on_key === false, { what: "stop on key off" });
      await page.click("Screen", { role: "radio", within: '[aria-label="Coordinates"]' });
      await until(() => pb().coord_mode === "screen", { what: "screen coordinates" });
    });
  });

  test("the step editor stays on its step when an earlier step is deleted", async () => {
    const all = await steps();
    const i = all.findIndex((s, n) => n > 1 && s.kind === "click" && s.label);
    await openStep(i);
    const label = () =>
      page.run(() => [...document.querySelectorAll(".editor label")].find((l) => l.textContent.trim().startsWith("Label"))?.querySelector("input").value);
    await until(async () => (await label()) === all[i].label, { what: "the editor open on it" });
    const before = disk();
    await page.run(() => {
      document.querySelector('[role="tabpanel"] .list .row .del').click(); // step 0's ×
      return true;
    });
    await saved(before, "the delete");
    await until(async () => (await label()) === all[i].label, { what: "the editor on the same step" });
    assert.equal((await steps())[i - 1].label, all[i].label, "which moved up one row");
  });

  test("nothing can be edited while a macro plays", async () => {
    const [path] = writeRly(app.path("import"), [waitingMacro({ name: "Waits", waits: [3000, 3000] })]);
    const [waits] = (await page.invoke("import_macros", { paths: [path] })).imported;
    await page.open(waits);
    await page.click("Play");
    await page.waitMode("playing");
    for (const name of ["+ Wait", "+ Pixel check", "Trim pauses", "Undo", "Redo"]) {
      assert.equal(await page.disabled(name), true, `${name} is disabled`);
    }
    const before = app.macro(waits);
    await assert.rejects(page.invoke("edit_macro", { id: waits, op: { op: "delete_step", index: 0 } }), (e) => e.code === "busy");
    await assert.rejects(page.invoke("undo_edit", { id: waits, redo: false }), (e) => e.code === "busy");
    assert.deepEqual(app.macro(waits), before);
    await page.click("Stop");
    await page.waitMode("idle");
    assert.equal(await page.disabled("+ Wait"), false, "editable again once stopped");
    await page.open(id);
  });

  test("the undo history is per macro, and doesn't outlive the app", async () => {
    assert.equal(await page.store("canUndo"), true);
    const other = await page.run((id) => window.__relay.library.find((m) => m.id !== id && m.name !== "Waits").id, id);
    await page.open(other);
    assert.equal(await page.store("canUndo"), false, "another macro has its own (empty) history");
    await page.open(id);
    assert.equal(await page.store("canUndo"), true, "and this one's is still there");
    const edited = await steps();
    page = await app.restart();
    await page.open(id); // the Library's first macro opens at startup
    assert.equal(await page.store("canUndo"), false);
    assert.deepEqual(await steps(), edited, "the edits themselves are kept");
  });
});
