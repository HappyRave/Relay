// Settings and the window against the real app: every setting reaches
// settings.json, Keep on top changes the native window, the compact player
// resizes it, the editor and its dividers resize and are remembered (and
// Relay's own resizes aren't taken for the user's), and the close button
// hides or quits.
import { after, afterEach, before, describe, test } from "node:test";
import assert from "node:assert/strict";
import { writeFileSync } from "node:fs";
import { App, sleep, until, waitingMacro, writeRly } from "./harness.mjs";

describe("settings and window", () => {
  const app = new App();
  let page;
  let waits; // a macro that only waits, safe to play
  before(async () => {
    page = await app.start();
    const [path] = writeRly(app.path("import"), [waitingMacro({ name: "Waits", waits: [2000, 2000] })]);
    [waits] = (await page.invoke("import_macros", { paths: [path] })).imported;
    await page.tab("Settings");
  });
  after(() => app.dispose());
  afterEach(() => app.page?.reset());

  const settings = () => (app.exists("settings.json") ? app.json("settings.json") : {});
  const onTop = () => page.invoke("plugin:window|is_always_on_top", { label: "main" });
  const visible = () => page.invoke("plugin:window|is_visible", { label: "main" });
  /** The native window's rect, in physical pixels. */
  const rect = async () => {
    const [pos, size] = await Promise.all([
      page.invoke("plugin:window|outer_position", { label: "main" }),
      page.invoke("plugin:window|outer_size", { label: "main" }),
    ]);
    return { x: pos.x, y: pos.y, w: size.width, h: size.height };
  };
  const bottomCenter = (r) => [r.x + r.w / 2, r.y + r.h];

  for (const [label, key] of [
    ["Capture mouse path", "capture_moves"],
    ["Capture keystrokes", "capture_keys"],
    ["Screenshot", "capture_screen"],
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

  test("Background is saved as preview_background", async () => {
    const bg = { role: "radio", within: '.list [aria-label="Background"]' };
    await page.click("Sketch", bg);
    await until(() => settings().preview_background === "sketch", { what: "sketch" });
    await page.click("Screen", bg);
    await until(() => settings().preview_background === "screen", { what: "screen" });
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
    await until(onTop, { what: "on top during the countdown" });
    await page.click("Stop recording");
    await page.waitMode("idle");
    await until(async () => (await onTop()) === false, { what: "off again" });

    // Playing, and paused, too.
    await page.open(waits);
    await page.click("Play");
    await page.waitMode("playing");
    await until(onTop, { what: "on top while playing" });
    await page.click("Pause");
    await page.waitMode("paused");
    assert.equal(await onTop(), true, "still on top while paused");
    await page.click("Stop");
    await page.waitMode("idle");
    await until(async () => (await onTop()) === false, { what: "off after playing" });
    await page.tab("Settings");

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
    // Relay resized the window itself, twice: that's not the user's size for the editor.
    assert.equal(app.json("window.json").size, null);
  });

  test("a resize that isn't the user's (no drag of an edge) isn't saved as the editor's size", async () => {
    await page.invoke("plugin:window|set_size", { label: "main", value: { Logical: { width: 900, height: 600 } } });
    await sleep(1200); // longer than the save after a resize
    assert.equal(app.json("window.json").size ?? null, null);
    await page.invoke("reset_layout");
  });

  test("switching to the compact player keeps the widget's bottom-center", async () => {
    const expanded = await rect();
    await page.click("Compact player");
    await until(async () => (await rect()).h < expanded.h / 3, { what: "the compact size" });
    const compact = await rect();
    const [ex, ey] = bottomCenter(expanded);
    const [cx, cy] = bottomCenter(compact);
    assert.ok(Math.abs(cx - ex) <= 2 && Math.abs(cy - ey) <= 2, `${cx},${cy} vs ${ex},${ey}`);
    await page.click("Expand");
    await until(async () => (await rect()).h === expanded.h, { what: "the expanded size" });
    assert.deepEqual(await rect(), expanded, "back exactly where it was");
  });

  test("near a screen edge, compact → expanded → compact doesn't drift", async () => {
    await page.click("Compact player");
    await until(async () => !(await page.store("expanded")) && (await rect()).h < 200, { what: "compact" });
    // Park the compact bar near the top-left corner (inside the margins), where the expanded
    // widget doesn't fit around the same bottom-center and has to be clamped.
    await page.invoke("plugin:window|set_position", { label: "main", value: { Physical: { x: 100, y: 200 } } });
    await sleep(1200);
    const parked = await rect();
    await page.click("Expand"); // too wide to stay centered there: clamped
    await until(async () => (await rect()).h > 200, { what: "expanded" });
    await page.click("Compact player");
    await until(async () => (await rect()).h < 200, { what: "compact again" });
    assert.deepEqual(await rect(), parked, "the compact bar is back where it was parked");
    await page.click("Expand");
    await until(async () => (await rect()).h > 200, { what: "expanded" });
  });

  const resizable = () => page.invoke("plugin:window|is_resizable", { label: "main" });
  const widthOf = (selector) => page.run((s) => Math.round(document.querySelector(s).getBoundingClientRect().width), selector);
  const heightOf = (selector) => page.run((s) => Math.round(document.querySelector(s).getBoundingClientRect().height), selector);
  /** Drags a divider by `delta` px along `axis`, with the page's own pointer events. */
  const dragDivider = (label, axis, delta) =>
    page.run(
      (label, axis, delta) => {
        const el = document.querySelector(`[role="separator"][aria-label="${label}"]`);
        const r = el.getBoundingClientRect();
        const at = { clientX: r.left + 1, clientY: r.top + 1 };
        const send = (type, d) =>
          el.dispatchEvent(new PointerEvent(type, { bubbles: true, button: 0, pointerId: 7, ...at, [axis]: at[axis] + d }));
        send("pointerdown", 0);
        send("pointermove", delta);
        send("pointerup", delta);
        return true;
      },
      label,
      axis,
      delta,
    );
  const panes = () => (app.exists("window.json") ? app.json("window.json").panes : undefined);

  test("the editor can be resized, the compact player can't", async () => {
    assert.equal(await resizable(), true);
    await page.click("Compact player");
    await until(async () => (await resizable()) === false, { what: "a fixed compact player" });
    await page.click("Expand");
    await until(resizable, { what: "a resizable editor again" });
  });

  test("the dividers are saved to window.json and survive a restart; double-click resets one", async () => {
    assert.equal(await widthOf(".preview"), 600, "the design's layout by default");
    const play = () => page.run(() => Math.round(document.querySelector('[aria-label="Play"]').getBoundingClientRect().height));
    const smallPlay = await play();
    assert.equal(await heightOf(".timeline-pane"), 146);
    await dragDivider("Resize preview", "clientX", -100);
    await until(() => panes()?.preview_w === 500, { what: "the preview's width saved" });
    await dragDivider("Resize timeline", "clientY", -60); // up: a taller timeline
    await until(() => panes()?.timeline_h === 206, { what: "the timeline's height saved" });
    await dragDivider("Resize buttons", "clientY", -20); // up: a taller button row, bigger buttons
    await until(() => panes()?.transport_h === 96, { what: "the button row's height saved" });
    assert.equal(await widthOf(".preview"), 500);
    assert.equal(await heightOf(".timeline-pane"), 206);
    assert.equal(await heightOf(".transport-pane"), 96);
    // A taller timeline has taller lanes.
    assert.ok((await heightOf(".lanes .lane")) > 26);
    // The narrower preview still shows the whole desktop, at the drawing's shape.
    const fits = await page.run(() => {
      const [x, y, w, h] = document.querySelector(".stage svg").getAttribute("viewBox").split(" ").map(Number);
      const stage = document.querySelector(".stage");
      const d = window.__relay.desktop;
      const inside = x <= d.x + 0.5 && y <= d.y + 0.5 && x + w >= d.x + d.w - 0.5 && y + h >= d.y + d.h - 0.5;
      return { inside, aspect: Math.abs(w / h - stage.clientWidth / stage.clientHeight) < 0.01 };
    });
    assert.deepEqual(fits, { inside: true, aspect: true });
    // The buttons are drawn bigger (by up to 96 / 76, as far as the row's width lets them).
    const bigPlay = await play();
    assert.ok(bigPlay > smallPlay && bigPlay <= Math.ceil((smallPlay * 96) / 76), `${smallPlay} → ${bigPlay}`);

    page = await app.restart();
    assert.deepEqual(await page.store("panes"), { preview_w: 500, transport_h: 96, timeline_h: 206 });
    await until(async () => (await widthOf(".preview")) === 500, { what: "the preview as it was left" });
    assert.equal(await heightOf(".timeline-pane"), 206);
    assert.equal(await play(), bigPlay);
    await page.run(() => {
      const el = document.querySelector('[role="separator"][aria-label="Resize buttons"]');
      el.dispatchEvent(new MouseEvent("dblclick", { bubbles: true }));
      return true;
    });
    await until(() => panes()?.transport_h === null, { what: "the button row reset" });
    await until(async () => (await play()) === smallPlay, { what: "the buttons back to their size" });

    await page.run(() => {
      const el = document.querySelector('[role="separator"][aria-label="Resize preview"]');
      el.dispatchEvent(new MouseEvent("dblclick", { bubbles: true }));
      return true;
    });
    await until(() => panes()?.preview_w === null, { what: "the preview's width reset" });
    assert.equal(await widthOf(".preview"), 600);
  });

  test("an editor made bigger keeps the preview's width and gives the rest to the side panel; Reset layout undoes it all", async () => {
    await app.quit();
    const saved = app.json("window.json");
    // 980 × 700 fits a 1024 × 768 screen (the CI runner's) without being shrunk.
    const layout = { expanded: true, size: [980, 700], panes: { preview_w: null, transport_h: null, timeline_h: 206 } };
    writeFileSync(app.path("window.json"), JSON.stringify({ ...saved, ...layout }));
    page = await app.start();
    try {
      const scale = await page.invoke("plugin:window|scale_factor", { label: "main" });
      const inner = await page.invoke("plugin:window|inner_size", { label: "main" });
      assert.deepEqual([inner.width, inner.height], [Math.round(980 * scale), Math.round(700 * scale)]);
      await until(async () => (await widthOf(".panel")) === 980 - 4 - 600 - 2, { what: "the side panel to take the room" });
      assert.equal(await widthOf(".preview"), 600);
      assert.equal(await heightOf(".timeline-pane"), 206, "the timeline as saved");
      // The preview row got the extra height: all but the header, the button row, the timeline and two dividers.
      const rest = 700 - 4 - 44 - 2 - (await heightOf(".transport-pane")) - 2 - 206;
      assert.equal(await heightOf(".preview"), rest);
      assert.ok(rest > 338);

      await page.tab("Settings");
      await page.click("Reset layout");
      await until(async () => (await page.invoke("plugin:window|inner_size", { label: "main" })).width === Math.round(944 * scale), {
        what: "the default size",
      });
      await until(() => app.json("window.json").size === null && panes().timeline_h === null && panes().preview_w === null, {
        what: "window.json reset",
      });
      await until(async () => (await heightOf(".timeline-pane")) === 146, { what: "the default timeline" });
    } finally {
      // Whatever happened, the tests after this one get the default layout.
      await page.invoke("reset_layout");
      await page.run(() => (window.__relay.movePanes({ preview_w: null, transport_h: null, timeline_h: null }), true));
    }
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
    await page.tab("Settings");
    await page.click("Close to tray", { role: "switch" });
    await until(() => settings().close_to_tray === false);
    await page.click("Quit");
    const code = await Promise.race([app.exited, sleep(8000).then(() => "still running")]);
    assert.equal(code, 0, "quit cleanly");
    app.proc = null;
    app.page = null;
  });
});
