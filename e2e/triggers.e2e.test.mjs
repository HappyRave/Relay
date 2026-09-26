// Triggers against the real app: what's saved, the hotkey checks Rust
// makes, and the app-launch, pixel and schedule triggers actually firing.
// The macros they run only wait, so nothing is sent to the desktop.
import { after, before, describe, test } from "node:test";
import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { readdirSync, readFileSync } from "node:fs";
import { App, sleep, until, waitingMacro, writeRly } from "./harness.mjs";

describe("triggers", () => {
  const app = new App();
  let page;
  let target; // a macro that only waits
  let sample; // "Export invoice to PDF"

  before(async () => {
    page = await app.start();
    sample = await page.store("view.id");
    const [path] = writeRly(app.path("import"), [waitingMacro({ name: "Triggered", waits: [1500] })]);
    [target] = (await page.invoke("import_macros", { paths: [path] })).imported;
    await page.run(async (id) => {
      await window.__relay.refreshLibrary();
      await window.__relay.loadMacro(id);
    }, target);
    await page.click("Triggers", { role: "tab" });
    await page.idle();
  });
  after(() => app.dispose());

  const saved = (id = target) => app.json("library.json").entries[id]?.triggers;
  const status = (id = target) => page.invoke("get_triggers", { id });
  const set = async (patch, id = target) => {
    const { triggers } = await status(id);
    return page.invoke("set_triggers", { id, triggers: { ...triggers, ...patch } });
  };
  /** Waits for a triggered run of `id` to start, then for it to end. */
  const firesAndRuns = async (id, timeout) => {
    await until(async () => (await page.store("mode")) === "playing" && (await page.store("view.id")) === id, {
      timeout,
      every: 100,
      what: "the trigger to run the macro",
    });
    await page.waitMode("idle");
  };
  /**
   * Starts ping.exe for a few seconds. First waits out one poll of the app watcher (2 s),
   * so it has seen ping not running: a program already running when the watcher first
   * looks, or one restarted between two looks, isn't a launch.
   */
  const ping = async (seconds) => {
    await sleep(2500);
    return spawn("ping", ["-n", String(seconds), "127.0.0.1"], { stdio: "ignore", windowsHide: true });
  };

  describe("hotkey", () => {
    test("a free combo is registered and shown in the Library", async () => {
      const s = await set({ hotkey: { enabled: true, combo: "Ctrl + Alt + Shift + F11" } });
      assert.deepEqual(s.triggers.hotkey, { enabled: true, combo: "Ctrl + Alt + Shift + F11" });
      assert.equal(s.hotkey_error, null);
      assert.deepEqual(saved().hotkey, { enabled: true, combo: "Ctrl + Alt + Shift + F11" });
      const list = await page.invoke("list_macros");
      assert.equal(list.find((m) => m.id === target).hotkey, "Ctrl + Alt + Shift + F11");
    });

    test("Relay's own hotkeys are refused", async () => {
      for (const combo of ["F9", "F10", "Ctrl + Shift + M", "Ctrl + Alt + End"]) {
        await assert.rejects(set({ hotkey: { enabled: true, combo } }), /one of Relay's own hotkeys/, combo);
      }
      assert.equal(saved().hotkey.combo, "Ctrl + Alt + Shift + F11", "the old one is kept");
    });

    test("another macro's hotkey is refused, written either way", async () => {
      await set({ hotkey: { enabled: true, combo: "Ctrl + Alt + 1" } }, sample);
      await assert.rejects(set({ hotkey: { enabled: true, combo: "Alt + Ctrl + 1" } }), /already runs “Export invoice to PDF”/);
      // Once that one is off, the combo is free.
      await set({ hotkey: { enabled: false, combo: "Ctrl + Alt + 1" } }, sample);
      await set({ hotkey: { enabled: true, combo: "Ctrl + Alt + 1" } });
      await set({ hotkey: { enabled: true, combo: "Ctrl + Alt + Shift + F11" } });
    });

    test("combos that can't work are explained", async () => {
      await assert.rejects(set({ hotkey: { enabled: true, combo: "Q" } }), /Add Ctrl, Alt, Shift or Win/);
      await assert.rejects(set({ hotkey: { enabled: true, combo: "Hyper + Q" } }), /isn't a modifier/);
      await assert.rejects(set({ hotkey: { enabled: true, combo: "Ctrl + Banana" } }), /isn't a key Relay can use/);
    });

    test("the Triggers tab shows it", async () => {
      await page.run((id) => window.__relay.loadMacro(id), target);
      await page.click("Triggers", { role: "tab" });
      await page.idle();
      assert.match(await page.text(".list"), /Ctrl \+ Alt \+ Shift \+ F11/);
      assert.equal(await page.run(() => document.querySelector('[aria-label="Hotkey trigger"]').getAttribute("aria-checked")), "true");
    });
  });

  describe("schedule", () => {
    test("days and time from the tab are saved, and Rust says when it runs next", async () => {
      await page.click("Schedule trigger", { role: "switch" });
      await until(() => saved()?.schedule?.enabled === true, { what: "the schedule on" });
      await page.run(() => document.querySelectorAll('[aria-label="Days"] button')[6].click()); // Sunday
      await until(() => saved().schedule.schedule.days[6] === true, { what: "Sunday" });
      await page.fill('input[aria-label="Time"]', "23:59");
      await until(() => saved().schedule.schedule.time === "23:59", { what: "the time" });
      const s = await status();
      assert.ok(s.next_run, "a next run");
      const next = new Date(s.next_run);
      assert.equal(next.getHours(), 23);
      assert.equal(next.getMinutes(), 59);
      await until(async () => /^Next run: (Today|Tomorrow|Mon|Tue|Wed|Thu|Fri|Sat|Sun) 23:59$/.test((await page.text(".sub.accent")) ?? ""), {
        what: "the label",
      });
    });

    test("with no day picked there's no next run", async () => {
      const s = await set({ schedule: { enabled: true, schedule: { days: [false, false, false, false, false, false, false], time: "09:00" } } });
      assert.equal(s.next_run, null);
      await set({ schedule: { enabled: false, schedule: { days: [true, true, true, true, true, false, false], time: "09:00" } } });
      assert.equal((await status()).next_run, null, "off: no next run either");
    });

    test("it runs the macro at the scheduled minute", async () => {
      // The next whole minute, at least 10 s away.
      const at = new Date(Date.now() + 10_000);
      at.setSeconds(0, 0);
      at.setMinutes(at.getMinutes() + 1);
      const time = `${String(at.getHours()).padStart(2, "0")}:${String(at.getMinutes()).padStart(2, "0")}`;
      await set({ schedule: { enabled: true, schedule: { days: [true, true, true, true, true, true, true], time } } });
      await firesAndRuns(target, at.getTime() - Date.now() + 15_000);
      assert.ok(Date.now() >= at.getTime(), "not early");
      await until(() => app.json("library.json").entries[target].runs >= 1, { what: "the run counted" });
      await set({ schedule: { enabled: false, schedule: { days: [true, true, true, true, true, true, true], time } } });
    });
  });

  describe("app launch", () => {
    test("the program and delay from the tab are saved; the switch needs a program", async () => {
      await page.click("App launch trigger", { role: "switch" });
      await page.idle(300);
      assert.equal(saved().app_launch.enabled, false);
      await page.fill('input[aria-label="Program"]', "  PING.EXE  ");
      await until(() => saved().app_launch.exe === "PING.EXE", { what: "the program" });
      await page.fill('input[aria-label="Delay in seconds"]', "0.5");
      await until(() => saved().app_launch.delay_ms === 500, { what: "the delay" });
      await page.click("App launch trigger", { role: "switch" });
      await until(() => saved().app_launch.enabled === true, { what: "on" });
    });

    test("starting the program runs the macro", async () => {
      const p = await ping(6);
      try {
        await firesAndRuns(target, 8000);
      } finally {
        p.kill();
      }
    });

    test("while Relay is busy, the trigger is skipped with a notice", async () => {
      await page.run((id) => window.__relay.loadMacro(id), sample);
      await page.run(() => window.__relay.updateSettings({ countdown: false }));
      await page.click("Record");
      await page.waitMode("recording");
      const p = await ping(4);
      try {
        await until(async () => /^Skipped “Triggered”: Relay was busy$/.test((await page.store("toast.message")) ?? ""), {
          timeout: 8000,
          what: "the notice",
        });
      } finally {
        p.kill();
        await page.click("Stop recording");
        await page.waitMode("idle");
      }
    });

    test("paused triggers don't run; Resume turns them back on", async () => {
      await page.invoke("set_triggers_paused", { paused: true });
      await page.run((id) => window.__relay.loadMacro(id), target);
      await page.click("Triggers", { role: "tab" });
      await until(async () => /Triggers are paused/.test((await page.text(".paused")) ?? ""), { what: "the banner" });
      const p = await ping(4);
      await sleep(5000);
      p.kill();
      assert.equal(await page.store("mode"), "idle", "nothing ran");

      await page.click("Resume");
      await until(async () => (await page.text(".paused")) == null, { what: "the banner to go" });
      assert.equal((await status()).paused, false);
      const again = await ping(6);
      try {
        await firesAndRuns(target, 8000);
      } finally {
        again.kill();
      }
      await set({ app_launch: { enabled: false, exe: "PING.EXE", delay_ms: 500 } });
    });
  });

  describe("pixel", () => {
    /** A patch of Relay's own window, and the screen pixel at its center. */
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

    test("Relay reads screen pixels", async () => {
      const { x, y } = await patch("#FFFFFF");
      await until(async () => (await page.invoke("sample_pixel", { x, y })) === "#FFFFFF", { what: "white under the patch" });
      await patch("#0000FF");
      await until(async () => (await page.invoke("sample_pixel", { x, y })) === "#0000FF", { what: "blue under the patch" });
    });

    test("a pixel turning its color runs the macro, once per change", async () => {
      const { x, y } = await patch("#FFFFFF");
      await until(async () => (await page.invoke("sample_pixel", { x, y })) === "#FFFFFF");
      await set({ pixel: { enabled: true, x, y, color: "#EC3013", tolerance: 8 } });
      assert.deepEqual(saved().pixel, { enabled: true, x, y, color: "#EC3013", tolerance: 8 });
      await sleep(800); // a white baseline first
      await patch("#E83410"); // within the tolerance
      await firesAndRuns(target, 5000);
      // Still red: not again.
      await sleep(1500);
      assert.equal(await page.store("mode"), "idle");
      await patch("#FFFFFF");
      await sleep(800);
      await patch("#EC3013");
      await firesAndRuns(target, 5000);
      await set({ pixel: { enabled: false, x, y, color: "#EC3013", tolerance: 8 } });
      await page.run(() => document.getElementById("e2e-patch")?.remove());
    });
  });

  test("the log records the start and each triggered run", async () => {
    const log = () => {
      const files = readdirSync(app.path("logs")).filter((f) => /^relay\.\d{4}-\d{2}-\d{2}\.log$/.test(f));
      return files.map((f) => readFileSync(app.path("logs", f), "utf8")).join("");
    };
    await until(() => /Relay started/.test(log()) && /trigger due/.test(log()) && /running a triggered macro/.test(log()), {
      what: "the log lines",
    });
    for (const source of ["Schedule", "AppLaunch", "Pixel"]) assert.match(log(), new RegExp(`source=${source}`));
  });

  test("triggers survive a restart", async () => {
    const before = saved();
    page = await app.restart();
    const s = await status();
    assert.deepEqual(s.triggers, before);
    assert.equal(s.paused, false, "a restart starts with triggers running");
  });
});
