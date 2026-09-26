// Sessions against the real engine and coordinator. The macros played here
// only wait (and check a pixel), so nothing is sent to the desktop.
import { after, before, describe, test } from "node:test";
import assert from "node:assert/strict";
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
      // A color the screen won't have at (0, 0) with no tolerance.
      pixel: waitingMacro({ name: "Pixel", waits: [200], pixel: { x: 0, y: 0, color: "#FE01FD" } }),
    };
    const paths = writeRly(app.path("import"), Object.values(docs));
    const { imported } = await page.invoke("import_macros", { paths });
    Object.keys(docs).forEach((k, i) => (ids[k] = imported[i]));
    await page.run(() => window.__relay.refreshLibrary());
  });
  after(() => app.dispose());

  const open = async (id) => {
    await page.run((id) => window.__relay.loadMacro(id), id);
    await until(async () => (await page.store("view.id")) === id, { what: "the macro to open" });
  };
  const runs = (id) => app.json("library.json").entries[id]?.runs ?? 0;

  test("Play runs the macro to the end and counts the run", async () => {
    await open(ids.short);
    const started = Date.now();
    await page.click("Play");
    await page.waitMode("playing");
    await page.waitMode("idle");
    const took = Date.now() - started;
    assert.ok(took >= 700 && took < 3000, `took ${took} ms`);
    assert.equal(await page.store("lastFinish"), "completed");
    assert.equal(await page.store("cur"), await page.store("duration"), "stays at the end");
    await until(() => runs(ids.short) === 1, { what: "the run count on disk" });
    assert.ok(app.json("library.json").entries[ids.short].last_run);
    await until(async () => (await page.store("library")).find((m) => m.id === ids.short).runs === 1, { what: "the Library tab to update" });
  });

  test("Play at the end starts over", async () => {
    await page.click("Play");
    await page.waitMode("playing");
    assert.ok((await page.store("cur")) < 400);
    await page.waitMode("idle");
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
    const started = Date.now();
    await page.click("Play");
    await page.waitMode("playing");
    await page.waitMode("idle");
    const took = Date.now() - started;
    assert.ok(took < 1500, `took ${took} ms`);
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
    await page.idle();
    const started = Date.now();
    await page.click("Play");
    await page.waitMode("playing");
    await page.waitMode("idle");
    assert.ok(Date.now() - started < 2000, "only the last 0.8 s played");
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
    await page.click("4×", { role: "radio" });
    await page.waitMode("idle");
    assert.ok(Date.now() - started < 2500, `took ${Date.now() - started} ms`);
    await page.click("1×", { role: "radio" });
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

  test("during playback, the macro can't be deleted or switched", async () => {
    await open(ids.long);
    await page.click("Play");
    await page.waitMode("playing");
    await assert.rejects(page.invoke("delete_macro", { id: ids.long }), (e) => e.code === "busy");
    await page.run((id) => window.__relay.loadMacro(id), ids.short);
    await page.idle();
    assert.equal(await page.store("view.id"), ids.long);
    await page.click("Stop");
    await page.waitMode("idle");
  });

  test("recording: the countdown, then recording, then stop", async () => {
    await page.run(() => window.__relay.updateSettings({ countdown: true }));
    await page.click("Record");
    await page.waitMode("countdown");
    const left = await page.store("countLeft");
    assert.ok(left > 0 && left <= 3000, `${left} ms left`);
    assert.equal(await page.text(".countdown"), String(Math.ceil(left / 1000)));
    await page.waitMode("recording", 5000);
    assert.equal(await page.store("recording"), true);
    await until(async () => (await page.store("cur")) > 200, { what: "the recording clock" });
    assert.match(await page.text(".transport"), /of recording/);
    await page.click("Stop recording");
    await page.waitMode("idle");
  });

  test("recording without the countdown starts at once; Record during the countdown cancels it", async () => {
    await page.run(() => window.__relay.updateSettings({ countdown: false }));
    await page.click("Record");
    await page.waitMode("recording", 2000);
    await page.click("Stop recording");
    await page.waitMode("idle");
    await page.run(() => window.__relay.updateSettings({ countdown: true }));
    await page.click("Record");
    await page.waitMode("countdown");
    await page.click("Stop recording");
    await page.waitMode("idle");
    await sleep(3500);
    assert.equal(await page.store("mode"), "idle", "the cancelled countdown didn't start a recording");
  });

  test("Play is ignored while recording", async () => {
    await page.run(() => window.__relay.updateSettings({ countdown: false }));
    await page.click("Record");
    await page.waitMode("recording");
    await page.invoke("toggle_play", { from: 0 });
    await sleep(300);
    assert.equal(await page.store("mode"), "recording");
    await page.click("Stop recording");
    await page.waitMode("idle");
  });

  test("the compact player's buttons drive the same session", async () => {
    await open(ids.short);
    await page.click("Compact player");
    await until(() => page.run(() => !!document.querySelector(".compact")), { what: "the compact player" });
    await page.click("Play");
    await page.waitMode("playing");
    assert.match(await page.text(".compact .badge"), /playing/i);
    await page.waitMode("idle");
    await page.click("Expand");
  });
});
