// Controls shared by the compact player and the expanded widget.
import { beforeEach, describe, expect, test } from "vitest";
import { fireEvent, render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import RecPlayButtons from "./RecPlayButtons.svelte";
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

describe("Record and Play", () => {
  test("Record sends toggle_record; Play sends toggle_play", async () => {
    render(RecPlayButtons, { variant: "square" });
    await userEvent.click(screen.getByRole("button", { name: "Record" }));
    await userEvent.click(screen.getByRole("button", { name: "Play" }));
    expect(core.calls).toEqual([
      { cmd: "toggle_record", args: {} },
      { cmd: "toggle_play", args: { from: 0 } },
    ]);
  });

  test("while recording, Record becomes Stop recording", async () => {
    render(RecPlayButtons, { variant: "bar" });
    core.emit({ type: "session", mode: "recording", macro_id: null });
    await settle();
    const stop = screen.getByRole("button", { name: "Stop recording" });
    expect(stop).toHaveAttribute("title", "Stop recording (F9)");
    await userEvent.click(stop);
    expect(core.commands()).toEqual(["toggle_record"]);
  });

  test("while playing, Play becomes Pause", async () => {
    render(RecPlayButtons, { variant: "bar" });
    core.emit({ type: "session", mode: "playing", macro_id: core.ids[0] });
    await settle();
    await userEvent.click(screen.getByRole("button", { name: "Pause" }));
    expect(core.commands()).toEqual(["toggle_play"]);
    core.emit({ type: "session", mode: "paused", macro_id: core.ids[0] });
    await settle();
    expect(screen.getByRole("button", { name: "Play" })).toBeInTheDocument();
  });

  test("the hotkeys are in the tooltips", () => {
    render(RecPlayButtons, { variant: "square" });
    expect(screen.getByRole("button", { name: "Record" })).toHaveAttribute("title", "Record (F9)");
    expect(screen.getByRole("button", { name: "Play" })).toHaveAttribute("title", "Play / pause (F10)");
  });
});

describe("Toast", () => {
  test("nothing when there is no message", () => {
    const { container } = render(Toast);
    expect(container.querySelector(".toast")).toBeNull();
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
      expect(core.calls).toEqual([]);
    } finally {
      core.install();
    }
  });
});
