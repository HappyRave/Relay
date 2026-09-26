// Settings and the window against the real app: every setting reaches
// settings.json, Keep on top changes the native window, the compact player
// resizes it, and the close button hides or quits.
import { after, before, describe, test } from "node:test";
import assert from "node:assert/strict";
import { App, sleep, until } from "./harness.mjs";

describe("settings and window", () => {
  const app = new App();
  let page;
  before(async () => {
    page = await app.start();
    await page.click("Settings", { role: "tab" });
    await page.idle();
  });
  after(() => app.dispose());

  const settings = () => (app.exists("settings.json") ? app.json("settings.json") : {});
  const onTop = () => page.invoke("plugin:window|is_always_on_top", { label: "main" });
  const visible = () => page.invoke("plugin:window|is_visible", { label: "main" });

  for (const [label, key] of [
    ["Capture mouse path", "capture_moves"],
    ["Capture keystrokes", "capture_keys"],
    ["3-second countdown", "countdown"],
    ["Esc stops recording", "esc_stops_recording"],
    ["Ignore simulated input", "ignore_injected"],
    ["Click labels", "show_click_labels"],
    ["Close to tray", "close_to_tray"],
  ]) {
    test(`${label} is saved as ${key}`, async () => {
      await page.click(label, { role: "switch" });
      await until(() => settings()[key] === false, { what: `${key} off` });
      await page.click(label, { role: "switch" });
      await until(() => settings()[key] === true, { what: `${key} on` });
    });
  }

  test("Mouse path is saved as path_mode", async () => {
    await page.click("Trail only", { role: "radio" });
    await until(() => settings().path_mode === "trail", { what: "trail" });
    await page.click("Full path", { role: "radio" });
    await until(() => settings().path_mode === "full", { what: "full" });
  });

  test("Keep on top: Always, Never, and only during sessions", async () => {
    assert.equal(await onTop(), true, "always, by default");
    await page.click("Never", { role: "radio" });
    await until(async () => settings().keep_on_top === "never" && (await onTop()) === false, { what: "never" });
    await page.click("While recording or playing", { role: "radio" });
    await until(async () => settings().keep_on_top === "sessions", { what: "sessions" });
    assert.equal(await onTop(), false, "not while idle");

    // A recording's countdown is a session: on top for it, then back.
    await page.click("Record");
    await page.waitMode("countdown");
    await until(onTop, { what: "on top during the session" });
    await page.click("Stop recording");
    await page.waitMode("idle");
    await until(async () => (await onTop()) === false, { what: "off again" });

    await page.click("Always", { role: "radio" });
    await until(async () => settings().keep_on_top === "always" && (await onTop()) === true, { what: "always" });
  });

  test("settings survive a restart", async () => {
    await page.click("Click labels", { role: "switch" });
    await until(() => settings().show_click_labels === false);
    page = await app.restart();
    assert.equal(await page.store("settings.show_click_labels"), false);
    await page.run(() => window.__relay.updateSettings({ show_click_labels: true }));
  });

  test("an invalid settings file falls back to the defaults", async () => {
    await app.quit();
    app.writeSettings({ countdown: "maybe" });
    page = await app.start();
    assert.equal(await page.store("settings.countdown"), true);
    assert.equal(await page.store("settings.keep_on_top"), "always");
  });

  test("the compact player resizes the window, and it reopens compact", async () => {
    const size = () => page.run(() => [window.innerWidth, window.innerHeight]);
    const [w, h] = await size();
    await page.click("Compact player");
    await until(async () => (await size())[1] < h / 3, { what: "the window to shrink" });
    await until(() => app.json("window.json").expanded === false, { what: "window.json" });
    const [cw] = await size();
    assert.ok(cw < w, "narrower too");

    page = await app.restart();
    assert.equal(await page.store("expanded"), false);
    await page.click("Expand");
    await until(async () => (await size())[1] > h * 0.9, { what: "the window to grow" });
    await until(() => app.json("window.json").expanded === true, { what: "window.json" });
  });

  test("the widget keeps its bottom-center where it was", async () => {
    const anchor = () => app.json("window.json").anchor;
    await until(() => Array.isArray(anchor()), { what: "a saved anchor" });
    const before = anchor();
    await page.click("Compact player");
    await until(() => app.json("window.json").expanded === false);
    await sleep(300);
    const compact = anchor();
    assert.ok(Math.abs(compact[0] - before[0]) <= 2 && Math.abs(compact[1] - before[1]) <= 2, `${compact} vs ${before}`);
    await page.click("Expand");
    await until(() => app.json("window.json").expanded === true);
  });

  test("× with Close to tray hides Relay, which keeps running", async () => {
    assert.equal(await page.store("settings.close_to_tray"), true);
    await page.click("Hide to tray");
    await until(async () => (await visible()) === false, { what: "hidden" });
    assert.equal(app.proc.exitCode, null, "still running");
    await page.invoke("plugin:window|show", { label: "main" });
    await until(visible, { what: "shown again" });
  });

  test("× without Close to tray quits", async () => {
    await page.click("Settings", { role: "tab" });
    await page.idle();
    await page.click("Close to tray", { role: "switch" });
    await until(() => settings().close_to_tray === false);
    await page.click("Quit");
    const code = await Promise.race([app.exited, sleep(8000).then(() => "still running")]);
    assert.notEqual(code, "still running");
    app.proc = null;
    app.page = null;
  });

  test("Start with Windows shows what Windows has (not changed here: it writes the registry)", async () => {
    page = await app.start();
    assert.equal(typeof (await page.invoke("get_autostart")), "boolean");
  });
});
