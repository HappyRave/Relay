// The Library: its macros (filters, search, sort, several selected, the row
// menu and keys) and Activity (the run history and its problems).
import { beforeEach, describe, expect, test } from "vitest";
import { fireEvent, render, screen, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import LibraryPage from "./LibraryPage.svelte";
import { core, freshStore, settle } from "../../test/app";
import { defaultTriggers } from "../../test/fake-core";
import { browserBackend } from "../../lib/ipc/backend";
import type { RelayStore } from "../../lib/state/relay.svelte";
import type { RunEntry } from "../../lib/ipc/bindings/RunEntry";

let relay: RelayStore;
const [A, B, C, D] = [1, 2, 3, 4].map((n) => `00000000-0000-0000-0000-00000000000${n}`);

const run = (over: Partial<RunEntry> = {}): RunEntry => ({
  at: new Date(Date.now() - 60_000).toISOString(),
  macro_id: A,
  macro_name: "Export invoice to PDF",
  source: "hotkey",
  outcome: { type: "finished", reason: "completed" },
  duration_ms: 31_840,
  from_ms: 0,
  loops: 3,
  speed: 1,
  humanize: false,
  checks: [
    { step: 8, loop_idx: 0, image: false, after_ms: 640, outcome: { type: "matched" } },
    { step: 8, loop_idx: 1, image: false, after_ms: 910, outcome: { type: "matched" } },
  ],
  checks_dropped: 0,
  ...over,
});
const history = [
  run(),
  run({
    at: new Date(Date.now() - 120_000).toISOString(),
    macro_id: B,
    macro_name: "Fill weekly timesheet",
    source: "schedule",
    outcome: { type: "skipped", reason: "busy" },
    duration_ms: 0,
    loops: 0,
    checks: [],
  }),
  run({
    at: new Date(Date.now() - 180_000).toISOString(),
    source: "manual",
    outcome: { type: "finished", reason: "pixel_timeout" },
    duration_ms: 7200,
    loops: 1,
    from_ms: 3200,
    speed: 2,
    checks: [{ step: 8, loop_idx: 0, image: false, after_ms: 5000, outcome: { type: "timed_out" } }],
    checks_dropped: 4,
  }),
];

beforeEach(async () => {
  relay = await freshStore({ init: false });
  core.runLog = structuredClone(history);
  await relay.init();
  await settle();
  core.clearCalls();
});

const button = (name: string | RegExp) => screen.getByRole("button", { name });
const macroRows = () => within(screen.getByRole("table", { name: "Macros" })).getAllByRole("row").slice(1);
const names = () => macroRows().map((r) => r.querySelector(".mname b")!.textContent);
/** The row of the macro named `name`, and the button that opens it. */
const rowOf = (name: string) => macroRows().find((r) => r.querySelector(".mname b")?.textContent === name)!;
const opener = (name: string) => rowOf(name).querySelector<HTMLElement>(".mname")!;
const nav = (name: string | RegExp) => within(screen.getByRole("navigation", { name: "Library" })).getByRole("button", { name });

describe("the macros", () => {
  test("a row each: name, length, steps and runs, hotkey, what else runs it, and the last run", async () => {
    const t = defaultTriggers();
    await relay.setTriggers({ hotkey: { enabled: true, combo: "Ctrl + Alt + 1" }, schedule: { ...t.schedule, enabled: true } });
    await relay.refreshLibrary();
    render(LibraryPage);
    const first = rowOf("Export invoice to PDF");
    expect(first).toHaveTextContent("10.2 s · 18 steps · 148 runs");
    expect(within(first).getByRole("img", { name: "Ctrl + Alt + 1" })).toBeInTheDocument();
    expect(within(first).getByRole("button", { name: "Mon–Fri at 09:00: open its triggers" })).toBeInTheDocument();
    // Its newest run in the history completed.
    expect(first.querySelector(".ok")).toHaveAttribute("title", "Completed");
  });

  test("the last run's outcome comes from the history: a failure in red", async () => {
    core.runLog = [history[2]];
    await relay.refreshRuns();
    render(LibraryPage);
    expect(rowOf("Export invoice to PDF").querySelector(".bad")).toHaveAttribute("title", "Pixel check timed out");
    // A run the history no longer has: when, without how it went.
    expect(rowOf("Fill weekly timesheet").querySelector(".ok")).not.toHaveAttribute("title");
  });

  test("a macro never run says so", async () => {
    await relay.duplicateMacro(A);
    render(LibraryPage);
    expect(macroRows().find((r) => r.textContent?.includes("(copy)"))).toHaveTextContent("Never");
  });

  test("Also runs opens the macro on its triggers", async () => {
    const t = defaultTriggers();
    await relay.setTriggers({ app_launch: { enabled: true, exe: "EXCEL.EXE", delay_ms: 0 } });
    void t;
    await relay.refreshLibrary();
    render(LibraryPage);
    await userEvent.click(button("When EXCEL.EXE starts: open its triggers"));
    await settle();
    expect([relay.place, relay.inspector, relay.macroSection]).toEqual(["macro", "macro", "triggers"]);
  });

  test("a click on a name opens the macro; Enter too", async () => {
    render(LibraryPage);
    await userEvent.click(opener("Fill weekly timesheet"));
    await settle();
    expect(relay.view?.id).toBe(B);
    expect(relay.place).toBe("macro");
  });

  test("search, and sort by name, newest or last run", async () => {
    render(LibraryPage);
    expect(names()).toEqual(["Batch rename photos", "Export invoice to PDF", "Fill weekly timesheet", "Open standup tools"]);
    await userEvent.click(screen.getByRole("radio", { name: "Last run" }));
    expect(names()[0]).toBe("Export invoice to PDF");
    await relay.duplicateMacro(D);
    await settle();
    await userEvent.click(screen.getByRole("radio", { name: "Newest" }));
    expect(names()[0]).toBe("Open standup tools (copy)");
    await userEvent.type(screen.getByRole("searchbox", { name: "Search macros" }), "PHOTO");
    expect(names()).toEqual(["Batch rename photos"]);
    await userEvent.type(screen.getByRole("searchbox", { name: "Search macros" }), "zz");
    expect(screen.getByText("No macros match “PHOTOzz”")).toBeInTheDocument();
  });

  test("Ctrl + F searches", async () => {
    render(LibraryPage);
    await userEvent.keyboard("{Control>}f{/Control}");
    expect(screen.getByRole("searchbox", { name: "Search macros" })).toHaveFocus();
  });

  test("the side's filters: with triggers, recently run, all; each with its count", async () => {
    await relay.setTriggers({ hotkey: { enabled: true, combo: "Ctrl + Alt + 1" } });
    core.entries[1].last_run = new Date(Date.now() - 3_600_000).toISOString(); // an hour ago
    await relay.refreshLibrary();
    render(LibraryPage);
    expect(nav(/^All ?[0-9]/)).toHaveTextContent("All4");
    expect(nav(/With triggers/)).toHaveTextContent("1");
    await userEvent.click(nav(/With triggers/));
    expect(names()).toEqual(["Export invoice to PDF"]);
    expect(nav(/With triggers/)).toHaveAttribute("aria-current", "page");
    await userEvent.click(nav(/Recently run/));
    expect(names()).toEqual(["Fill weekly timesheet"]);
    await userEvent.click(nav(/^All ?[0-9]/));
    expect(names()).toHaveLength(4);
  });

  test("the row's menu: Open, Play, Duplicate, Export, Runs and Delete", async () => {
    render(LibraryPage);
    await userEvent.click(button("More for Fill weekly timesheet"));
    const menu = within(screen.getByRole("menu", { name: "More for Fill weekly timesheet" }));
    expect(menu.getAllByRole("menuitem").map((m) => m.textContent?.replace(/(Ctrl \+ D|Del)$/, "").trim())).toEqual([
      "Open",
      "Play",
      "Duplicate",
      "Export…",
      "Runs",
      "Delete",
    ]);
    await userEvent.click(menu.getByRole("menuitem", { name: "Play" }));
    await settle();
    expect(core.calls.map((c) => c.cmd)).toContain("load_macro");
    expect(core.lastArgs("toggle_play")).toEqual({ from: 0 });
    expect(relay.place).toBe("library");
  });

  test("Export… from a row opens the export dialog for that macro", async () => {
    render(LibraryPage);
    await userEvent.click(button("More for Batch rename photos"));
    await userEvent.click(screen.getByRole("menuitem", { name: "Export…" }));
    await settle();
    expect([relay.view?.id, relay.exportOpen]).toEqual([C, true]);
  });

  test("Runs shows that macro's runs", async () => {
    render(LibraryPage);
    await userEvent.click(button("More for Fill weekly timesheet"));
    await userEvent.click(screen.getByRole("menuitem", { name: "Runs" }));
    await settle();
    expect(relay.libraryShows).toBe("runs");
    expect(screen.getByRole("combobox", { name: "Show the runs of" })).toHaveValue(B);
  });

  test("in a row, the arrow keys go to the next one, Del deletes and Ctrl + D duplicates", async () => {
    render(LibraryPage);
    opener("Batch rename photos").focus();
    await userEvent.keyboard("{ArrowDown}");
    expect(opener("Export invoice to PDF")).toHaveFocus();
    await userEvent.keyboard("{Control>}d{/Control}");
    await settle();
    expect(core.lastArgs("duplicate_macro")).toEqual({ id: A });
    opener("Batch rename photos").focus();
    await userEvent.keyboard("{Delete}");
    await settle();
    expect(core.lastArgs("delete_macro")).toEqual({ id: C });
    expect(relay.toast).toMatchObject({ message: "Moved “Batch rename photos” to the trash" });
  });

  test("several selected: export them to a folder, or delete them with one Undo", async () => {
    render(LibraryPage);
    await userEvent.click(screen.getByRole("checkbox", { name: "Select Fill weekly timesheet" }));
    await userEvent.click(screen.getByRole("checkbox", { name: "Select Open standup tools" }));
    const bulk = within(screen.getByRole("group", { name: "Selected macros" }));
    expect(bulk.getByText("2 macros selected")).toBeInTheDocument();

    core.dialog.open = "C:\\Exports";
    await userEvent.click(bulk.getByRole("button", { name: "Export…" }));
    await settle();
    expect(core.argsOf("export_macro")).toEqual([
      { id: B, format: "rly", path: "C:\\Exports\\fill-weekly-timesheet.rly" },
      { id: D, format: "rly", path: "C:\\Exports\\open-standup-tools.rly" },
    ]);
    expect(relay.toast).toMatchObject({ message: "Exported 2 macros to Exports" });

    await userEvent.click(bulk.getByRole("button", { name: "Delete" }));
    await settle();
    expect(core.argsOf("delete_macro")).toEqual([{ id: B }, { id: D }]);
    expect(names()).toEqual(["Batch rename photos", "Export invoice to PDF"]);
    expect(screen.queryByRole("group", { name: "Selected macros" })).toBeNull();
    expect(relay.toast).toMatchObject({ message: "Moved 2 macros to the trash" });
    relay.toast!.action!.run();
    await settle();
    expect(names()).toHaveLength(4);
  });

  test("Select all, and Clear", async () => {
    render(LibraryPage);
    await userEvent.click(screen.getByRole("checkbox", { name: "Select all" }));
    expect(relay.libSelection).toHaveLength(4);
    await userEvent.click(button("Clear"));
    expect(relay.libSelection).toEqual([]);
  });

  test("Del on a selected row deletes the selection", async () => {
    render(LibraryPage);
    await userEvent.click(screen.getByRole("checkbox", { name: "Select Batch rename photos" }));
    await userEvent.click(screen.getByRole("checkbox", { name: "Select Open standup tools" }));
    opener("Batch rename photos").focus();
    await userEvent.keyboard("{Delete}");
    await settle();
    expect(core.argsOf("delete_macro")).toEqual([{ id: C }, { id: D }]);
  });

  test("the open macro shows the name being typed", async () => {
    render(LibraryPage);
    relay.rename("Typing…");
    await settle();
    expect(macroRows().some((r) => r.textContent?.includes("Typing…"))).toBe(true);
  });

  test("an empty Library says what Relay does, and records", async () => {
    core.entries = [];
    await relay.refreshLibrary();
    render(LibraryPage);
    expect(screen.getByText("Record what you do once. Relay plays it back.")).toBeInTheDocument();
    await userEvent.click(within(screen.getByRole("main")).getByRole("button", { name: "Record" }));
    expect(core.commands()).toContain("toggle_record");
  });

  test("Import… asks for files and imports them", async () => {
    render(LibraryPage);
    core.dialog.open = ["C:\\macros\\Weekly report.rly"];
    await userEvent.click(nav(/Import/));
    await settle();
    expect(core.argsOf("import_macros")).toEqual([{ paths: ["C:\\macros\\Weekly report.rly"] }]);
    expect(names()).toContain("Weekly report");
    expect(relay.toast).toMatchObject({ kind: "info", message: "Imported 1 macro" });
  });

  test("during a session, nothing changes the Library", async () => {
    render(LibraryPage);
    core.emit({ type: "session", mode: "playing", macro_id: A, source: null });
    await settle();
    expect(nav(/Import/)).toBeDisabled();
    await userEvent.click(button("More for Fill weekly timesheet"));
    for (const name of ["Play", "Duplicate", "Export…", "Delete"]) expect(screen.getByRole("menuitem", { name: new RegExp(`^${name}`) })).toBeDisabled();
  });

  test("triggers paused: Resume", async () => {
    core.emit({ type: "triggers_paused", paused: true });
    render(LibraryPage);
    await settle();
    expect(screen.getByText("Triggers are paused (the kill switch).")).toBeInTheDocument();
    await userEvent.click(button("Resume"));
    await settle();
    expect(core.argsOf("set_triggers_paused")).toEqual([{ paused: false }]);
  });

  test("Record, the player and Settings in the top bar", async () => {
    render(LibraryPage);
    await userEvent.click(button("Record"));
    expect(core.commands()).toContain("toggle_record");
    await userEvent.click(button("Settings"));
    expect(relay.place).toBe("settings");
    await userEvent.click(button("Switch to the player"));
    await settle();
    expect(core.lastArgs("set_player_mode")).toEqual({ on: true });
  });

  test("the browser preview can't change the Library", async () => {
    core.uninstall();
    try {
      await freshStore({ backend: browserBackend() });
      render(LibraryPage);
      await settle();
      expect(screen.queryByRole("button", { name: /Import/ })).toBeNull();
      await userEvent.click(button("More for Fill weekly timesheet"));
      expect(screen.getByRole("menuitem", { name: /^Delete/ })).toBeDisabled();
    } finally {
      core.install();
    }
  });
});

describe("activity", () => {
  const runRows = () => screen.getAllByRole("button").filter((b) => b.classList.contains("head"));
  async function openRuns(which: RegExp = /All runs/) {
    render(LibraryPage);
    await userEvent.click(nav(which));
    await settle();
  }

  test("All runs, newest first: what ran it, when, how it ended and how long it took", async () => {
    await openRuns();
    const rows = runRows();
    expect(rows).toHaveLength(3);
    expect(rows[0]).toHaveTextContent("Export invoice to PDF");
    expect(rows[0]).toHaveTextContent(/Hotkey · Today, \d\d:\d\d · 3 loops/);
    expect(rows[0]).toHaveTextContent("Completed");
    expect(rows[0]).toHaveTextContent("31.8 s");
    expect(rows[1]).toHaveTextContent("Skipped: Relay was busy");
    expect(rows[1]).not.toHaveTextContent(" s"); // a skip took no time
    expect(rows[2]).toHaveTextContent("Pixel check timed out");
    expect(screen.getByText("Relay keeps the last 200 runs.")).toBeInTheDocument();
  });

  test("a run opens to what it did; one at a time", async () => {
    await openRuns();
    const [first, , timedOut] = runRows();
    await userEvent.click(first);
    expect(first).toHaveAttribute("aria-expanded", "true");
    expect(first.parentElement).toHaveTextContent("Loop 1 · Step 8 · Pixel check matched after 0.6 s");
    expect(first.parentElement).toHaveTextContent("Loop 2 · Step 8 · Pixel check matched after 0.9 s");
    await userEvent.click(timedOut);
    expect(first).toHaveAttribute("aria-expanded", "false");
    const detail = timedOut.parentElement!;
    expect(detail).toHaveTextContent("From 00:03.20 · 2× speed");
    expect(detail).toHaveTextContent("4 earlier checks not kept");
    expect(within(detail).getByText("Step 8 · Pixel check timed out after 5.0 s")).toHaveClass("bad");
  });

  test("Problems: the runs that didn't end as planned, with a count", async () => {
    render(LibraryPage);
    expect(within(nav(/Problems/)).getByLabelText("2 problems")).toHaveTextContent("2");
    await userEvent.click(nav(/Problems/));
    await settle();
    expect(runRows().map((r) => r.querySelector("b")?.textContent)).toEqual(["Fill weekly timesheet", "Export invoice to PDF"]);
    expect(screen.getByRole("heading", { name: "Problems" })).toBeInTheDocument();
  });

  test("Open at step N opens the macro on the step it timed out on", async () => {
    await openRuns(/Problems/);
    await userEvent.click(runRows()[1]);
    await userEvent.click(button("Open at step 8")); // the pixel check: step 8, moves not counted
    await settle();
    expect([relay.place, relay.view?.id, relay.selected, relay.failedStep]).toEqual(["macro", A, 11, 11]);
  });

  test("a run from a data file says which row", async () => {
    core.runLog = [run({ loops: 112, rows: 200, outcome: { type: "finished", reason: "pixel_timeout" }, checks: [{ step: 8, loop_idx: 111, image: false, after_ms: 5000, outcome: { type: "timed_out" } }] })];
    await relay.refreshRuns();
    await openRuns();
    expect(runRows()[0]).toHaveTextContent("200 rows");
    await userEvent.click(runRows()[0]);
    expect(screen.getByText("Row 112 of 200 · Step 8 · Pixel check timed out after 5.0 s")).toBeInTheDocument();
  });

  test("the filter shows one macro's runs", async () => {
    await openRuns();
    const filter = screen.getByRole("combobox", { name: "Show the runs of" });
    expect(within(filter).getAllByRole("option").map((o) => o.textContent)[0]).toBe("Every macro");
    await userEvent.selectOptions(filter, "Fill weekly timesheet");
    expect(runRows()).toHaveLength(1);
    await userEvent.selectOptions(filter, "Open standup tools");
    expect(screen.getByText("No runs yet")).toBeInTheDocument();
  });

  test("a new run shows up, and none at all says so", async () => {
    await openRuns();
    core.runLog = [run({ at: new Date().toISOString(), macro_name: "Just now", checks: [] }), ...history];
    core.emit({ type: "runs_changed" });
    await settle();
    expect(runRows()[0]).toHaveTextContent("Just now");
    core.runLog = [];
    core.emit({ type: "runs_changed" });
    await settle();
    expect(screen.getByText("No runs yet")).toBeInTheDocument();
    await userEvent.click(nav(/Problems/));
    await settle();
    expect(screen.getByText("No problems")).toBeInTheDocument();
  });

  test("a slow answer never replaces a newer one", async () => {
    core.hold("list_runs");
    const older = relay.refreshRuns();
    const newer = relay.refreshRuns();
    const [first, second] = core.held;
    second.resolve([run({ macro_name: "Newer" })]);
    await newer;
    first.resolve([run({ macro_name: "Older" })]);
    await older;
    expect(relay.runs.map((r) => r.macro_name)).toEqual(["Newer"]);
  });
});

void fireEvent;
