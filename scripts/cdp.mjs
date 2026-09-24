// Runs a script inside the running Relay webview over the Chrome DevTools
// Protocol, for end-to-end tests and documentation screenshots.
//
// Start Relay with remote debugging enabled first, for example in PowerShell:
//   $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = "--remote-debugging-port=9333"
//   $env:RELAY_DATA_DIR = "$env:TEMP\relay-e2e"
//   .\target\release\relay.exe
//
// Usage: node scripts/cdp.mjs <port> <file.js>
// The file's body runs as an async function: `await` works, `return` the result.
// `window.__relay` is the UI store; `window.__TAURI_INTERNALS__.invoke` calls commands.
import { readFileSync } from "node:fs";

const [port, file] = process.argv.slice(2);
if (!port || !file) {
  console.error("usage: node scripts/cdp.mjs <port> <file.js>");
  process.exit(2);
}
const targets = await (await fetch(`http://127.0.0.1:${port}/json`)).json();
const page = targets.find((t) => t.type === "page");
const ws = new WebSocket(page.webSocketDebuggerUrl);
await new Promise((r) => ws.addEventListener("open", r));
let id = 0;
const send = (method, params) =>
  new Promise((resolve) => {
    const my = ++id;
    ws.addEventListener("message", function on(ev) {
      const msg = JSON.parse(ev.data);
      if (msg.id === my) {
        ws.removeEventListener("message", on);
        resolve(msg);
      }
    });
    ws.send(JSON.stringify({ id: my, method, params }));
  });
const expr = `(async () => { ${readFileSync(file, "utf8")} })()`;
const res = await send("Runtime.evaluate", { expression: expr, awaitPromise: true, returnByValue: true });
const r = res.result;
if (r?.exceptionDetails) {
  console.log("EXCEPTION:", r.exceptionDetails.exception?.description ?? JSON.stringify(r.exceptionDetails));
  process.exitCode = 1;
} else console.log(JSON.stringify(r?.result?.value ?? r?.result, null, 2));
ws.close();
