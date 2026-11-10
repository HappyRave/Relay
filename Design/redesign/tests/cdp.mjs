// A small Chrome DevTools Protocol driver for headless Edge: navigate, run JS,
// press keys, click, emulate media and vision, screenshot, read the AX tree.
import { spawn } from "node:child_process";
import fs from "node:fs";
import path from "node:path";

const EDGE = process.env.EDGE ?? "C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe";

export async function launch({ port = 9333, profile, width = 1366, height = 768 } = {}) {
  fs.rmSync(profile, { recursive: true, force: true });
  const proc = spawn(EDGE, ["--headless=new", `--remote-debugging-port=${port}`, `--user-data-dir=${profile}`, "--hide-scrollbars", "--no-first-run", "--disable-extensions", `--window-size=${width},${height}`, "about:blank"], { stdio: "ignore" });
  let targets;
  for (let i = 0; i < 50; i++) {
    try { targets = await (await fetch(`http://127.0.0.1:${port}/json`)).json(); if (targets.some((t) => t.type === "page")) break; } catch {}
    await new Promise((r) => setTimeout(r, 200));
  }
  const page = targets.find((t) => t.type === "page");
  const ws = new WebSocket(page.webSocketDebuggerUrl);
  await new Promise((r) => ws.addEventListener("open", r, { once: true }));
  let id = 0;
  const pending = new Map();
  const listeners = [];
  ws.addEventListener("message", (ev) => {
    const msg = JSON.parse(ev.data);
    if (msg.id && pending.has(msg.id)) { const { res, rej } = pending.get(msg.id); pending.delete(msg.id); msg.error ? rej(new Error(msg.error.message)) : res(msg.result); }
    else listeners.forEach((l) => l(msg));
  });
  const send = (method, params = {}) => new Promise((res, rej) => { const i = ++id; pending.set(i, { res, rej }); ws.send(JSON.stringify({ id: i, method, params })); });
  const errors = [];
  listeners.push((m) => { if (m.method === "Runtime.exceptionThrown") errors.push(m.params.exceptionDetails?.exception?.description ?? m.params.exceptionDetails?.text); if (m.method === "Runtime.consoleAPICalled" && m.params.type === "error") errors.push(m.params.args.map((a) => a.value ?? a.description).join(" ")); });
  await send("Page.enable"); await send("Runtime.enable"); await send("Accessibility.enable");
  await send("Emulation.setDeviceMetricsOverride", { width, height, deviceScaleFactor: 1, mobile: false });

  const api = {
    send, errors,
    async goto(url) {
      await send("Page.navigate", { url });
      await new Promise((r) => { const l = (m) => { if (m.method === "Page.loadEventFired") { listeners.splice(listeners.indexOf(l), 1); r(); } }; listeners.push(l); });
      await api.eval("document.fonts.ready.then(() => true)");
      await api.wait(250);
    },
    async eval(expr) {
      const r = await send("Runtime.evaluate", { expression: expr, awaitPromise: true, returnByValue: true });
      if (r.exceptionDetails) throw new Error(r.exceptionDetails.exception?.description ?? r.exceptionDetails.text);
      return r.result.value;
    },
    wait: (ms) => new Promise((r) => setTimeout(r, ms)),
    async shot(file, clip) {
      const r = await send("Page.captureScreenshot", { format: "png", ...(clip ? { clip: { ...clip, scale: 1 } } : {}) });
      fs.mkdirSync(path.dirname(file), { recursive: true });
      fs.writeFileSync(file, Buffer.from(r.data, "base64"));
    },
    async media(features = [], type = "") { await send("Emulation.setEmulatedMedia", { media: type, features }); },
    async vision(type = "none") { await send("Emulation.setEmulatedVisionDeficiency", { type }); },
    async key(key, { code, mods = 0, text } = {}) {
      const vk = { Tab: 9, Enter: 13, Escape: 27, Backspace: 8, Delete: 46, Insert: 45, ArrowUp: 38, ArrowDown: 40, ArrowLeft: 37, ArrowRight: 39, Home: 36, End: 35, " ": 32, F9: 120, F10: 121 }[key] ?? key.toUpperCase().charCodeAt(0);
      const base = { modifiers: mods, key, code: code ?? (key.length === 1 ? `Key${key.toUpperCase()}` : key), windowsVirtualKeyCode: vk };
      const t = text ?? (key === "Enter" ? String.fromCharCode(13) : key === " " ? " " : undefined);
      await send("Input.dispatchKeyEvent", { type: t ? "keyDown" : "rawKeyDown", ...base, text: t });
      await send("Input.dispatchKeyEvent", { type: "keyUp", ...base });
      await api.wait(40);
    },
    async click(x, y) {
      for (const type of ["mousePressed", "mouseReleased"]) await send("Input.dispatchMouseEvent", { type, x, y, button: "left", clickCount: 1 });
      await api.wait(80);
    },
    async clickSel(sel) {
      const r = await api.eval(`(() => { const e = document.querySelector(${JSON.stringify(sel)}); if (!e) return null; const b = e.getBoundingClientRect(); return [b.left + b.width / 2, b.top + b.height / 2]; })()`);
      if (!r) throw new Error(`no element ${sel}`);
      await api.eval(`document.querySelector(${JSON.stringify(sel)}).scrollIntoView({ block: "nearest" })`); await api.wait(60);
      const r2 = await api.eval(`(() => { const b = document.querySelector(${JSON.stringify(sel)}).getBoundingClientRect(); return [b.left + b.width / 2, b.top + b.height / 2]; })()`); r[0] = r2[0]; r[1] = r2[1];
      await api.click(r[0], r[1]);
    },
    async ax() { return (await send("Accessibility.getFullAXTree")).nodes; },
    async close() { try { await send("Browser.close"); } catch {} proc.kill(); },
  };
  return api;
}
export const MOD = { Alt: 1, Ctrl: 2, Meta: 4, Shift: 8 };
