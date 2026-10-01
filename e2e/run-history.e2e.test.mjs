// The run history against the real app: what runs.json records for each
// way a playback ends and for its pixel checks, the Library's Runs view, and
// the file across restarts. The macros only wait and check pixels, so
// nothing is sent to the desktop. Triggered runs are checked in the
// triggers suite.
import { after, afterEach, before, describe, test } from "node:test";
import assert from "node:assert/strict";
import { writeFileSync } from "node:fs";
import { App, until, waitingMacro, writeRly } from "./harness.mjs";

describe("run history", () => {
  const app = new App();
  let page;
  const ids = {};

  /** A patch of Relay's own window in `color`, and the screen pixel at its center. */
  async function patch(color) {
    const pos = await page.invoke("plugin:window|inner_position", { label: "main" });
    const at = await page.run((color) => {
      let el = document.getElementById("e2e-patch");
      if (!el) {
        el = document.createElement("div");
        el.id = "e2e-patch";
        Object.assign(el.style, { position: "fixed", left: "40px", top: "300px", width: "40px", height: "40px", zIndex: 99999 });
        document.body.append(el);
      }
      el.style.background = color;
      const r = el.getBoundingClientRect();
      const k = window.devicePixelRatio;
      return { x: Math.round((r.left + r.width / 2) * k), y: Math.round((r.top + r.height / 2) * k) };
    }, color);
    return { x: pos.x + at.x, y: pos.y + at.y };
  }

  before(async () => {
    page = await app.start();
    const { x, y } = await patch("#12C34E");
    await until(async () => (await page.invoke("sample_pixel", { x, y })) === "#12C34E", { what: "the patch on screen" });
    const docs = {
      short: waitingMacro({ name: "Short", waits: [400, 400], repeat: { count: 2 } }),
      long: waitingMacro({ name: "Long", waits: [1000, 1000, 1000, 1000] }),
      matches: waitingMacro({ name: "Matches", waits: [200], pixel: { x, y, color: "#12C34E" } }),
      // A color the screen won't have at (0, 0) with no tolerance.
      never: waitingMacro({ name: "Never", waits: [200], pixel: { x: 0, y: 0, color: "#FE01FD" } }),
    };
    const paths = writeRly(app.path("import"), Object.values(docs));
    const { imported } = await page.invoke("import_macros", { paths });
    Object.keys(docs).forEach((k, i) => (ids[k] = imported[i]));
    await page.run(() => window.__relay.refreshLibrary());
  });
  after(() => app.dispose());
  afterEach(() => app.page?.reset());

  const onDisk = () => (app.exists("runs.json") ? app.json("runs.json").entries : []);
  /** Plays `id` from the Play button until idle; returns the entry it added (runs.json keeps the oldest first). */
  const played = async (id, { stopAfter } = {}) => {
    const before = onDisk().length;
    await page.open(id);
    if (stopAfter == null) {
      await page.playTimed();
    } else {
      await page.click("Play");
      await page.waitMode("playing");
      await new Promise((r) => setTimeout(r, stopAfter));
      await page.click("Stop");
      await page.waitMode("idle");
    }
    await until(() => onDisk().length === before + 1, { what: "the run in runs.json" });
    return onDisk().at(-1);
  };

  test("a fresh data folder has no runs, and no runs.json until one", async () => {
    assert.deepEqual(await page.invoke("list_runs"), []);
    assert.equal(app.exists("runs.json"), false);
  });

  test("a completed run is recorded with when, how and how long", async () => {
    const startedAt = Date.now();
    const e = await played(ids.short);
    assert.equal(app.json("runs.json").version, 1);
    assert.equal(e.macro_id, ids.short);
    assert.equal(e.macro_name, "Short");
    assert.equal(e.source, "manual");
    assert.deepEqual(e.outcome, { type: "finished", reason: "completed" });
    assert.equal(e.loops, 2);
    assert.ok(e.duration_ms > 1400 && e.duration_ms < 4000, `two 0.8 s loops took ${e.duration_ms} ms`);
    const at = new Date(e.at).getTime();
    assert.ok(at >= startedAt - 1000 && at <= Date.now(), `started at ${e.at}`);
    assert.deepEqual([e.from_ms, e.speed, e.humanize, e.checks, e.checks_dropped], [0, 1, false, [], 0]);
    assert.deepEqual((await page.invoke("list_runs"))[0], e);
  });

  test("a matched pixel check is recorded with its step and how long it waited", async () => {
    const e = await played(ids.matches);
    assert.deepEqual(e.outcome, { type: "finished", reason: "completed" });
    assert.equal(app.macro(ids.matches).events.at(-1).type, "pixel_wait");
    assert.equal(e.checks.length, 1);
    const [c] = e.checks;
    assert.deepEqual([c.step, c.loop_idx, c.image, c.outcome], [2, 0, false, { type: "matched" }]);
    assert.ok(c.after_ms < 500, `matched at once, after ${c.after_ms} ms`);
  });

  test("a pixel check that times out is recorded as the reason the run ended", async () => {
    const e = await played(ids.never);
    assert.deepEqual(e.outcome, { type: "finished", reason: "pixel_timeout" });
    assert.deepEqual(
      e.checks.map((c) => [c.step, c.outcome]),
      [[2, { type: "timed_out" }]],
    );
    assert.ok(e.checks[0].after_ms >= 800 && e.checks[0].after_ms < 1500, `timed out after ${e.checks[0].after_ms} ms`);
  });

  test("a stopped run is recorded as stopped, with how long it played", async () => {
    const e = await played(ids.long, { stopAfter: 700 });
    assert.deepEqual(e.outcome, { type: "finished", reason: "stopped" });
    assert.ok(e.duration_ms >= 600 && e.duration_ms < 3000, `played ${e.duration_ms} ms`);
  });

  test("Library → Runs lists them newest first; a run opens to show its checks", async () => {
    await page.tab("Library");
    await page.click("Runs");
    await until(async () => (await page.run(() => document.querySelectorAll(".runs .head").length)) === 4, { what: "the runs" });
    const rows = await page.run(() => [...document.querySelectorAll(".runs .head")].map((r) => r.innerText.replace(/\s+/g, " ").trim()));
    assert.match(rows[0], /^Long Today, \d\d:\d\d Play · Stopped \d+\.\d s$/);
    assert.match(rows[1], /^Never Today, \d\d:\d\d Play · Pixel check timed out \d+\.\d s$/);
    assert.match(rows[2], /^Matches Today, \d\d:\d\d Play · Completed \d+\.\d s$/);
    assert.match(rows[3], /^Short Today, \d\d:\d\d Play · Completed · 2 loops \d+\.\d s$/);
    await page.run(() => document.querySelectorAll(".runs .head")[1].click());
    await until(async () => /Step 2 · Pixel check timed out after \d\.\d s/.test((await page.text(".runs .details")) ?? ""), {
      what: "the check",
    });
    // A new run shows up while the view is open.
    await page.open(ids.short);
    await page.tab("Library");
    await page.playTimed();
    await until(async () => (await page.run(() => document.querySelectorAll(".runs .head").length)) === 5, { what: "the new run" });
    await page.click("Macros");
    await until(async () => !(await page.store("runsOpen")), { what: "back to the macros" });
  });

  test("the history survives a restart", async () => {
    const before = await page.invoke("list_runs");
    page = await app.restart();
    assert.deepEqual(await page.invoke("list_runs"), before);
  });

  test("a damaged runs.json is set aside and reported, and a new one is started", async () => {
    await app.quit();
    writeFileSync(app.path("runs.json"), "{ not json");
    page = await app.start();
    await until(async () => /runs\.json\.bad/.test((await page.store("toast.message")) ?? ""), { what: "the error" });
    assert.equal(app.exists("runs.json"), false);
    assert.deepEqual(await page.invoke("list_runs"), []);
    await page.run(() => window.__relay.refreshLibrary());
    const e = await played(ids.short);
    assert.equal(e.macro_name, "Short");
    assert.equal(app.json("runs.json").entries.length, 1);
  });
});
