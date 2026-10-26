// Sessions against the real engine and coordinator, and which of Relay's
// windows shows during them. The macros played here only wait (and check a
// pixel), so nothing is sent to the desktop.
import { after, afterEach, before, describe, test } from "node:test";
import assert from "node:assert/strict";
import { mkdirSync, renameSync, writeFileSync } from "node:fs";
import { App, sleep, until, waitingMacro, writeRly } from "./harness.mjs";

describe("playback and recording", () => {
  const app = new App();
  let page;
  const ids = {};

  before(async () => {
    page = await app.start();
    const docs = {
      short: waitingMacro({ name: "Short", waits: [400, 400] }),
      long: waitingMacro({ name: "Long", waits: [1000, 1000, 1000, 1000] }),
      loops: waitingMacro({ name: "Loops", waits: [300], repeat: { count: 3 } }),
      fast: waitingMacro({ name: "Fast", waits: [1000, 1000], speed: 4 }),
      rows: waitingMacro({ name: "Rows", waits: [300] }),
      // A color the screen won't have at (0, 0) with no tolerance.
      pixel: waitingMacro({ name: "Pixel", waits: [200], pixel: { x: 0, y: 0, color: "#FE01FD" } }),
    };
    const paths = writeRly(app.path("import"), Object.values(docs));
    const { imported } = await page.invoke("import_macros", { paths });
    Object.keys(docs).forEach((k, i) => (ids[k] = imported[i]));
    await page.run(() => window.__relay.refreshLibrary());
  });
  after(() => app.dispose());
  afterEach(() => app.page?.reset());

  const open = (id) => page.open(id);
  /** Which windows show: [the main window, the player]. */
  const shown = async () => [await app.visible("main"), await app.visible("player")];
  const showing = (want, what) => until(async () => JSON.stringify(await shown()) === JSON.stringify(want), { what });
  const runs = (id) => app.json("library.json").entries[id]?.runs ?? 0;

  test("Play runs the macro to the end and counts the run", async () => {
    await open(ids.short);
    const run = await page.playTimed();
    assert.ok(run.ms > 700 && run.ms < 1500, `an 0.8 s macro took ${run.ms} ms (${run.timing})`);
    assert.equal(run.finish, "completed");
    assert.equal(await page.store("cur"), await page.store("duration"), "stays at the end");
    await until(() => runs(ids.short) === 1, { what: "the run count on disk" });
    assert.ok(app.json("library.json").entries[ids.short].last_run);
    await until(async () => (await page.store("library")).find((m) => m.id === ids.short).runs === 1, { what: "the Library to update" });
  });

  test("Play at the end starts over", async () => {
    assert.equal(await page.store("cur"), await page.store("duration"));
    const run = await page.playTimed();
    assert.ok(run.from < 100, `started at ${run.from}`);
    assert.ok(run.ms > 650 && run.ms < 2000, `took ${run.ms} ms (${run.timing})`);
  });

  test("repeats: every loop runs", async () => {
    await open(ids.loops);
    const seen = new Set();
    await page.click("Play");
    await page.waitMode("playing");
    await until(
      async () => {
        seen.add(await page.store("loopIdx"));
        return (await page.store("mode")) === "idle";
      },
      { every: 30, what: "the loops to finish" },
    );
    assert.deepEqual([...seen].sort(), [0, 1, 2]);
    assert.equal(await page.store("lastFinish"), "completed");
  });

  test("the macro's speed applies (a 2 s macro at 4× takes about half a second)", async () => {
    await open(ids.fast);
    const run = await page.playTimed();
    assert.ok(run.ms > 350 && run.ms < 1200, `took ${run.ms} ms (${run.timing})`);
    assert.equal(run.finish, "completed");
  });

  test("Pause holds the playhead; Play resumes from it", async () => {
    await open(ids.long);
    await page.click("Play");
    await page.waitMode("playing");
    await sleep(600);
    await page.click("Pause");
    await page.waitMode("paused");
    const held = await page.store("cur");
    assert.ok(held > 300 && held < 1500, `paused at ${held}`);
    await sleep(700);
    assert.equal(await page.store("cur"), held);
    assert.equal(await page.store("view.id"), ids.long);
    await page.click("Play");
    await page.waitMode("playing");
    await until(async () => (await page.store("cur")) > held + 200, { what: "the playhead to move again" });
    await page.click("Stop");
    await page.waitMode("idle");
  });

  test("Stop ends it and rewinds", async () => {
    await page.click("Play");
    await page.waitMode("playing");
    await sleep(500);
    await page.click("Stop");
    await page.waitMode("idle");
    assert.equal(await page.store("lastFinish"), "stopped");
    assert.equal(await page.store("cur"), 0);
  });

  test("Play starts from the playhead", async () => {
    await page.run(() => window.__relay.seek(3200));
    await until(async () => (await page.store("cur")) === 3200, { what: "the playhead" });
    const run = await page.playTimed();
    assert.ok(Math.abs(run.from - 3200) < 100, `started at ${run.from}`);
    assert.ok(run.ms > 600 && run.ms < 1600, `only the last 0.8 s played (${run.ms} ms; ${run.timing})`);
  });

  test("seeking while playing jumps the engine there", async () => {
    await page.click("Play");
    await page.waitMode("playing");
    const started = Date.now();
    await page.run(() => window.__relay.seek(3700));
    await page.waitMode("idle");
    assert.ok(Date.now() - started < 2000);
    assert.equal(await page.store("lastFinish"), "completed");
  });

  test("changing the speed mid-playback applies at once", async () => {
    await page.run(() => window.__relay.seek(0));
    await page.click("Play");
    await page.waitMode("playing");
    const started = Date.now();
    // The play options can change while it plays.
    await page.inspector("macro", "play");
    await page.click("4×", { role: "radio", within: ".inspector" });
    await page.waitMode("idle");
    assert.ok(Date.now() - started < 2500, `took ${Date.now() - started} ms`);
    await page.click("1×", { role: "radio", within: ".inspector" });
    await until(async () => (await page.store("playback.speed")) === 1, { what: "back to 1×" });
  });

  test("a pixel check that never matches times out, stops, and stays on its step", async () => {
    await open(ids.pixel);
    await page.click("Play");
    await page.waitMode("playing");
    await page.waitMode("idle", 5000);
    assert.equal(await page.store("lastFinish"), "pixel_timeout");
    assert.match(await page.store("toast.message"), /^Pixel check timed out at step \d+; playback stopped\.$/);
    const steps = await page.store("view.steps");
    assert.equal(steps[await page.store("curStepIdx")].kind, "pixel_wait");
  });

  test("with a data file, the macro plays once per row", async () => {
    mkdirSync(app.path("data"), { recursive: true });
    const csv = app.path("data", "customers.csv");
    writeFileSync(csv, "Customer;Total\r\nACME;12,50\r\nGlobex;7\r\nInitech;3\r\n");
    await open(ids.rows);
    // Where the UI would open the file dialog.
    const info = await page.invoke("set_data_file", { id: ids.rows, path: csv });
    assert.deepEqual(info, { path: csv, columns: ["Customer", "Total"], rows: 3, error: null });
    assert.equal(app.json("library.json").entries[ids.rows].data_file, csv);
    await page.run(() => window.__relay.loadDataFile());
    await page.inspector("macro", "play");
    await until(async () => (await page.text(".inspector"))?.includes("customers.csv · 3 rows"), { what: "the Data file" });
    assert.match(await page.text("header.bar"), /Each row \(3\)/);

    const before = runs(ids.rows);
    const seen = new Set();
    const loops = new Set();
    await page.click("Play");
    await page.waitMode("playing");
    await until(
      async () => {
        seen.add(await page.store("loopIdx"));
        loops.add((await page.store("playInfo"))?.loops);
        return (await page.store("mode")) === "idle";
      },
      { every: 30, what: "the rows to play" },
    );
    assert.deepEqual([...seen].sort(), [0, 1, 2], "one loop per row, though Repeat is 1");
    assert.ok(loops.has(3) && !loops.has(1), `loops shown: ${[...loops]}`);
    assert.equal(await page.store("lastFinish"), "completed");
    await until(() => runs(ids.rows) === before + 1, { what: "the run count on disk" });
  });

  test("a data file that's gone refuses to play, and says so", async () => {
    await open(ids.rows);
    const csv = app.path("data", "customers.csv");
    renameSync(csv, `${csv}.moved`);
    try {
      const before = runs(ids.rows);
      await page.click("Play");
      await until(async () => (await page.store("toast.kind")) === "error", { what: "the error" });
      assert.equal(await page.store("toast.message"), "customers.csv isn't there anymore: choose it again in the macro's play options.");
      // Nothing played, so it's back to idle without a "finished".
      await page.waitMode("idle");
      await sleep(300);
      assert.equal(runs(ids.rows), before, "not a run");
    } finally {
      renameSync(`${csv}.moved`, csv);
    }
    await page.invoke("set_data_file", { id: ids.rows, path: null });
    assert.equal(app.json("library.json").entries[ids.rows].data_file, undefined);
  });

  test("during playback, the macro can't be deleted or switched", async () => {
    await open(ids.long);
    await page.click("Play");
    await page.waitMode("playing");
    await assert.rejects(page.invoke("delete_macro", { id: ids.long }), (e) => e.code === "busy");
    // Opening another macro is refused at once (the promise settles without switching).
    await page.run((id) => window.__relay.loadMacro(id), ids.short);
    assert.equal(await page.store("view.id"), ids.long);
    await page.click("Stop");
    await page.waitMode("idle");
  });

  test("recording: the window steps aside for the player, which counts down, then records; Stop brings the window back", async () => {
    await open(ids.short);
    await page.run(() => window.__relay.updateSettings({ countdown: true }));
    await page.click("Record a new macro");
    await page.waitMode("countdown");
    await showing([false, true], "the player instead of the window");
    // Both read in one go, in the player: the number shown matches the time left.
    const { left, digit } = await app.player.run(() => ({
      left: window.__relay.countLeft,
      digit: document.querySelector(".player .digit")?.textContent.trim(),
    }));
    assert.ok(left > 0 && left <= 3000, `${left} ms left`);
    assert.equal(digit, String(Math.ceil(left / 1000)));
    assert.match(await app.player.text(".screen"), /^GET READY/);
    await page.waitMode("recording", 5000);
    await until(async () => /^REC\s+New recording/.test((await app.player.text(".screen")) ?? ""), { what: "REC in the player" });
    await app.player.click("Stop recording");
    await page.waitMode("idle");
    await showing([true, false], "the window back");
  });

  test("recording without the countdown starts at once; Record during the countdown cancels it", async () => {
    await page.run(() => window.__relay.updateSettings({ countdown: false }));
    await page.click("Record a new macro");
    await page.waitMode("recording", 2000);
    await page.click("Stop recording");
    await page.waitMode("idle");
    await page.run(() => window.__relay.updateSettings({ countdown: true }));
    await page.click("Record a new macro");
    await page.waitMode("countdown");
    await page.click("Stop recording");
    await page.waitMode("idle");
    await sleep(3500);
    assert.equal(await page.store("mode"), "idle", "the cancelled countdown didn't start a recording");
  });

  test("Play is ignored while recording", async () => {
    await page.run(() => window.__relay.updateSettings({ countdown: false }));
    await page.click("Record a new macro");
    await page.waitMode("recording");
    await page.invoke("toggle_play", { from: 0 });
    await sleep(300);
    assert.equal(await page.store("mode"), "recording");
    await page.click("Stop recording");
    await page.waitMode("idle");
  });

  test("playing keeps the window where it is, unless Hide the window while playing is on", async () => {
    await open(ids.short);
    await page.click("Play");
    await page.waitMode("playing");
    assert.deepEqual(await shown(), [true, false]);
    await page.waitMode("idle");
    await page.run(() => window.__relay.updateSettings({ hide_while_playing: true }));
    try {
      await page.click("Play");
      await page.waitMode("playing");
      await showing([false, true], "the player instead");
      await page.waitMode("idle");
      await showing([true, false], "the window back");
    } finally {
      await page.run(() => window.__relay.updateSettings({ hide_while_playing: false }));
    }
  });

  test("player mode: the player stands in for the window, its keys drive the session, ⤢ brings the window back", async () => {
    await open(ids.short);
    await page.click("Switch to the player");
    await showing([false, true], "the player");
    assert.equal(app.json("window.json").player_mode, true, "remembered");
    assert.equal(await app.player.store("view.id"), ids.short, "on the window's macro");
    await app.player.click("Play");
    await page.waitMode("playing");
    await until(async () => /^PLAYING\s+Short/.test((await app.player.text(".screen")) ?? ""), { what: "PLAYING in the player" });
    await page.waitMode("idle");
    await app.player.click("Open the window");
    await showing([true, false], "the window");
    assert.equal(await page.store("place"), "macro");
    assert.equal(app.json("window.json").player_mode, false);
  });

  test("the player's menu switches macros, and the window follows", async () => {
    await page.invoke("set_player_mode", { on: true });
    await showing([false, true], "the player");
    await app.player.menu("Macro: Short. Switch macros", "Long");
    await until(async () => (await page.store("view.id")) === ids.long, { what: "the window on Long too" });
    assert.equal(await app.player.store("view.id"), ids.long);
    await page.invoke("set_player_mode", { on: false });
  });
});
