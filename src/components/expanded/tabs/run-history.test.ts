// Library → Runs: the run history.
import { beforeEach, describe, expect, test } from "vitest";
import { render, screen, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import LibraryTab from "./LibraryTab.svelte";
import { core, freshStore, settle } from "../../../test/app";
import type { RelayStore } from "../../../lib/state/relay.svelte";
import type { RunEntry } from "../../../lib/ipc/bindings/RunEntry";

let relay: RelayStore;
const [A, B] = [1, 2].map((n) => `00000000-0000-0000-0000-00000000000${n}`);

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
    { step: 12, loop_idx: 0, image: false, after_ms: 640, outcome: { type: "matched" } },
    { step: 12, loop_idx: 1, image: false, after_ms: 910, outcome: { type: "matched" } },
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
    checks: [{ step: 7, loop_idx: 0, image: true, after_ms: 5000, outcome: { type: "timed_out" } }],
    checks_dropped: 4,
  }),
];

beforeEach(async () => {
  relay = await freshStore();
  core.runLog = structuredClone(history);
});

const runRows = () => screen.getAllByRole("button").filter((b) => b.classList.contains("head"));

async function openRuns() {
  render(LibraryTab);
  core.clearCalls();
  await userEvent.click(screen.getByRole("button", { name: "Runs" }));
  await settle();
}

describe("Run history", () => {
  test("Runs loads the history and lists it newest first", async () => {
    await openRuns();
    expect(core.calls).toEqual([{ cmd: "list_runs", args: {} }]);
    const rows = runRows();
    expect(rows).toHaveLength(3);
    expect(rows[0]).toHaveTextContent("Export invoice to PDF");
    expect(rows[0]).toHaveTextContent(/Today, \d\d:\d\d/);
    expect(rows[0]).toHaveTextContent("Hotkey · Completed · 3 loops");
    expect(rows[0]).toHaveTextContent("31.8 s");
    expect(rows[1]).toHaveTextContent("Schedule · Skipped: Relay was busy");
    expect(rows[1]).not.toHaveTextContent(" s"); // a skip took no time
    expect(rows[2]).toHaveTextContent("Play · Image not found");
    expect(rows[0].parentElement).not.toHaveClass("failed");
    expect(rows[1].parentElement).toHaveClass("failed");
    expect(rows[2].parentElement).toHaveClass("failed");
    expect(screen.getByText("Relay keeps the last 200 runs.")).toBeInTheDocument();
  });

  test("a run opens to show its checks and settings; a skip has nothing to open", async () => {
    await openRuns();
    const [first, skip, timedOut] = runRows();
    expect(first).toHaveAttribute("aria-expanded", "false");
    await userEvent.click(first);
    expect(first).toHaveAttribute("aria-expanded", "true");
    const details = within(first.parentElement!).getAllByRole("listitem").map((li) => li.textContent);
    expect(details).toEqual([
      "Loop 1 · Step 12 · Pixel check matched after 0.6 s",
      "Loop 2 · Step 12 · Pixel check matched after 0.9 s",
    ]);
    expect(skip).toBeDisabled();
    expect(skip).not.toHaveAttribute("aria-expanded");
    // One run open at a time.
    await userEvent.click(timedOut);
    expect(first).toHaveAttribute("aria-expanded", "false");
    const more = within(timedOut.parentElement!).getAllByRole("listitem");
    expect(more.map((li) => li.textContent)).toEqual([
      "From 00:03.20 · 2× speed",
      "4 earlier checks not kept",
      "Step 7 · Image not found in 5.0 s",
    ]);
    expect(more[2]).toHaveClass("bad");
    await userEvent.click(timedOut);
    expect(timedOut).toHaveAttribute("aria-expanded", "false");
  });

  test("the filter shows one macro's runs", async () => {
    await openRuns();
    const filter = screen.getByRole("combobox", { name: "Show the runs of" });
    expect(within(filter).getAllByRole("option").map((o) => o.textContent)).toEqual([
      "All macros",
      "Export invoice to PDF",
      "Fill weekly timesheet",
      "Batch rename photos",
      "Open standup tools",
    ]);
    await userEvent.selectOptions(filter, "Fill weekly timesheet");
    expect(runRows()).toHaveLength(1);
    expect(runRows()[0]).toHaveTextContent("Skipped: Relay was busy");
    await userEvent.selectOptions(filter, "Open standup tools");
    expect(screen.getByText("No runs yet.")).toBeInTheDocument();
    await userEvent.selectOptions(filter, "All macros");
    expect(runRows()).toHaveLength(3);
  });

  test("an empty history says so", async () => {
    core.runLog = [];
    await openRuns();
    expect(screen.getByText("No runs yet.")).toBeInTheDocument();
  });

  test("a new run shows up while it's open, and isn't fetched while it's closed", async () => {
    await openRuns();
    core.runLog = [run({ at: new Date().toISOString(), macro_name: "Just now", checks: [] }), ...history];
    core.clearCalls();
    core.emit({ type: "runs_changed" });
    await settle();
    expect(core.calls).toEqual([{ cmd: "list_runs", args: {} }]);
    expect(runRows()[0]).toHaveTextContent("Just now");
    await userEvent.click(screen.getByRole("button", { name: "Macros" }));
    expect(relay.runsOpen).toBe(false);
    expect(screen.getByText("New recordings are saved here automatically.")).toBeInTheDocument();
    core.clearCalls();
    core.emit({ type: "runs_changed" });
    await settle();
    expect(core.calls).toEqual([]);
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
