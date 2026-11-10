// Runs a script inside a running Relay window over the Chrome DevTools
// Protocol, for checks by hand and documentation screenshots.
//
// Start Relay with remote debugging enabled first, for example in PowerShell:
//   $env:RELAY_DEVTOOLS_PORT = "9333"
//   $env:RELAY_DATA_DIR = "$env:TEMP\relay-dev"
//   .\target\release\relay.exe
//
// Usage: node scripts/cdp.mjs <port> <file.js> [window]
// The file's body runs as an async function: `await` works, `return` the result.
// It runs in the main window, or in the one whose label is given ("player").
// `window.__relay` is the UI store; `window.__TAURI_INTERNALS__.invoke` calls commands.
import { readFileSync } from "node:fs";

const [port, file, label = "main"] = process.argv.slice(2);
if (!port || !file) {
  console.error("usage: node scripts/cdp.mjs <port> <file.js> [window]");
  process.exit(2);
}

let id = 0;
let ws = null;
function send(method, params) {
  const my = ++id;
  return new Promise((resolve) => {
    ws.addEventListener("message", function on(ev) {
      const msg = JSON.parse(ev.data);
      if (msg.id === my) {
        ws.removeEventListener("message", on);
        resolve(msg);
      }
    });
    ws.send(JSON.stringify({ id: my, method, params }));
  });
}

const targets = await (await fetch(`http://127.0.0.1:${port}/json`)).json();
for (const t of targets.filter((t) => t.type === "page" && !t.url.startsWith("devtools"))) {
  ws = new WebSocket(t.webSocketDebuggerUrl);
  await new Promise((r) => ws.addEventListener("open", r));
  const res = await send("Runtime.evaluate", {
    expression: "window.__TAURI_INTERNALS__?.metadata.currentWindow.label",
    returnByValue: true,
  });
  if (res.result?.result?.value === label) break;
  ws.close();
  ws = null;
}
if (!ws) {
  console.error(`no "${label}" window`);
  process.exit(1);
}

const expr = `(async () => { ${readFileSync(file, "utf8")} })()`;
const res = await send("Runtime.evaluate", { expression: expr, awaitPromise: true, returnByValue: true });
const r = res.result;
if (r?.exceptionDetails) {
  console.log("EXCEPTION:", r.exceptionDetails.exception?.description ?? JSON.stringify(r.exceptionDetails));
  process.exitCode = 1;
} else console.log(JSON.stringify(r?.result?.value ?? r?.result, null, 2));
ws.close();
