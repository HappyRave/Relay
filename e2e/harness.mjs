// End-to-end harness: starts the real Relay (a built relay.exe) on a
// throwaway data folder with WebView2 remote debugging (RELAY_DEVTOOLS_PORT),
// and drives its UI
// over the Chrome DevTools Protocol: clicking the page's own buttons,
// reading `window.__relay` (the UI store) and calling commands directly
// where the UI would open a native dialog.
//
// It never synthesizes OS input: the tests only use the page's DOM and
// Relay's commands. Macros that are played contain only waits and pixel
// checks, so playback sends nothing to the desktop either.
import { spawn, execFileSync } from "node:child_process";
import { mkdtempSync, rmSync, existsSync, readFileSync, readdirSync, writeFileSync, mkdirSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { randomUUID } from "node:crypto";

const ROOT = resolve(import.meta.dirname, "..");
export const EXE = resolve(process.env.RELAY_EXE ?? join(ROOT, "target", "debug", "relay.exe"));

export const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

/** Polls `check` until it returns something truthy, or fails after `timeout` ms. */
export async function until(check, { timeout = 10_000, every = 100, what = "condition" } = {}) {
  const end = Date.now() + timeout;
  let last;
  for (;;) {
    try {
      last = await check();
      if (last) return last;
    } catch (e) {
      if (e?.fatal) throw e;
      last = e;
    }
    if (Date.now() > end) throw new Error(`timed out waiting for ${what} (last: ${last instanceof Error ? last.message : JSON.stringify(last)})`);
    await sleep(every);
  }
}

function relayRunning() {
  try {
    const out = execFileSync("tasklist", ["/FI", "IMAGENAME eq relay.exe", "/FO", "CSV", "/NH"], { encoding: "utf8" });
    return out.toLowerCase().includes("relay.exe");
  } catch {
    return false;
  }
}

/** A Chrome DevTools Protocol session with the page. */
class Page {
  constructor(ws) {
    this.ws = ws;
    this.id = 0;
    this.pending = new Map();
    ws.addEventListener("message", (ev) => {
      const msg = JSON.parse(ev.data);
      const p = this.pending.get(msg.id);
      if (p) {
        this.pending.delete(msg.id);
        p(msg);
      }
    });
  }

  send(method, params = {}) {
    const id = ++this.id;
    return new Promise((resolve) => {
      this.pending.set(id, resolve);
      this.ws.send(JSON.stringify({ id, method, params }));
    });
  }

  /** Runs `fn(...args)` in the page (it may be async) and returns its JSON result. */
  async run(fn, ...args) {
    const expression = `(${fn})(...${JSON.stringify(args)})`;
    const res = await this.send("Runtime.evaluate", { expression, awaitPromise: true, returnByValue: true });
    const r = res.result;
    if (r?.exceptionDetails) {
      throw new Error("in page: " + (r.exceptionDetails.exception?.description ?? r.exceptionDetails.text));
    }
    return r?.result?.value;
  }

  /** A command, as the UI would send it. Rejections come back as `{ code, message }`. */
  invoke(cmd, args = {}) {
    return this.run(
      async (cmd, args) => {
        try {
          return { ok: await window.__TAURI_INTERNALS__.invoke(cmd, args) };
        } catch (e) {
          return { err: e };
        }
      },
      cmd,
      args,
    ).then((r) => {
      if (r && "err" in r) throw Object.assign(new Error(r.err?.message ?? String(r.err)), { code: r.err?.code });
      return r?.ok;
    });
  }

  /** Reads the store: `page.store("mode")`, `page.store("view.name")`. */
  store(path) {
    return this.run((path) => path.split(".").reduce((o, k) => o?.[k], window.__relay), path);
  }

  /** Clicks the button (or tab, switch, radio…) with this accessible name, like `getByRole`. */
  click(name, { role, within } = {}) {
    return this.run(
      (name, role, within) => {
        const root = within ? document.querySelector(within) : document;
        const label = (el) =>
          (el.getAttribute("aria-label") ?? el.textContent ?? "").replace(/\s+/g, " ").trim();
        const candidates = [...root.querySelectorAll(role ? `[role="${role}"]` : 'button, [role="button"], [role="tab"], [role="switch"], [role="radio"]')];
        const el = candidates.find((c) => label(c) === name) ?? candidates.find((c) => label(c).startsWith(name));
        if (!el) throw new Error(`no control named “${name}”`);
        if (el.disabled) throw new Error(`“${name}” is disabled`);
        el.click();
        return true;
      },
      name,
      role ?? null,
      within ?? null,
    );
  }

  /** Sets an input's value and fires the events Svelte listens to. */
  fill(selector, value) {
    return this.run(
      (selector, value) => {
        const el = document.querySelector(selector);
        if (!el) throw new Error(`no ${selector}`);
        el.value = value;
        el.dispatchEvent(new Event("input", { bubbles: true }));
        el.dispatchEvent(new Event("change", { bubbles: true }));
        return true;
      },
      selector,
      value,
    );
  }

  text(selector = "body") {
    return this.run((s) => document.querySelector(s)?.innerText ?? null, selector);
  }

  /** Waits until the store's `mode` is `mode`. */
  waitMode(mode, timeout = 10_000) {
    return until(async () => (await this.store("mode")) === mode, { timeout, what: `mode ${mode}` });
  }

  /** Waits for the UI to settle (pending commands answered, a frame drawn). */
  idle(ms = 150) {
    return this.run((ms) => new Promise((r) => setTimeout(() => requestAnimationFrame(() => r(true)), ms)), ms);
  }

  close() {
    this.ws.close();
  }
}

/**
 * Relay on a scratch data folder. `start()` launches it and connects;
 * `quit()` asks it to exit (as the tray's Quit does); `restart()` does both,
 * keeping the data, to check what survives.
 */
export class App {
  constructor({ port = 9300 + Math.floor(Math.random() * 500), dir } = {}) {
    this.port = port;
    this.dir = dir ?? mkdtempSync(join(tmpdir(), "relay-e2e-"));
    this.proc = null;
    this.page = null;
    this.log = "";
  }

  path(...parts) {
    return join(this.dir, ...parts);
  }

  json(...parts) {
    return JSON.parse(readFileSync(this.path(...parts), "utf8"));
  }

  exists(...parts) {
    return existsSync(this.path(...parts));
  }

  macroFiles() {
    return existsSync(this.path("macros")) ? readdirSync(this.path("macros")).filter((f) => f.endsWith(".rly")) : [];
  }

  /** The macro as saved on disk. */
  macro(id) {
    return this.json("macros", `${id}.rly`);
  }

  writeSettings(settings) {
    mkdirSync(this.dir, { recursive: true });
    writeFileSync(this.path("settings.json"), JSON.stringify(settings));
  }

  async start() {
    if (!existsSync(EXE)) throw new Error(`${EXE} doesn't exist: build it with \`npx tauri build --debug --no-bundle\``);
    if (relayRunning()) throw new Error("Relay is already running. Quit it first: Relay only runs once, so the test would reach it instead.");
    const env = {
      ...process.env,
      RELAY_DATA_DIR: this.dir,
      RELAY_DEVTOOLS_PORT: String(this.port),
    };
    this.proc = spawn(EXE, [], { env, stdio: ["ignore", "pipe", "pipe"], windowsHide: true });
    this.proc.stdout.on("data", (d) => (this.log += d));
    this.proc.stderr.on("data", (d) => (this.log += d));
    this.exitCode = null;
    this.exited = new Promise((r) => this.proc.once("exit", (code) => r((this.exitCode = code ?? -1))));

    let target;
    try {
      target = await until(
        async () => {
          if (this.exitCode != null) throw Object.assign(new Error(`Relay exited with code ${this.exitCode}`), { fatal: true });
          const targets = await (await fetch(`http://127.0.0.1:${this.port}/json`)).json();
          return targets.find((t) => t.type === "page" && !t.url.startsWith("devtools"));
        },
        { timeout: 45_000, every: 250, what: "the webview's debugging port" },
      );
    } catch (e) {
      throw new Error(`${e.message}\n${this.diagnostics()}`);
    }
    const ws = new WebSocket(target.webSocketDebuggerUrl);
    await new Promise((r, j) => {
      ws.addEventListener("open", r, { once: true });
      ws.addEventListener("error", j, { once: true });
    });
    this.page = new Page(ws);
    await until(() => this.page.run(() => !!window.__relay?.ready && window.__relay.library.length > 0 && !!window.__relay.view), {
      timeout: 20_000,
      what: "the UI to load",
    });
    return this.page;
  }

  /** What Relay printed and logged, for a failure message. */
  diagnostics() {
    const tail = (text, n = 30) => text.trim().split(/\r?\n/).slice(-n).join("\n");
    let logs = "";
    try {
      for (const f of readdirSync(this.path("logs"))) logs += readFileSync(this.path("logs", f), "utf8");
    } catch {
      logs = "(no log folder)";
    }
    const state = this.exitCode == null ? "still running" : `exited with code ${this.exitCode}`;
    return `Relay ${state}.\n--- output ---\n${tail(this.log) || "(none)"}\n--- log ---\n${tail(logs) || "(empty)"}`;
  }

  async quit() {
    if (!this.proc) return;
    try {
      await Promise.race([this.page?.invoke("quit"), sleep(2000)]);
    } catch {
      // The page goes away as Relay exits.
    }
    this.page?.close();
    const exited = await Promise.race([this.exited.then(() => true), sleep(8000).then(() => false)]);
    if (!exited) this.proc.kill();
    await this.exited;
    this.proc = null;
    this.page = null;
  }

  async restart() {
    await this.quit();
    return this.start();
  }

  /** Quits and deletes the scratch folder. */
  async dispose() {
    await this.quit();
    rmSync(this.dir, { recursive: true, force: true });
  }
}

const DESKTOP = { x: 0, y: 0, w: 1920, h: 1080 };

/**
 * A `.rly` document that only waits (and optionally checks a pixel), so
 * playing it sends no input to the desktop.
 */
export function waitingMacro({ name = "Just waits", waits = [1500], pixel = null, speed = 1, repeat = { count: 1 } } = {}) {
  const events = [];
  let t = 0;
  for (const dur of waits) {
    events.push({ type: "wait", t, dur, label: `Wait ${dur}` });
    t += dur;
  }
  if (pixel) events.push({ type: "pixel_wait", t, dur: 500, tolerance: 0, timeout_ms: 800, label: "Never matches", ...pixel });
  return {
    format: "relay-macro",
    version: 1,
    id: randomUUID(),
    name,
    created_at: "2026-09-26T10:00:00Z",
    modified_at: "2026-09-26T10:00:00Z",
    recording: {
      os: "windows",
      virtual_desktop: DESKTOP,
      monitors: [{ name: "\\\\.\\DISPLAY1", rect: DESKTOP, work: { ...DESKTOP, h: 1032 }, dpi: 96, primary: true }],
      double_click_ms: 500,
      double_click_px: 4,
    },
    // No humanize (exact timing) and no stop-on-key (a stray key press mustn't stop a test).
    playback: { speed, repeat, humanize: false, jitter_ms: 0, stop_on_key: false, coord_mode: "screen" },
    events,
  };
}

/** Writes `.rly` documents to a folder and returns their paths. */
export function writeRly(dir, docs) {
  mkdirSync(dir, { recursive: true });
  return docs.map((doc, i) => {
    const p = join(dir, `${i}-${doc.name ?? "file"}.rly`.replace(/[^\w.-]+/g, "_"));
    writeFileSync(p, typeof doc === "string" ? doc : JSON.stringify(doc));
    return p;
  });
}
