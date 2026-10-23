// The player: its keys, what its screen says in each state, the macro menu,
// errors, and the key back to the window.
import { beforeEach, describe, expect, test, vi } from "vitest";
import { render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import Player from "./Player.svelte";
import { core, freshStore, settle } from "../../test/app";
import type { RelayStore } from "../../lib/state/relay.svelte";

const [A, B] = [1, 2].map((n) => `00000000-0000-0000-0000-00000000000${n}`);
const button = (name: string) => screen.getByRole("button", { name });
const screenText = () => document.querySelector(".screen")!.textContent!.replace(/\s+/g, " ").trim();

let relay: RelayStore;
beforeEach(async () => {
  relay = await freshStore();
});

describe("the player's keys", () => {
  test("Record sends toggle_record, Play toggle_play from the playhead, Stop stop_session", async () => {
    render(Player);
    core.clearCalls();
    await userEvent.click(button("Record"));
    await userEvent.click(button("Play"));
    core.emit({ type: "session", mode: "playing", macro_id: A, source: null });
    await settle();
    await userEvent.click(button("Stop"));
    expect(core.calls).toEqual([
      { cmd: "toggle_record", args: {} },
      { cmd: "toggle_play", args: { from: 0 } },
      { cmd: "stop_session", args: {} },
    ]);
  });

  test("while recording, Record stops it, and Play is off", async () => {
    render(Player);
    core.emit({ type: "session", mode: "recording", macro_id: null, source: null });
    await settle();
    expect(button("Stop recording")).toHaveAttribute("title", "Stop recording (F9)");
    expect(button("Play")).toBeDisabled();
    expect(button("Stop")).toBeEnabled();
  });

  test("while playing, Play pauses and Record is off; Stop is off when idle", async () => {
    render(Player);
    expect(button("Stop")).toBeDisabled();
    core.emit({ type: "session", mode: "playing", macro_id: A, source: null });
    await settle();
    expect(button("Record")).toBeDisabled();
    await userEvent.click(button("Pause"));
    expect(core.commands()).toContain("toggle_play");
    core.emit({ type: "session", mode: "paused", macro_id: A, source: null });
    await settle();
    expect(button("Play")).toBeEnabled();
  });

  test("the keys are in the tooltips", () => {
    render(Player);
    expect(button("Record")).toHaveAttribute("title", "Record (F9)");
    expect(button("Play")).toHaveAttribute("title", "Play (F10)");
    expect(button("Stop")).toHaveAttribute("title", "Stop (Esc)");
    expect(button("Open the window")).toHaveAttribute("title", "Open the window (Ctrl + Shift + M)");
  });

  test("the expand key opens the window on the player's macro", async () => {
    render(Player);
    core.clearCalls();
    await userEvent.click(button("Open the window"));
    expect(core.commands()).toEqual(["show_window"]);
  });
});

describe("the player's screen", () => {
  test("ready: the macro, and its length", () => {
    render(Player);
    expect(screenText()).toBe("READY Export invoice to PDF 00:00 / 00:10");
  });

  test("the countdown, then the recording's steps and the last one", async () => {
    render(Player);
    core.emit({ type: "session", mode: "countdown", macro_id: null, source: null });
    core.emit({ type: "countdown", left_ms: 2400 });
    await settle();
    expect(screenText()).toBe("GET READY New recording 3");
    core.emit({ type: "session", mode: "recording", macro_id: null, source: null });
    const click = relay.view!.steps.find((s) => s.kind === "click")!;
    core.emit({ type: "rec_progress", elapsed_ms: 900, desktop: { x: 0, y: 0, w: 1920, h: 1080 }, moves: [], steps: [click] });
    await settle();
    expect(screenText()).toMatch(/^REC New recording 1 step Last: Click · /);
  });

  test("playing: the time, or the row of a data file", async () => {
    render(Player);
    core.emit({ type: "session", mode: "playing", macro_id: A, source: null });
    core.emit({ type: "play_tick", t: 4200, advancing: false, speed: 1, loop_idx: 0, loops: 1 });
    await settle();
    expect(screenText()).toBe("PLAYING Export invoice to PDF 00:04 / 00:10");
    relay.dataFile = { path: "C:\\data.csv", columns: ["Name"], rows: 200, error: null } as never;
    core.emit({ type: "play_tick", t: 1000, advancing: false, speed: 1, loop_idx: 36, loops: 200 });
    await settle();
    expect(screenText()).toBe("PLAYING Export invoice to PDF Row 37 / 200");
  });

  test("a completed run says DONE for a moment", async () => {
    vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
    try {
      render(Player);
      core.emit({ type: "session", mode: "playing", macro_id: A, source: null });
      core.emit({ type: "finished", reason: "completed", timing: null });
      core.emit({ type: "session", mode: "idle", macro_id: A, source: null });
      await settle();
      expect(screenText()).toMatch(/^DONE ✓/);
      vi.advanceTimersByTime(1600);
      await settle();
      expect(screenText()).toMatch(/^READY/);
    } finally {
      vi.useRealTimers();
    }
  });

  test("an error replaces the progress until it's dismissed", async () => {
    render(Player);
    core.emit({ type: "error", message: "Pixel check timed out at step 10" });
    await settle();
    expect(screenText()).toBe("STOPPED Export invoice to PDF 00:00 / 00:10 Pixel check timed out at step 10");
    await userEvent.click(button("Dismiss"));
    expect(screenText()).toBe("READY Export invoice to PDF 00:00 / 00:10");
  });

  test("the large size", async () => {
    const { container } = render(Player);
    expect(container.querySelector(".player")).not.toHaveClass("big");
    await relay.updateSettings({ player_size: "large" });
    await settle();
    expect(container.querySelector(".player")).toHaveClass("big");
  });
});

describe("the player's macro menu", () => {
  test("lists the macros, the current one checked, and switches", async () => {
    render(Player);
    await userEvent.click(button("Macro: Export invoice to PDF. Switch macros"));
    const items = screen.getAllByRole("menuitemcheckbox").map((i) => [i.textContent?.trim(), i.getAttribute("aria-checked")]);
    expect(items[0]).toEqual(["Export invoice to PDF", "true"]);
    expect(items.slice(1).every(([, c]) => c === "false")).toBe(true);
    core.clearCalls();
    await userEvent.click(screen.getByRole("menuitemcheckbox", { name: "Fill weekly timesheet" }));
    await settle();
    expect(core.argsOf("load_macro")).toEqual([{ id: B }]);
    expect(relay.view?.id).toBe(B);
  });

  test("is off during a session", async () => {
    render(Player);
    core.emit({ type: "session", mode: "playing", macro_id: A, source: null });
    await settle();
    expect(button("Macro: Export invoice to PDF. Switch macros")).toBeDisabled();
  });
});
