// CI only, after a failed end-to-end run: starts Relay the way the harness
// does and reports, as annotations, what WebView2 did (runtime version,
// processes, listening ports), since CI logs need a sign-in to read.
import { spawn, execSync } from "node:child_process";
import { mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

const exe = resolve(process.env.RELAY_EXE ?? "target/debug/relay.exe");
const port = 9444;
const run = (cmd) => {
  try {
    return execSync(cmd, { encoding: "utf8" }).trim();
  } catch (e) {
    return `(failed: ${e.message.split("\n")[0]})`;
  }
};
const report = (title, text) =>
  console.log(`::warning title=${title}::${String(text).slice(0, 3000).replace(/%/g, "%25").replace(/\r?\n/g, "%0A")}`);

report(
  "WebView2 runtime",
  run('reg query "HKLM\\SOFTWARE\\WOW6432Node\\Microsoft\\EdgeUpdate\\Clients\\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}" /v pv') +
    "\n" +
    run('reg query "HKCU\\Software\\Microsoft\\EdgeUpdate\\Clients\\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}" /v pv'),
);
report("Session", run("query user") + "\n" + run("whoami"));

const proc = spawn(exe, [], {
  env: {
    ...process.env,
    RELAY_DATA_DIR: mkdtempSync(join(tmpdir(), "relay-diag-")),
    WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${port}`,
  },
  stdio: ["ignore", "pipe", "pipe"],
});
let out = "";
proc.stdout.on("data", (d) => (out += d));
proc.stderr.on("data", (d) => (out += d));
await new Promise((r) => setTimeout(r, 20_000));

report("WebView2 processes", run('tasklist /V /FI "IMAGENAME eq msedgewebview2.exe"'));
report("Relay process", run(`tasklist /V /FI "PID eq ${proc.pid}"`));
report("Listening ports", run("netstat -ano -p TCP | findstr LISTENING"));
let json;
try {
  json = await (await fetch(`http://127.0.0.1:${port}/json/version`)).text();
} catch (e) {
  json = `fetch failed: ${e.cause?.code ?? e.message}`;
}
report("DevTools endpoint", json);
report("Relay output", out || "(none)");
proc.kill();
process.exit(0);
