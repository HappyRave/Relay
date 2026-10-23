// The window and its places: the Library, a macro, Settings, and the window's own controls.
import { beforeEach, describe, expect, test } from "vitest";
import { render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import AppWindow from "./AppWindow.svelte";
import { core, freshStore, settle } from "../../test/app";
import type { RelayStore } from "../../lib/state/relay.svelte";

let relay: RelayStore;
const B = "00000000-0000-0000-0000-000000000002";
beforeEach(async () => {
  relay = await freshStore();
});

describe("the window", () => {
  test("opens on the Library, with Settings at the top", async () => {
    const { container } = render(AppWindow);
    expect(container.querySelector(".app-window")).toHaveAttribute("data-place", "library");
    expect(screen.getByText("Fill weekly timesheet")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Settings" }));
    expect(container.querySelector(".app-window")).toHaveAttribute("data-place", "settings");
    expect(screen.getByRole("heading", { name: "Settings" })).toBeInTheDocument();
  });

  test("a macro opens in the editor, and Back comes home", async () => {
    render(AppWindow);
    await userEvent.click(screen.getByText("Fill weekly timesheet"));
    await settle();
    expect(core.lastArgs("load_macro")).toEqual({ id: B });
    expect(screen.getByRole("textbox", { name: "Macro name" })).toHaveValue("Fill weekly timesheet");
    await userEvent.click(screen.getByRole("button", { name: "Back to the Library" }));
    expect(relay.place).toBe("library");
  });

  test("Enter on a macro opens it too", async () => {
    render(AppWindow);
    screen.getByText("Fill weekly timesheet").closest<HTMLElement>("button")!.focus();
    await userEvent.keyboard("{Enter}");
    await settle();
    expect(relay.place).toBe("macro");
    expect(relay.view?.id).toBe(B);
  });

  test("the top bar moves the window", () => {
    const { container } = render(AppWindow);
    expect(container.querySelector(".topbar")).toHaveAttribute("data-tauri-drag-region");
  });

  test("Minimize, and close to the tray", async () => {
    render(AppWindow);
    core.clearCalls();
    await userEvent.click(screen.getByRole("button", { name: "Minimize" }));
    await userEvent.click(screen.getByRole("button", { name: "Hide to the tray" }));
    await settle();
    expect(core.commands()).toEqual(["plugin:window|minimize", "hide_to_tray"]);
  });

  test("with Close to the tray off, closing quits", async () => {
    await relay.updateSettings({ close_to_tray: false });
    render(AppWindow);
    expect(screen.getByRole("button", { name: "Quit" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Hide to the tray" })).toBeNull();
  });

  test("toasts show in the Library", async () => {
    render(AppWindow);
    relay.notify("Imported 2 macros");
    await settle();
    expect(screen.getByRole("status")).toHaveTextContent("Imported 2 macros");
  });
});
