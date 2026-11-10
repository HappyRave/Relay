// Pieces shared by the window and the player: the messages, and the grip.
import { beforeEach, describe, expect, test } from "vitest";
import { fireEvent, render, screen, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import Toast from "./Toast.svelte";
import Grip from "./Grip.svelte";
import { core, freshStore, settle } from "../../test/app";
import { devDesktop } from "../../lib/dev/devDesktop.svelte";
import { browserBackend } from "../../lib/ipc/backend";
import type { RelayStore } from "../../lib/state/relay.svelte";

let relay: RelayStore;
beforeEach(async () => {
  relay = await freshStore();
});

describe("Toast", () => {
  test("nothing when there is no message", () => {
    const { container } = render(Toast);
    expect(container.querySelector(".toast")).toBeNull();
  });

  test("a run a trigger started says so first, with Stop; one from Play doesn't", async () => {
    render(Toast);
    const D = "00000000-0000-0000-0000-000000000002";
    core.emit({ type: "session", mode: "playing", macro_id: D, source: "schedule" });
    core.emit({ type: "notice", message: "Triggers are paused" });
    await settle();
    const [run, notice] = screen.getAllByRole("status");
    expect(run).toHaveTextContent("Running “Fill weekly timesheet”, started by its schedule");
    expect(notice).toHaveTextContent("Triggers are paused");
    await userEvent.click(within(run).getByRole("button", { name: "Stop" }));
    expect(core.commands()).toEqual(["stop_session"]);
    core.emit({ type: "session", mode: "idle", macro_id: D, source: null });
    await settle();
    expect(screen.queryByText(/Running/)).toBeNull();
    core.emit({ type: "session", mode: "playing", macro_id: D, source: "manual" });
    await settle();
    expect(screen.queryByText(/Running/)).toBeNull();
  });

  test("errors are alerts", async () => {
    render(Toast);
    core.emit({ type: "error", message: "Couldn't save the change" });
    await settle();
    expect(screen.getByRole("alert")).toHaveTextContent("Couldn't save the change");
  });

  test("notices are status messages, and × dismisses", async () => {
    render(Toast);
    relay.notify("Imported 2 macros");
    await settle();
    expect(screen.getByRole("status")).toHaveTextContent("Imported 2 macros");
    await userEvent.click(screen.getByRole("button", { name: "Dismiss" }));
    expect(screen.queryByRole("status")).toBeNull();
  });

  test("several stack, newest at the bottom, the newest three shown, each dismissed on its own", async () => {
    const { container } = render(Toast);
    const shown = () => [...container.querySelectorAll(".toast .message")].map((m) => m.textContent);
    for (const message of ["Couldn't read library.json", "Couldn't register the Play hotkey", "Couldn't read settings.json"]) {
      core.emit({ type: "error", message });
    }
    relay.notify("Imported 2 macros");
    await settle();
    expect(shown()).toEqual(["Couldn't register the Play hotkey", "Couldn't read settings.json", "Imported 2 macros"]);
    await userEvent.click(screen.getAllByRole("button", { name: "Dismiss" })[1]);
    expect(shown()).toEqual(["Couldn't read library.json", "Couldn't register the Play hotkey", "Imported 2 macros"]);
    core.emit({ type: "error", message: "Couldn't read library.json" });
    await settle();
    expect(shown()).toEqual(["Couldn't register the Play hotkey", "Imported 2 macros", "Couldn't read library.json"]);
    expect(relay.toasts).toHaveLength(3); // the same message again moves, it doesn't show twice
  });

  test("the action button runs the action (Undo)", async () => {
    render(Toast);
    await relay.deleteMacro(core.ids[2]);
    await settle();
    core.clearCalls();
    await userEvent.click(screen.getByRole("button", { name: "Undo" }));
    await settle();
    expect(core.commands()[0]).toBe("restore_macro");
    expect(screen.queryByRole("status")).toBeNull();
  });
});

describe("Grip", () => {
  test("in the app, dragging it moves the native window", async () => {
    const { container } = render(Grip);
    await fireEvent.pointerDown(container.querySelector(".grip")!, { button: 0, clientX: 5, clientY: 5 });
    await settle();
    expect(core.calls).toEqual([{ cmd: "plugin:window|start_dragging", args: { label: "main" } }]);
  });

  test("only the left button drags", async () => {
    const { container } = render(Grip);
    await fireEvent.pointerDown(container.querySelector(".grip")!, { button: 2 });
    await settle();
    expect(core.calls).toEqual([]);
  });

  test("in the browser preview, it moves the widget on the demo desktop", async () => {
    core.uninstall();
    try {
      await freshStore({ backend: browserBackend() });
      devDesktop.dx = 0;
      devDesktop.dy = 0;
      const { container } = render(Grip);
      const grip = container.querySelector(".grip")!;
      await fireEvent.pointerDown(grip, { button: 0, clientX: 100, clientY: 100, pointerId: 1 });
      await fireEvent.pointerMove(grip, { clientX: 130, clientY: 90, pointerId: 1 });
      expect([devDesktop.dx, devDesktop.dy]).toEqual([30, -10]);
      await fireEvent.pointerUp(grip, { pointerId: 1 });
      await fireEvent.pointerMove(grip, { clientX: 500, clientY: 500, pointerId: 1 });
      expect([devDesktop.dx, devDesktop.dy]).toEqual([30, -10]);
    } finally {
      core.install();
    }
  });
});
