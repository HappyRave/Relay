// Settings and the windows against the real app: every setting reaches
// settings.json, Relay's keys are set from Settings → Keys, Keep the player
// on top and the player's size change the native player, the editor's
// divider and player mode are remembered (and Relay's own resizes aren't
// taken for the user's), the close button hides or quits, and the first run
// shows for a new install, and its keys screen once after an upgrade.
import { after, afterEach, before, describe, test } from "node:test";
import assert from "node:assert/strict";
import { App, KEYS, sleep, until, waitingMacro, writeRly } from "./harness.mjs";

describe("settings and windows", () => {
  const app = new App();
  let page;
  let waits; // a macro that only waits, safe to play
  before(async () => {
    page = await app.start();
    const [path] = writeRly(app.path("import"), [waitingMacro({ name: "Waits", waits: [2000, 2000] })]);
    [waits] = (await page.invoke("import_macros", { paths: [path] })).imported;
  });
  after(() => app.dispose());
  afterEach(() => app.page?.reset());

  const settings = () => (app.exists("settings.json") ? app.json("settings.json") : {});
  /** Settings, on one of its sections. */
  const section = async (name) => {
    await page.go("settings");
    await page.click(name, { within: 'nav[aria-label="Settings sections"]' });
    await until(() => page.run((name) => document.querySelector('[aria-current="page"]')?.textContent.trim() === name, name), {
      what: `the ${name} section`,
    });
  };
  const playerOnTop = () => page.invoke("plugin:window|is_always_on_top", { label: "player" });
  const visible = (label) => app.visible(label);
  const size = (label) => page.invoke("plugin:window|inner_size", { label });

  for (const [where, label, key] of [
    ["Recording", "Capture the mouse path", "capture_moves"],
    ["Recording", "Capture keystrokes", "capture_keys"],
    ["Recording", "Screenshot", "capture_screen"],
    ["Recording", "Count down 3, 2, 1", "countdown"],
    ["Recording", "Esc stops recording", "esc_stops_recording"],
    ["Recording", "Ignore simulated input", "ignore_injected"],
    ["Window", "Hide the window while recording", "hide_while_recording"],
    ["Window", "Close to the tray", "close_to_tray"],
    ["Appearance", "Show click labels", "show_click_labels"],
  ]) {
    test(`${where}: ${label} is saved as ${key}`, async () => {
      await section(where);
      await page.click(label, { role: "switch" });
      await until(() => settings()[key] === false, { what: `${key} off` });
      await page.click(label, { role: "switch" });
      await until(() => settings()[key] === true, { what: `${key} on` });
    });
  }

  test("Window: Hide the window while playing is saved, off by default", async () => {
    await section("Window");
    await page.click("Hide the window while playing", { role: "switch" });
    await until(() => settings().hide_while_playing === true, { what: "on" });
    await page.click("Hide the window while playing", { role: "switch" });
    await until(() => settings().hide_while_playing === false, { what: "off" });
  });

  test("Appearance: theme, spacing, mouse path and reduce motion are saved, and the page follows", async () => {
    await section("Appearance");
    const attr = (name) => page.run((name) => document.documentElement.getAttribute(name), name);
    const choose = async (option, key, value) => {
      await page.click(option, { role: "radio", within: `[aria-labelledby="s-${key}"]` });
      await until(() => settings()[key] === value, { what: `${key}: ${option}` });
    };
    await choose("Dark", "theme", "dark");
    assert.equal(await attr("data-theme"), "dark");
    await choose("Light", "theme", "light");
    assert.equal(await attr("data-theme"), "light");
    await choose("Like Windows", "theme", "system");
    await choose("Large", "spacing", "large");
    assert.equal(await attr("data-spacing"), "large");
    await choose("Comfortable", "spacing", "comfortable");
    assert.equal(await attr("data-spacing"), null);
    await choose("Played part", "path_mode", "trail");
    await choose("Whole path", "path_mode", "full");
    await choose("Always", "reduce_motion", "always");
    assert.equal(await attr("data-motion"), "calm");
    await choose("Like Windows", "reduce_motion", "system");
  });

  test("Appearance: a bigger text size zooms the window and the player (the player up to 150%)", async () => {
    const zoom = (p) => p.run(() => window.devicePixelRatio);
    const textSize = (text_size) => page.run((text_size) => window.__relay.updateSettings({ text_size }), text_size);
    try {
      await section("Appearance");
      const window1 = await zoom(page);
      await page.click("150%", { role: "radio", within: '[aria-labelledby="s-text_size"]' });
      await until(() => settings().text_size === 150, { what: "saved" });
      await until(async () => Math.abs((await zoom(page)) / window1 - 1.5) < 0.05, { what: "the window zoomed" });
      await textSize(200);
      await until(async () => Math.abs((await zoom(page)) / window1 - 2) < 0.05, { what: "the window at 200%" });
      // The player, once it shows.
      await page.invoke("set_player_mode", { on: true });
      await until(async () => Math.abs((await zoom(app.player)) - 1.5) < 0.05, { what: "the player at 150%" });
    } finally {
      await textSize(100);
      await page.invoke("set_player_mode", { on: false });
    }
  });

  test("Appearance: the player's size", async () => {
    await page.invoke("set_player_mode", { on: true });
    await until(() => visible("player"), { what: "the player" });
    const scale = await page.invoke("plugin:window|scale_factor", { label: "player" });
    const compact = await size("player");
    assert.deepEqual([compact.width, compact.height], [Math.round(600 * scale), Math.round(64 * scale)]);
    await page.run(() => window.__relay.updateSettings({ player_size: "large" }));
    await until(async () => (await size("player")).height === Math.round(88 * scale), { what: "the large player" });
    assert.equal((await size("player")).width, Math.round(760 * scale));
    await until(() => app.player.run(() => document.querySelector(".player").classList.contains("big")), { what: "drawn large" });
    await page.run(() => window.__relay.updateSettings({ player_size: "compact" }));
    await until(async () => (await size("player")).height === Math.round(64 * scale), { what: "compact again" });
  });

  test("Keep the player on top: Always, Never, and only while working", async () => {
    await section("Window");
    assert.equal(await playerOnTop(), true, "always, by default");
    await page.click("Never", { role: "radio" });
    await until(async () => settings().keep_on_top === "never" && (await playerOnTop()) === false, { what: "never" });
    await page.click("While working", { role: "radio" });
    await until(async () => settings().keep_on_top === "sessions", { what: "while working" });
    assert.equal(await playerOnTop(), false, "not while idle");

    // A recording's countdown is a session: on top for it, then back.
    await page.run(() => window.__relay.updateSettings({ countdown: true }));
    await page.invoke("toggle_record");
    await page.waitMode("countdown");
    await until(playerOnTop, { what: "on top during the countdown" });
    await page.invoke("toggle_record");
    await page.waitMode("idle");
    await until(async () => (await playerOnTop()) === false, { what: "off again" });

    // Playing, and paused, too.
    await page.open(waits);
    await page.click("Play");
    await page.waitMode("playing");
    await until(playerOnTop, { what: "on top while playing" });
    await page.click("Pause");
    await page.waitMode("paused");
    assert.equal(await playerOnTop(), true, "still on top while paused");
    await page.click("Stop");
    await page.waitMode("idle");
    await until(async () => (await playerOnTop()) === false, { what: "off after playing" });

    await section("Window");
    await page.click("Always", { role: "radio" });
    await until(async () => settings().keep_on_top === "always" && (await playerOnTop()) === true, { what: "always" });
    // The window itself is never on top.
    assert.equal(await page.invoke("plugin:window|is_always_on_top", { label: "main" }), false);
  });

  describe("Relay's keys", () => {
    /** Presses keys in the page, as a key field hears them (not the OS). */
    const press = (key, init = {}) =>
      page.run(
        (key, init) => {
          window.dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true, cancelable: true, ...init }));
          return true;
        },
        key,
        init,
      );
    const row = (name) => `.setting:has(#key-${name})`;
    const said = (name) => page.text(`${row(name)} [role="status"]`);

    test("Change, then a combination: saved to settings.json, and said", async () => {
      await section("Keys");
      await page.click("Change", { within: row("record") });
      await until(async () => /Press keys/.test((await page.text(row("record"))) ?? ""), { what: "listening" });
      await press("F8", { code: "F8" });
      await until(() => settings().keys?.record === "F8", { what: "saved" });
      await until(async () => (await said("record")) === "Set to F8.", { what: "said" });
    });

    test("a key apps use is set with a warning; one Relay has is refused", async () => {
      await section("Keys");
      await page.click("Change", { within: row("record") });
      await press("F9", { code: "F9" });
      await until(() => settings().keys?.record === "F9", { what: "saved" });
      await until(async () => /^F9 recalculates in Excel/.test((await said("record")) ?? ""), { what: "the warning" });
      await page.click("Change", { within: row("play") });
      await press("F9", { code: "F9" });
      await until(async () => /already Relay's “Record” key/.test((await said("play")) ?? ""), { what: "refused" });
      assert.equal(settings().keys.play, KEYS.play);
    });

    test("Backspace turns a key off; not Stop everything", async () => {
      await section("Keys");
      await page.click("Change", { within: row("player") });
      await press("Backspace", { code: "Backspace" });
      await until(() => settings().keys?.player === null, { what: "off" });
      await until(async () => (await said("player")) === "Set to None: use the buttons instead.", { what: "said" });
      await page.click("Change", { within: row("kill") });
      await press("Backspace", { code: "Backspace" });
      await until(async () => /can't be turned off/.test((await said("kill")) ?? ""), { what: "refused" });
      assert.equal(settings().keys.kill, KEYS.kill);
    });

    test("a macro's hotkey can't be one of Relay's keys as set now", async () => {
      const id = await page.store("library.0.id");
      const { triggers } = await page.invoke("get_triggers", { id });
      await assert.rejects(
        page.invoke("set_triggers", { id, triggers: { ...triggers, hotkey: { enabled: true, combo: "F9" } } }),
        /F9 is Relay's “Record” key/,
      );
      // Back to a new install's keys.
      await page.run((keys) => window.__relay.updateSettings({ keys }), KEYS);
      await until(() => JSON.stringify(settings().keys) === JSON.stringify(KEYS), { what: "the keys back" });
    });
  });

  test("settings survive a restart", async () => {
    await page.run(() => window.__relay.updateSettings({ show_click_labels: false, sounds: true }));
    await until(() => settings().show_click_labels === false && settings().sounds === true);
    page = await app.restart();
    assert.equal(await page.store("settings.show_click_labels"), false);
    assert.equal(await page.store("settings.sounds"), true);
    await page.run(() => window.__relay.updateSettings({ show_click_labels: true, sounds: false }));
  });

  test("a resize that isn't the user's (no drag of an edge) isn't saved as the window's size", async () => {
    await page.invoke("plugin:window|set_size", { label: "main", value: { Logical: { width: 900, height: 600 } } });
    await sleep(1200); // longer than the save after a resize
    assert.equal(app.json("window.json").size ?? null, null);
    await page.invoke("plugin:window|set_size", { label: "main", value: { Logical: { width: 944, height: 612 } } });
  });

  test("the window can be resized, the player can't", async () => {
    assert.equal(await page.invoke("plugin:window|is_resizable", { label: "main" }), true);
    assert.equal(await page.invoke("plugin:window|is_resizable", { label: "player" }), false);
  });

  test("the inspector's divider is saved to window.json and survives a restart; double-click resets it", async () => {
    await page.open(waits);
    const width = () => page.run(() => Math.round(document.querySelector(".inspector").getBoundingClientRect().width));
    const panes = () => app.json("window.json").panes;
    const start = await width();
    await page.run(() => {
      const el = document.querySelector('[role="separator"][aria-label="Resize the inspector"]');
      const r = el.getBoundingClientRect();
      const at = { clientX: r.left + 1, clientY: r.top + 10 };
      const send = (type, dx) =>
        el.dispatchEvent(new PointerEvent(type, { bubbles: true, button: 0, pointerId: 7, ...at, clientX: at.clientX + dx }));
      send("pointerdown", 0);
      send("pointermove", -60);
      send("pointerup", -60);
      return true;
    });
    await until(() => panes()?.inspector_w === start + 60, { what: "the width saved" });
    assert.equal(await width(), start + 60, "dragged left: a wider inspector");
    page = await app.restart();
    await page.open(waits);
    await until(async () => (await width()) === start + 60, { what: "as it was left" });
    await page.run(() => {
      document.querySelector('[role="separator"][aria-label="Resize the inspector"]').dispatchEvent(new MouseEvent("dblclick", { bubbles: true }));
      return true;
    });
    await until(() => panes()?.inspector_w === null, { what: "reset" });
    await until(async () => (await width()) === start, { what: "the default width" });
  });

  test("player mode is remembered: Relay starts as the player", async () => {
    await page.invoke("set_player_mode", { on: true });
    await until(() => app.json("window.json").player_mode === true, { what: "saved" });
    page = await app.restart();
    await until(async () => (await visible("player")) && !(await visible("main")), { what: "the player, not the window" });
    assert.equal(await page.store("playerMode"), true);
    await app.player.click("Open the window");
    await until(async () => (await visible("main")) && !(await visible("player")), { what: "the window" });
  });

  test("× with Close to the tray hides Relay, which keeps running; the player only shows for a session", async () => {
    assert.equal(await page.store("settings.close_to_tray"), true);
    await page.click("Hide to the tray");
    await until(async () => !(await visible("main")) && !(await visible("player")), { what: "hidden" });
    assert.equal(app.proc.exitCode, null, "still running");
    // A run (from the tray's Record, a hotkey or a trigger) shows the player, and only it.
    await page.run(() => window.__relay.updateSettings({ countdown: true }));
    await page.invoke("toggle_record");
    await page.waitMode("countdown");
    await until(async () => (await visible("player")) && !(await visible("main")), { what: "the player for the session" });
    await page.invoke("toggle_record");
    await page.waitMode("idle");
    await until(async () => !(await visible("player")), { what: "the player gone again" });
    await page.invoke("plugin:window|show", { label: "main" });
    await until(() => visible("main"), { what: "shown again" });
  });

  test("× without Close to the tray quits", async () => {
    await section("Window");
    await page.click("Close to the tray", { role: "switch" });
    await until(() => settings().close_to_tray === false);
    await page.click("Quit");
    const code = await Promise.race([app.exited, sleep(8000).then(() => "still running")]);
    assert.equal(code, 0, "quit cleanly");
    app.proc = null;
    app.page = null;
    app.player = null;
  });
});

