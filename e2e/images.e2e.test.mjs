// Finding images against the real app and screen. The image is shown by a
// small PowerShell window (image-window.ps1), since Relay leaves its own
// window out of the search. A Find image step that finds its image clicks,
// so the only step played here is one whose image never shows.
import { after, afterEach, before, describe, test } from "node:test";
import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { App, sleep, until, waitingMacro, writeRly } from "./harness.mjs";

const SCRIPT = join(fileURLToPath(new URL(".", import.meta.url)), "image-window.ps1");
/** Where the window shows the image (physical pixels), clear of Relay's widget. */
const AT = { x: 120, y: 120 };

describe("images", () => {
  const app = new App();
  let page;
  let target; // a macro that only waits
  let image; // the pattern, as Relay keeps it
  let shown = null; // the window showing it

  const powershell = (args) =>
    spawn("powershell", ["-NoProfile", "-ExecutionPolicy", "Bypass", "-File", SCRIPT, ...args], { stdio: "ignore", windowsHide: false });
  const find = (threshold = 85) => page.invoke("test_find_image", { image, threshold, area: null });
  /** Shows the pattern at AT, and waits until Relay can see it. */
  const show = async () => {
    shown = powershell(["-Png", app.path("shown.png"), "-X", String(AT.x), "-Y", String(AT.y)]);
    await until(async () => (await find())?.score >= 85, { timeout: 20_000, every: 250, what: "the pattern on screen" });
  };
  const hide = async () => {
    shown?.kill();
    shown = null;
    await until(async () => !((await find())?.score >= 85), { every: 250, what: "the pattern gone" });
  };

  before(async () => {
    page = await app.start();
    const png = app.path("pattern.png");
    await new Promise((resolve, reject) => powershell(["-Png", png, "-SaveOnly"]).on("exit", (c) => (c === 0 ? resolve() : reject(new Error(`exit ${c}`)))));
    image = await page.invoke("load_image", { path: png });
    const [path] = writeRly(app.path("import"), [waitingMacro({ name: "Triggered", waits: [1500] })]);
    [target] = (await page.invoke("import_macros", { paths: [path] })).imported;
    await page.run(() => window.__relay.refreshLibrary());
  });
  after(() => {
    shown?.kill();
    return app.dispose();
  });
  afterEach(() => app.page?.reset());

  const runs = (id = target) => app.json("library.json").entries[id]?.runs ?? 0;

  test("an image file is read as a PNG, and a picture with no detail is refused", async () => {
    assert.match(image, /^iVBORw0KGgo/);
    const plain = app.path("plain.png");
    // A flat 20×20 gray PNG, written by PowerShell's drawing too.
    const script = `Add-Type -AssemblyName System.Drawing; $b = New-Object System.Drawing.Bitmap 20, 20; $g = [System.Drawing.Graphics]::FromImage($b); $g.Clear([System.Drawing.Color]::Gray); $b.Save('${plain}')`;
    await new Promise((r) => spawn("powershell", ["-NoProfile", "-Command", script], { stdio: "ignore" }).on("exit", r));
    await assert.rejects(page.invoke("load_image", { path: plain }), (e) => e.code === "image" && /too plain/.test(e.message));
    await assert.rejects(page.invoke("load_image", { path: app.path("missing.png") }), (e) => e.code === "io");
  });

  test("Test finds the image where another window shows it, and not in Relay's own window", async () => {
    await show();
    const m = await find();
    assert.ok(Math.abs(m.x - AT.x) <= 1 && Math.abs(m.y - AT.y) <= 1, `found at ${m.x}, ${m.y}`);
    assert.deepEqual([m.w, m.h], [120, 60]);
    const color = await page.invoke("sample_pixel", { x: AT.x + 7, y: AT.y + 10 });
    await hide();

    // The same picture in Relay's page, 1:1: on screen, but not found.
    const pos = await page.invoke("plugin:window|inner_position", { label: "main" });
    const at = await page.run((src) => {
      const img = document.createElement("img");
      img.id = "e2e-image";
      img.src = `data:image/png;base64,${src}`;
      const k = window.devicePixelRatio;
      Object.assign(img.style, { position: "fixed", left: "40px", top: "260px", width: `${120 / k}px`, height: `${60 / k}px`, zIndex: 99999 });
      document.body.append(img);
      return { x: Math.round((40 + 7 / k) * k), y: Math.round((260 + 10 / k) * k) };
    }, image);
    await until(async () => (await page.invoke("sample_pixel", { x: pos.x + at.x, y: pos.y + at.y })) === color, {
      what: "the picture drawn in Relay's window",
    });
    const inRelay = await find();
    assert.ok(!(inRelay?.score >= 85), `found in Relay's window: ${JSON.stringify(inRelay)}`);
    await page.run(() => document.getElementById("e2e-image")?.remove());
  });

  test("the trigger's own thumbnail is never found, on any monitor", async () => {
    const { triggers } = await page.invoke("get_triggers", { id: target });
    await page.run((id) => window.__relay.loadMacro(id), target);
    await page.invoke("set_triggers", { id: target, triggers: { ...triggers, image: { enabled: false, image, threshold: 85, area: null } } });
    await page.run(() => window.__relay.loadTriggers());
    await page.tab("Triggers");
    const home = await page.invoke("plugin:window|outer_position", { label: "main" });
    try {
      for (const m of await page.invoke("plugin:window|available_monitors", {})) {
        await page.invoke("plugin:window|set_position", { label: "main", value: { Physical: { x: m.position.x + 40, y: m.position.y + 40 } } });
        // On screen, at a size it would be found at (half its own, or more).
        await until(
          () =>
            page.run(() => {
              const el = document.querySelector(".thumb");
              el?.scrollIntoView({ block: "center" });
              return el && el.complete && el.getBoundingClientRect().width * devicePixelRatio >= 60 ? true : null;
            }),
          { what: "the thumbnail shown" },
        );
        await sleep(300);
        const m2 = await find();
        assert.ok(!(m2?.score >= 85), `the thumbnail was found on the monitor at ${m.position.x}, ${m.position.y}: ${JSON.stringify(m2)}`);
      }
    } finally {
      await page.invoke("plugin:window|set_position", { label: "main", value: { Physical: home } });
    }
  });

  test("the image trigger runs the macro when the image appears, once", async () => {
    const { triggers } = await page.invoke("get_triggers", { id: target });
    const set = (on) => page.invoke("set_triggers", { id: target, triggers: { ...triggers, image: { enabled: on, image, threshold: 85, area: null } } });
    await set(true);
    assert.deepEqual(app.json("library.json").entries[target].triggers.image, { enabled: true, image, threshold: 85, area: null });
    await sleep(2000); // looked at least twice without it: armed
    const before = runs();
    shown = powershell(["-Png", app.path("shown.png"), "-X", String(AT.x), "-Y", String(AT.y)]);
    await until(() => page.run((id) => window.__relay.mode === "playing" && window.__relay.view?.id === id, target), {
      timeout: 20_000,
      every: 50,
      what: "the trigger to run the macro",
    });
    await page.waitMode("idle");
    await until(() => runs() === before + 1, { what: "the run to be counted" });
    // Still on screen: not again.
    const end = Date.now() + 2000;
    while (Date.now() < end) {
      assert.equal(await page.store("mode"), "idle", "ran again");
      await sleep(100);
    }
    assert.equal(runs(), before + 1);
    await set(false);
    await hide();
  });

  test("a Find image step whose image never shows stops playback and stays on its step", async () => {
    assert.ok(!((await find())?.score >= 85), "the image isn't on screen: the step would click it");
    const macro = waitingMacro({ name: "Finds", waits: [200] });
    macro.version = 2;
    macro.events.push({ type: "find_image", t: 200, dur: 500, image, click_x: 60, click_y: 30, btn: "Left", threshold: 85, timeout_ms: 1000, label: "Never shows" });
    const [path] = writeRly(app.path("import"), [macro]);
    const [id] = (await page.invoke("import_macros", { paths: [path] })).imported;
    await page.run(async (id) => {
      await window.__relay.refreshLibrary();
      await window.__relay.loadMacro(id);
    }, id);
    await until(async () => (await page.store("view.id")) === id, { what: "the macro to open" });
    const steps = await page.store("view.steps");
    assert.deepEqual(steps.map((s) => s.kind), ["wait", "find_image"]);
    await page.click("Play");
    await page.waitMode("playing");
    await page.waitMode("idle", 6000);
    assert.equal(await page.store("lastFinish"), "pixel_timeout");
    assert.equal(await page.store("toast.message"), "Image not found at step 2; playback stopped.");
    assert.equal(await page.store("curStepIdx"), 1);
  });
});
