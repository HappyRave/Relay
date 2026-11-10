// Editing a macro through the UI, with the real relay-core applying each
// edit: the result is checked in the macro's file on disk.
import { after, afterEach, before, beforeEach, describe, test } from "node:test";
import assert from "node:assert/strict";
import { App, until, waitingMacro, writeRly } from "./harness.mjs";

describe("editing", () => {
  const app = new App();
  let page;
  let id;
  before(async () => {
    page = await app.start();
    id = await page.store("view.id");
    await page.open(id);
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
  /** Shows step `i` in the inspector's Step tab, as a click on its row does. */
  const openStep = (i) => page.selectStep(i);
  const editField = (name, value) => page.setField(name, value);
  /** Deletes step `i` with the Step tab's Delete step. */
  const deleteStep = async (i) => {
    await openStep(i);
    await page.click("Delete step");
  };
  const BAR = { within: "header.bar" };

  test("Delete step deletes its row's events; Undo puts them back exactly; Redo deletes again", async () => {
    const original = disk();
    const n = (await steps()).length;
    await deleteStep(0);
    const afterDelete = await saved(original, "the delete");
    // The first row: the first click, with the moves folded into it.
    assert.ok((await steps()).length < n - 1);
    assert.ok(!(await steps()).some((s) => s.kind === "click" && s.t === 850), "the click is gone");
    assert.ok(afterDelete.events.length < original.events.length);
    assert.equal(await page.store("canUndo"), true);

    await page.click("Undo", BAR);
    await until(() => JSON.stringify(disk().events) === JSON.stringify(original.events), { what: "the undo on disk" });
    await synced("the undo");
    assert.equal((await steps()).length, n);
    assert.equal(await page.store("canRedo"), true);

    await page.click("Redo", BAR);
    await until(() => JSON.stringify(disk().events) === JSON.stringify(afterDelete.events), { what: "the redo on disk" });
    await synced("the redo");
    await page.click("Undo", BAR);
    await until(() => disk().events.length === original.events.length, { what: "the undo again" });
    await synced("the undo");
  });

  test("Ctrl + Z and Ctrl + Y work too", async () => {
    const original = disk();
    await deleteStep(0);
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

  test("Add → Wait inserts half a second at the playhead", async () => {
    const before = disk();
    await page.run(() => window.__relay.seek(1000));
    await until(async () => (await page.store("cur")) === 1000, { what: "the playhead" });
    await page.addStep("Wait");
    const after = await saved(before);
    const wait = after.events.find((e) => e.type === "wait" && e.label === "Inserted");
    assert.deepEqual(wait, { type: "wait", t: 1000, dur: 500, label: "Inserted" });
    // Everything after the playhead moved half a second later.
    const later = (evs) => evs.filter((e) => e.t > 1000 && e.type === "button").map((e) => e.t);
    assert.deepEqual(later(after.events), later(before.events).map((t) => t + 500));
  });

  test("Add → Wait for pixel inserts a check for the pixel under the macro's cursor, in its current color", async () => {
    const before = disk();
    // On the click at 850–910 ms (at 850 it's on the move that ends there).
    await page.run(() => window.__relay.seek(880));
    await until(async () => (await page.store("cur")) === 880, { what: "the playhead" });
    await page.addStep("Wait for pixel");
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

  test("the Step tab: a click's label", async () => {
    const before = disk();
    const i = (await steps()).findIndex((s) => s.kind === "click"); // after the move to it
    assert.ok(i > 0);
    await openStep(i);
    await editField("Label", "The File menu");
    const after = await saved(before);
    const first = (await steps())[i];
    assert.equal(first.label, "The File menu");
    assert.ok(after.events.some((e) => e.type === "button" && e.down && e.label === "The File menu"));
  });

  test("the Step tab: the pause before a step", async () => {
    const before = disk();
    const i = (await steps()).findIndex((s) => s.kind === "keys");
    await openStep(i);
    await editField("Pause before", "2.5");
    await saved(before);
    assert.equal((await steps())[i].pause, 2500);
  });

  test("the Step tab: a wait's duration", async () => {
    const before = disk();
    const i = (await steps()).findIndex((s) => s.kind === "wait" && s.label === "Dialog opens");
    await openStep(i);
    await editField("Waits", "1.25");
    const after = await saved(before);
    assert.equal(after.events.find((e) => e.type === "wait" && e.label === "Dialog opens").dur, 1250);
  });

  test("the Step tab: a pixel check's position, color, tolerance and timeout", async () => {
    const i = (await steps()).findIndex((s) => s.kind === "pixel_wait" && s.label === "Save button turns grey");
    await openStep(i);
    for (const [label, value] of [
      ["X", "1200"],
      ["Y", "600"],
      ["Color", "#aabbcc"],
      ["Tolerance", "20"],
      ["Gives up after", "2.5"],
    ]) {
      const before = disk();
      await editField(label, value);
      await saved(before, label);
    }
    const check = disk().events.find((e) => e.type === "pixel_wait" && e.label === "Save button turns grey");
    assert.deepEqual([check.x, check.y, check.color, check.tolerance, check.timeout_ms], [1200, 600, "#AABBCC", 20, 2500]);
  });

  describe("text steps", () => {
    /** Types `text` into the Text step's field and leaves it. */
    const setText = (text) => page.setField("Text", text);
    const status = () => page.run(() => document.querySelector(".inspector .hint[role=status]")?.textContent.trim() ?? null);

    test("Add → Text inserts a Text step at the playhead and saves what's typed in it, as a v3 file", async () => {
      const before = disk();
      assert.equal(before.version, 1);
      await page.run(() => window.__relay.seek(1000));
      await until(async () => (await page.store("cur")) === 1000, { what: "the playhead" });
      await page.addStep("Text");
      const inserted = await saved(before, "the insert");
      // After the step under the playhead (earlier tests added some here).
      const empty = inserted.events.find((e) => e.type === "text");
      assert.ok(empty.t >= 1000, `at ${empty.t}`);
      assert.deepEqual(empty, { type: "text", t: empty.t, dur: 0, text: "" });
      assert.equal(inserted.version, 3);
      // Its editor opened; it shows what the text types now, filled in by Rust.
      await until(async () => (await status()) === "Types nothing yet", { what: "the empty preview" });
      await setText("No. {n} on {date}");
      const after = await saved(inserted, "the text");
      const text = after.events.find((e) => e.type === "text");
      // 19 characters to type, 10 ms each.
      assert.deepEqual(text, { type: "text", t: empty.t, dur: 190, text: "No. {n} on {date}" });
      const shown = await until(async () => /^Types now: “No/.test((await status()) ?? "") && status(), { what: "the preview" });
      assert.match(shown, /^Types now: “No\. 1 on \d{4}-\d{2}-\d{2}”$/);
      // What follows moved back by the step's length.
      const later = (evs) => evs.filter((e) => e.type === "button" && e.t > empty.t).map((e) => e.t);
      assert.deepEqual(later(after.events), later(inserted.events).map((t) => t + 190));
    });

    test("a text with a mistake says what's wrong and isn't saved", async () => {
      const i = (await steps()).findIndex((s) => s.kind === "text");
      assert.ok(i >= 0, "the Text step from the test before");
      const before = disk();
      await openStep(i);
      await setText("Hello {name}");
      await until(async () => (await status())?.includes("{name} isn't a placeholder"), { what: "the mistake" });
      assert.equal(await status(), "{name} isn't a placeholder: use {date}, {time}, {clipboard}, {n} or {col:Name}.");
      await page.idle(500);
      assert.deepEqual(disk(), before);
    });

    test("Make editable turns typing into a Text step over the same time", async () => {
      const before = disk();
      const i = (await steps()).findIndex((s) => s.kind === "type");
      const typed = (await steps())[i];
      await openStep(i);
      await page.click("Make editable");
      const after = await saved(before);
      const made = (await steps())[i];
      assert.deepEqual([made.kind, made.t, made.end, made.text], ["text", typed.t, typed.end, typed.text]);
      assert.deepEqual(after.events.find((e) => e.type === "text" && e.t === typed.t), {
        type: "text",
        t: typed.t,
        dur: typed.end - typed.t,
        text: typed.text,
      });
      // Its key presses are gone; the rest keep their times.
      const keys = (m) => m.events.filter((e) => e.type === "key" && e.t >= typed.t && e.t <= typed.end);
      assert.equal(keys(after).length, 0);
      const outside = (m) => m.events.filter((e) => e.type !== "key" && e.type !== "text").map((e) => e.t);
      assert.deepEqual(outside(after), outside(before));
    });
  });

  test("Shorten pauses shortens exactly the pauses over a second, to a second, with an Undo", async () => {
    const pausesBefore = (await steps()).map((s) => s.pause);
    const long = pausesBefore.filter((p) => p > 1000).length;
    assert.ok(long > 0, "the keys step has a 2.5 s pause by now");
    assert.equal(await page.store("longPauses"), long);
    const before = disk();
    await page.menu("Steps options", "Shorten pauses");
    await saved(before);
    const pausesAfter = (await steps()).map((s) => s.pause);
    assert.deepEqual(pausesAfter, pausesBefore.map((p) => Math.min(p, 1000)));
    assert.equal(await page.store("toast.message"), `Shortened ${long} pause${long === 1 ? "" : "s"} to 1 s`);

    const trimmed = disk();
    await page.click("Undo", { within: ".toast" });
    await saved(trimmed, "the undo");
    assert.deepEqual((await steps()).map((s) => s.pause), pausesBefore);
    await page.click("Redo", BAR);
    await until(async () => JSON.stringify((await steps()).map((s) => s.pause)) === JSON.stringify(pausesAfter), { what: "the redo" });
  });

  test("a rejected edit is explained and changes nothing", async () => {
    const before = disk();
    await assert.rejects(page.invoke("edit_macro", { id, op: { op: "set_wait_duration", index: 999, dur: 1 } }), (e) => e.code === "edit_rejected");
    assert.deepEqual(disk(), before);
  });

  describe("play options (the inspector's Macro tab)", () => {
    const pb = () => disk().playback;
    const MACRO = { within: ".inspector" };
    beforeEach(() => page.inspector("macro", "play"));

    test("speed: 2×, 4×, 0.5× and back to 1×", async () => {
      for (const speed of [2, 4, 0.5, 1]) {
        await page.click(`${speed}×`, { role: "radio", ...MACRO });
        await until(() => pb().speed === speed, { what: `speed ${speed}` });
      }
    });

    test("repeat: a count, then Forever and back to the count", async () => {
      await page.setField("Repeat", "3");
      await until(() => pb().repeat.count === 3, { what: "3 times" });
      await page.click("Forever", MACRO);
      await until(() => pb().repeat === "forever", { what: "forever" });
      await page.click("Forever", MACRO);
      await until(() => pb().repeat.count === 3, { what: "back to 3" });
      await page.setField("Repeat", "1");
      await until(() => pb().repeat.count === 1, { what: "once" });
    });

    test("vary timing and by how much, where clicks land, stop on key", async () => {
      await page.click("Vary timing", { role: "switch" });
      await until(() => pb().humanize === false, { what: "humanize off" });
      await page.click("Vary timing", { role: "switch" });
      await until(() => pb().humanize === true, { what: "humanize on" });
      await page.setField("By up to", "85");
      await until(() => pb().jitter_ms === 85, { what: "jitter" });
      await page.click("Window", { role: "radio", within: '[aria-labelledby="m-coords"]' });
      await until(() => pb().coord_mode === "window", { what: "window coordinates" });
      await page.click("Stop on key", { role: "switch" });
      await until(() => pb().stop_on_key === false, { what: "stop on key off" });
      await page.click("Screen", { role: "radio", within: '[aria-labelledby="m-coords"]' });
      await until(() => pb().coord_mode === "screen", { what: "screen coordinates" });
    });
  });

  test("the Step tab stays on its step when an earlier step is deleted", async () => {
    const all = await steps();
    const i = all.findIndex((s, n) => n > 1 && s.kind === "click" && s.label);
    await openStep(i);
    const label = () => page.run(() => document.querySelector(".inspector #f-label")?.value);
    await until(async () => (await label()) === all[i].label, { what: "the Step tab on it" });
    const before = disk();
    await page.run(() => window.__relay.edit({ op: "delete_step", index: 0 }).then(() => true));
    await saved(before, "the delete");
    assert.equal(await page.store("selected"), i - 1, "which moved up one");
    await until(async () => (await label()) === all[i].label, { what: "the Step tab on the same step" });
  });

  describe("moves", () => {
    // A click at 100, 100, a zigzag with a bump in it to a click at 300, 100,
    // then the cursor leaving for Relay's Stop button. Only edited, never played.
    const zigzag = Array.from({ length: 20 }, (_, i) => ({
      type: "move",
      t: 100 + i * 16,
      x: 110 + i * 10,
      y: 100 + (i % 2 ? -3 : 3) + (i >= 8 && i <= 12 ? 40 : 0),
    }));
    const doc = {
      ...waitingMacro({ name: "Moves" }),
      events: [
        { type: "button", t: 0, x: 100, y: 100, btn: "Left", down: true },
        { type: "button", t: 60, x: 100, y: 100, btn: "Left", down: false },
        ...zigzag,
        { type: "move", t: 420, x: 300, y: 100 },
        { type: "button", t: 500, x: 300, y: 100, btn: "Left", down: true },
        { type: "button", t: 560, x: 300, y: 100, btn: "Left", down: false },
        { type: "move", t: 800, x: 500, y: 300 },
        { type: "move", t: 1000, x: 700, y: 400 },
        { type: "move", t: 1200, x: 900, y: 500 },
      ],
    };
    let moves;
    const file = () => app.macro(moves);
    const saves = async (before, what) => {
      await until(() => JSON.stringify(file()) !== JSON.stringify(before), { what });
      await until(async () => (await page.store("view.modified_at")) === file().modified_at, { what: `${what} in the UI` });
      return file();
    };
    const path = (m) => m.events.filter((e) => e.type === "move" && e.t > 60 && e.t < 500);

    before(async () => {
      const [p] = writeRly(app.path("import"), [doc]);
      [moves] = (await page.invoke("import_macros", { paths: [p] })).imported;
    });
    after(() => page.open(id));

    test("are steps: between the clicks, and after the last one", async () => {
      await page.open(moves);
      assert.deepEqual((await steps()).map((s) => s.kind), ["click", "move", "click", "move"]);
      assert.deepEqual(
        (await steps()).filter((s) => s.kind === "move").map((s) => [s.x, s.y, s.to_x, s.to_y, s.samples]),
        [
          [100, 100, 300, 100, 21],
          [300, 100, 900, 500, 3],
        ],
      );
      assert.equal(await page.store("view.duration"), 1200 + 500);
    });

    test("deleting the trip to the Stop button shortens the macro", async () => {
      await page.open(moves);
      const before = file();
      await deleteStep(3);
      const after = await saves(before, "the delete");
      assert.equal(after.events.at(-1).t, 560);
      assert.equal(after.events.length, before.events.length - 3);
      assert.equal(await page.store("view.duration"), 560 + 500);
    });

    test("Smooth takes the zigzag out, Straighten makes it a line; the times stay", async () => {
      await page.open(moves);
      await openStep(1);
      const before = file();
      await page.click("Smooth the path");
      const smoothed = await saves(before, "the smoothing");
      const ys = (m) => path(m).map((e) => e.y);
      const turns = (y) => y.slice(2).filter((c, i) => (y[i + 1] - y[i]) * (c - y[i + 1]) < 0).length;
      assert.ok(turns(ys(before)) > 15 && turns(ys(smoothed)) < 5 && Math.max(...ys(smoothed)) > 120, `no zigzag, the bump kept: ${ys(smoothed)}`);
      assert.deepEqual(path(smoothed).map((e) => e.t), path(before).map((e) => e.t));
      assert.deepEqual(path(smoothed).at(-1), path(before).at(-1), "it still ends on the click");
      assert.equal(await page.store("toast.message"), "Smoothed the move");
      await page.click("Straighten");
      const straight = await saves(smoothed, "the straightening");
      assert.deepEqual(ys(straight), Array(21).fill(100));
      await page.click("Undo", { within: ".toast" });
      await saves(straight, "the undo");
      assert.deepEqual(ys(file()), ys(smoothed));
    });

    test("a move's duration: shorter is faster, and what follows comes sooner", async () => {
      await page.open(moves);
      await openStep(1);
      const before = file();
      await editField("Moving there", "0.16");
      const after = await saves(before, "the duration");
      const ts = path(after).map((e) => e.t);
      assert.deepEqual([ts[0], ts.at(-1)], [100, 260]);
      assert.equal(after.events.find((e) => e.type === "button" && e.down && e.t > 60).t, 340);
    });
  });

  test("nothing can be edited while a macro plays", async () => {
    const [path] = writeRly(app.path("import"), [waitingMacro({ name: "Waits", waits: [3000, 3000] })]);
    const [waits] = (await page.invoke("import_macros", { paths: [path] })).imported;
    await page.open(waits);
    await page.click("Play");
    await page.waitMode("playing");
    assert.equal(await page.disabled("Add", { within: ".steps" }), true, "Add is disabled");
    for (const name of ["Undo", "Redo"]) assert.equal(await page.disabled(name, BAR), true, `${name} is disabled`);
    const before = app.macro(waits);
    await assert.rejects(page.invoke("edit_macro", { id: waits, op: { op: "delete_step", index: 0 } }), (e) => e.code === "busy");
    await assert.rejects(page.invoke("undo_edit", { id: waits, redo: false }), (e) => e.code === "busy");
    assert.deepEqual(app.macro(waits), before);
    await page.click("Stop");
    await page.waitMode("idle");
    assert.equal(await page.disabled("Add", { within: ".steps" }), false, "editable again once stopped");
    await page.open(id);
  });

  test("the undo history is per macro, and doesn't outlive the app", async () => {
    assert.equal(await page.store("canUndo"), true);
    const other = await page.run((id) => window.__relay.library.find((m) => m.id !== id && m.name !== "Waits" && m.name !== "Moves").id, id);
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
