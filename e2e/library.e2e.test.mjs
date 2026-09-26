// The library against the real app: the first run, renaming, duplicating,
// deleting and restoring, importing and exporting, and what's on disk
// after a restart.
import { after, before, describe, test } from "node:test";
import assert from "node:assert/strict";
import { mkdirSync, readFileSync, writeFileSync, existsSync } from "node:fs";
import { join } from "node:path";
import { App, until, waitingMacro, writeRly } from "./harness.mjs";

describe("library", () => {
  const app = new App();
  let page;
  before(async () => {
    page = await app.start();
  });
  after(() => app.dispose());

  const names = () =>
    page.run(async () => {
      await window.__relay.refreshLibrary();
      return window.__relay.library.map((m) => m.name);
    });
  const id = () => page.store("view.id");

  test("the first run seeds the design's four samples, with their hotkeys off", async () => {
    assert.deepEqual(await names(), ["Export invoice to PDF", "Fill weekly timesheet", "Batch rename photos", "Open standup tools"]);
    assert.equal(app.macroFiles().length, 4);
    const index = app.json("library.json");
    assert.equal(index.order.length, 4);
    for (const e of Object.values(index.entries)) assert.equal(e.triggers?.hotkey?.enabled ?? false, false);
    // Not in the Library's hotkey column; the Triggers tab has the combo, switched off.
    await page.click("Library", { role: "tab" });
    await page.idle();
    assert.doesNotMatch(await page.text(".list"), /Ctrl \+ Alt/);
    await page.click("Triggers", { role: "tab" });
    await page.idle();
    assert.match(await page.text(".list"), /Ctrl \+ Alt \+ 1/);
    assert.equal(await page.run(() => document.querySelector('[aria-label="Hotkey trigger"]').getAttribute("aria-checked")), "false");
  });

  test("renaming in the header is saved to the macro's file", async () => {
    const macro = await id();
    await page.fill('input[aria-label="Macro name"]', "Invoice → PDF (renamed)");
    await until(() => app.macro(macro).name === "Invoice → PDF (renamed)", { what: "the rename on disk" });
    assert.equal((await names())[0], "Invoice → PDF (renamed)");
  });

  test("Duplicate makes a copy right after it, in its own file", async () => {
    await page.click("Library", { role: "tab" });
    await page.click("Duplicate Fill weekly timesheet");
    await until(async () => (await page.store("view.name")) === "Fill weekly timesheet (copy)", { what: "the copy to open" });
    const list = await names();
    assert.equal(list.length, 5);
    assert.equal(list[2], "Fill weekly timesheet (copy)");
    assert.equal(await page.store("tab"), "library");
    const copy = await id();
    assert.equal(app.macro(copy).name, "Fill weekly timesheet (copy)");
    assert.deepEqual(app.macro(copy).events, app.macro(await page.store("library.1.id")).events);
    assert.equal(app.macroFiles().length, 5);
  });

  test("Delete moves it to the trash folder; Undo brings it back where it was", async () => {
    await page.click("Library", { role: "tab" });
    const victim = (await page.run(() => window.__relay.library[2])).id;
    await page.click("Delete Fill weekly timesheet (copy)");
    await until(async () => (await page.store("toast.message"))?.startsWith("Moved"), { what: "the delete" });
    assert.equal((await names()).length, 4);
    assert.ok(!app.exists("macros", `${victim}.rly`));
    assert.ok(app.exists("macros", ".trash", `${victim}.rly`));
    assert.ok(victim in app.json("library.json").trash);
    assert.equal(await page.store("toast.message"), "Moved “Fill weekly timesheet (copy)” to the trash");

    await page.click("Undo", { within: ".toast" });
    await until(async () => (await page.store("library.length")) === 5 && (await page.store("toast")) == null, { what: "the restore" });
    assert.equal((await names())[2], "Fill weekly timesheet (copy)");
    assert.ok(app.exists("macros", `${victim}.rly`));
    assert.ok(!(victim in app.json("library.json").trash));
  });

  test("deleting the open macro opens its neighbour", async () => {
    await page.run(() => window.__relay.loadMacro(window.__relay.library[2].id));
    await page.click("Library", { role: "tab" }); // opening it switched to Steps
    await page.idle();
    await page.click("Delete Fill weekly timesheet (copy)");
    await until(async () => (await page.store("view.name")) === "Batch rename photos", { what: "the neighbour" });
  });

  test("import: .rly and .json files at the top, a broken one explained, names kept unique", async () => {
    const dir = app.path("to-import");
    const a = waitingMacro({ name: "Imported waits" });
    const [good, twin] = writeRly(dir, [a, { ...a }]);
    const broken = join(dir, "broken.rly");
    writeFileSync(broken, "{ not a macro");
    const exportedJson = join(dir, "exported.json");
    writeFileSync(exportedJson, JSON.stringify({ ...waitingMacro({ name: "From JSON" }), steps: [] }, null, 2));

    const result = await page.invoke("import_macros", { paths: [good, twin, broken, exportedJson] });
    assert.equal(result.imported.length, 3);
    assert.equal(result.problems.length, 1);
    assert.match(result.problems[0], /^broken\.rly: /);
    const list = await page.run(async () => {
      await window.__relay.refreshLibrary();
      return window.__relay.library.map((m) => m.name);
    });
    assert.deepEqual(list.slice(0, 3), ["Imported waits", "Imported waits 2", "From JSON"]);
    // The second copy got a new id, since the first took the file's.
    assert.ok(result.imported.includes(a.id));
    assert.equal(new Set(result.imported).size, 3);
    for (const i of result.imported) assert.ok(app.exists("macros", `${i}.rly`));
  });

  test("a file that isn't a Relay macro, or is from a newer Relay, is refused", async () => {
    const dir = app.path("to-import");
    const [other, newer] = writeRly(dir, [
      JSON.stringify({ hello: "world" }),
      JSON.stringify({ ...waitingMacro(), version: 99 }),
    ]);
    const result = await page.invoke("import_macros", { paths: [other, newer, join(dir, "missing.rly")] });
    assert.deepEqual(result.imported, []);
    assert.equal(result.problems.length, 3);
    assert.match(result.problems[1], /newer/i);
  });

  test("export as .rly and as .json: both import again unchanged", async () => {
    const macro = await id();
    const out = app.path("exports");
    mkdirSync(out, { recursive: true });
    const rly = join(out, "copy.rly");
    const json = join(out, "copy.json");
    await page.invoke("export_macro", { id: macro, format: "rly", path: rly });
    await page.invoke("export_macro", { id: macro, format: "json", path: json });
    const saved = app.macro(macro);
    const fromRly = JSON.parse(readFileSync(rly, "utf8"));
    const fromJson = JSON.parse(readFileSync(json, "utf8"));
    assert.deepEqual(fromRly.events, saved.events);
    assert.deepEqual(fromJson.events, saved.events);
    assert.ok(Array.isArray(fromJson.steps) && fromJson.steps.length > 0, "the JSON export includes the steps");
    assert.ok(readFileSync(json, "utf8").includes("\n  "), "and is pretty-printed");
    const back = await page.invoke("import_macros", { paths: [rly, json] });
    assert.equal(back.problems.length, 0);
    for (const i of back.imported) assert.deepEqual(app.macro(i).events, saved.events);
  });

  test("export to a folder that can't be written reports it", async () => {
    await assert.rejects(
      page.invoke("export_macro", { id: await id(), format: "rly", path: app.path("macros") }),
      (e) => e.code === "io",
    );
  });

  test("commands on a macro that doesn't exist fail as not_found", async () => {
    const ghost = "00000000-0000-0000-0000-00000000dead";
    for (const [cmd, args] of [
      ["load_macro", { id: ghost }],
      ["edit_macro", { id: ghost, op: { op: "rename", name: "x" } }],
      ["duplicate_macro", { id: ghost }],
      ["delete_macro", { id: ghost }],
      ["restore_macro", { id: ghost }],
      ["get_triggers", { id: ghost }],
    ]) {
      await assert.rejects(page.invoke(cmd, args), (e) => e.code === "not_found", cmd);
    }
  });

  test("everything survives a restart, in the same order", async () => {
    const before = await names();
    const current = await page.store("view.name");
    page = await app.restart();
    assert.deepEqual(await names(), before);
    assert.equal(await page.store("view.name"), before[0]);
    assert.ok(before.includes(current));
    assert.equal(await page.store("library.1.name"), before[1]);
  });

  test("a damaged macro file is skipped, not deleted, and the rest load", async () => {
    await app.quit();
    const bad = app.path("macros", "11111111-2222-3333-4444-555555555555.rly");
    writeFileSync(bad, "garbage");
    page = await app.start();
    const list = await names();
    assert.ok(list.length >= 5);
    assert.ok(existsSync(bad), "left for the user to inspect");
  });

  test("a damaged library.json is set aside and the macros still load", async () => {
    await app.quit();
    writeFileSync(app.path("library.json"), "{ broken");
    page = await app.start();
    assert.ok(app.exists("library.json.bad"));
    assert.ok((await names()).length >= 5);
  });
});