describe("the first run", () => {
  test("a new install: an empty Library, a new install's keys, and the three screens; Later ends them for good", async () => {
    const app = new App({ samples: false, settings: null });
    try {
      const page = await app.start();
      assert.equal(await page.store("library.length"), 0);
      assert.equal(app.macroFiles().length, 0, "no samples");
      assert.deepEqual(await page.store("settings.keys"), KEYS);
      const place = (p) => p.run(() => document.querySelector(".app-window").dataset.place);
      await until(async () => (await place(page)) === "first-run", { what: "the first run" });
      assert.match(await page.text("h1"), /Do it once/);
      await page.click("Next");
      assert.match(await page.text("h1"), /Your keys/);
      assert.equal(
        await page.run(() => document.querySelector('[role="radio"][aria-checked="true"] b').textContent),
        "Ctrl + Shift + F9 and Ctrl + Shift + F10",
      );
      await page.click("Next");
      assert.match(await page.text("h1"), /Your first macro/);
      await page.click("Later");
      await until(() => app.exists("settings.json") && app.json("settings.json").intro === "done", { what: "the first run done" });
      assert.equal(await place(page), "library");
      const again = await app.restart();
      assert.equal(await place(again), "library");
    } finally {
      await app.dispose();
    }
  });

  test("an upgrade (settings without keys) keeps F9, F10 and Ctrl + Alt + End, and shows the keys screen once", async () => {
    const app = new App({ settings: { countdown: false } });
    try {
      const page = await app.start();
      assert.deepEqual(await page.store("settings.keys"), {
        record: "F9",
        play: "F10",
        player: "Ctrl + Shift + M",
        kill: "Ctrl + Alt + End",
      });
      assert.equal(await page.store("settings.countdown"), false, "the rest kept");
      await until(async () => /Your keys/.test((await page.text("h1")) ?? ""), { what: "the keys screen" });
      assert.equal(await page.run(() => document.querySelector('[role="radio"][aria-checked="true"] b').textContent), "F9 and F10");
      // Ctrl + Alt + End doesn't reach Relay over Remote Desktop: one click changes it.
      await page.click("Use Ctrl + Alt + Backspace");
      await until(() => app.json("settings.json").keys?.kill === "Ctrl + Alt + Backspace", { what: "the kill switch" });
      await page.click("Done");
      await until(() => app.json("settings.json").intro === "done", { what: "done" });
      assert.equal(app.json("settings.json").keys.record, "F9", "kept");
    } finally {
      await app.dispose();
    }
  });
});
