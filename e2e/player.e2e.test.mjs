// Exported programs: Relay writes the player with the macro appended, and the
// program plays it on its own. The macros only wait (and check a pixel that
// never matches), so nothing is sent to the desktop; the programs are timed
// and their exit codes checked.
import { after, before, describe, test } from "node:test";
import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { join } from "node:path";
import { App, waitingMacro, writeRly } from "./harness.mjs";

/** Runs a program to its end: its exit code, how long it took, and what it wrote. */
function run(exe, args = []) {
  return new Promise((resolve, reject) => {
    const started = performance.now();
    const child = spawn(exe, args, { stdio: ["ignore", "pipe", "pipe"] });
    let stderr = "";
    child.stderr.on("data", (d) => (stderr += d));
    const timer = setTimeout(() => {
      child.kill();
      reject(new Error(`${exe} ${args.join(" ")} didn't end in 30 s`));
    }, 30_000);
    child.on("error", reject);
    child.on("exit", (code) => {
      clearTimeout(timer);
      resolve({ code, ms: performance.now() - started, stderr });
    });
  });
}

/** `ms` is between `low` and `high` (starting a program takes a moment). */
function about(ms, low, high, what) {
  assert.ok(ms >= low && ms <= high, `${what}: ${Math.round(ms)} ms, expected ${low}–${high}`);
}

const QUICK = ["--quiet", "--no-countdown"];

describe("exported programs", () => {
  const app = new App();
  let page;
  const programs = {};
  let saved;

  before(async () => {
    page = await app.start();
    const docs = {
      waits: waitingMacro({ name: "Waits", waits: [1500] }),
      twice: waitingMacro({ name: "Twice", waits: [1000], repeat: { count: 2 } }),
      pixel: waitingMacro({ name: "Pixel", waits: [200], pixel: { x: 0, y: 0, color: "#FE01FD" } }),
    };
    const paths = writeRly(app.path("sources"), Object.values(docs));
    const { imported } = await page.invoke("import_macros", { paths });
    for (const [i, key] of Object.keys(docs).entries()) {
      programs[key] = join(app.path("exports"), `${key}.exe`);
      await page.invoke("export_macro", { id: imported[i], format: "exe", path: programs[key] });
    }
    saved = app.macro(imported[0]);
  });
  after(() => app.dispose());

  test("a program plays its macro and exits with 0", async () => {
    const r = await run(programs.waits, QUICK);
    assert.equal(r.code, 0, r.stderr);
    about(r.ms, 1400, 3500, "a 1.5 s macro");
  });

  test("with its window, too", async () => {
    const r = await run(programs.waits, ["--no-countdown"]);
    assert.equal(r.code, 0, r.stderr);
    about(r.ms, 1400, 5000, "a 1.5 s macro, and the window closing after it");
  });

  test("it counts down 3 seconds first", async () => {
    const r = await run(programs.waits, ["--quiet"]);
    assert.equal(r.code, 0, r.stderr);
    about(r.ms, 4400, 6500, "3 s, then a 1.5 s macro");
  });

  test("it plays with the saved options, which options override", async () => {
    let r = await run(programs.twice, QUICK);
    assert.equal(r.code, 0, r.stderr);
    about(r.ms, 1900, 4000, "saved to play twice");
    r = await run(programs.waits, [...QUICK, "--repeat", "2"]);
    assert.equal(r.code, 0, r.stderr);
    about(r.ms, 2900, 5000, "--repeat 2");
    r = await run(programs.waits, [...QUICK, "--speed=2"]);
    assert.equal(r.code, 0, r.stderr);
    about(r.ms, 650, 2500, "--speed 2");
  });

  test("a pixel check that never matches ends it with 5", async () => {
    const r = await run(programs.pixel, QUICK);
    assert.equal(r.code, 5, r.stderr);
    about(r.ms, 900, 3500, "200 ms, then an 800 ms timeout");
  });

  test("bad options end it with 2 and say what's wrong", async () => {
    for (const [args, message] of [
      [["--repeat", "0"], "--repeat must be a number of times (1 or more) or forever"],
      [["--speed", "abc"], "--speed must be a number from 0.01 to 100"],
      [["--bogus"], "Unknown option: --bogus"],
    ]) {
      const r = await run(programs.waits, ["--quiet", ...args]);
      assert.equal(r.code, 2, `${args}: ${r.stderr}`);
      assert.ok(r.stderr.includes(message), r.stderr);
      assert.ok(r.ms < 3000, `${args} ended at once: ${Math.round(r.ms)} ms`);
    }
  });

  test("--help lists the options", async () => {
    const r = await run(programs.waits, ["--help"]);
    assert.equal(r.code, 0);
    assert.ok(r.stderr.includes("--repeat N|forever") && r.stderr.includes("Exit codes:"), r.stderr);
  });

  test("Import reads the macro back from the program", async () => {
    const { imported, problems } = await page.invoke("import_macros", { paths: [programs.waits] });
    assert.deepEqual(problems, []);
    const back = app.macro(imported[0]);
    assert.deepEqual(back.events, saved.events);
    assert.deepEqual(back.playback, saved.playback);
    assert.equal(back.name, "Waits 2");
  });

  test("Import refuses another program", async () => {
    const ping = join(process.env.SystemRoot ?? "C:\\Windows", "System32", "PING.EXE");
    const { imported, problems } = await page.invoke("import_macros", { paths: [ping] });
    assert.deepEqual(imported, []);
    assert.deepEqual(problems, ["PING.EXE: not a program exported by Relay"]);
  });
});
