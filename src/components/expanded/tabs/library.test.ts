// The Library tab.
import { beforeEach, describe, expect, test, vi } from "vitest";
import { fireEvent, render, screen, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import LibraryTab from "./LibraryTab.svelte";
import { core, freshStore, settle } from "../../../test/app";
import { browserBackend } from "../../../lib/ipc/backend";
import type { RelayStore } from "../../../lib/state/relay.svelte";
import type { Step } from "../../../lib/types";

let relay: RelayStore;
const [A, B, C] = [1, 2, 3].map((n) => `00000000-0000-0000-0000-00000000000${n}`);
beforeEach(async () => {
  relay = await freshStore();
});

const rows = () => screen.getAllByRole("button").filter((b) => b.classList.contains("item") || b.classList.contains("row"));

describe("Library tab", () => {
  test("lists every macro with its length, steps, runs, hotkey and last run", async () => {
    await relay.setTriggers({ hotkey: { enabled: true, combo: "Ctrl + Alt + 1" } });
    render(LibraryTab);
    const items = rows();
    expect(items).toHaveLength(4);
    expect(items[0]).toHaveTextContent("Export invoice to PDF");
    expect(items[0]).toHaveTextContent("10.2 s · 18 steps · 148 runs");
    expect(items[0]).toHaveTextContent("Ctrl + Alt + 1");
    expect(items[1]).toHaveTextContent("—"); // the samples' hotkeys are off
    expect(items[0]).toHaveClass("active");
  });

  test("turning a hotkey off takes it out of the list", async () => {
    await relay.setTriggers({ hotkey: { enabled: true, combo: "Ctrl + Alt + 1" } });
    render(LibraryTab);
    expect(rows()[0]).toHaveTextContent("Ctrl + Alt + 1");
    await relay.setTriggers({ hotkey: { enabled: false, combo: "Ctrl + Alt + 1" } });
    await settle();
    expect(rows()[0]).not.toHaveTextContent("Ctrl + Alt + 1");
    expect(rows()[0]).toHaveTextContent("—");
  });

  test("an empty library says how to make a macro", async () => {
    core.entries = [];
    await relay.refreshLibrary();
    render(LibraryTab);
    expect(screen.getByText("No macros yet — press Record (F9) to make one.")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Import/ })).toBeEnabled();
  });

  test("clicking a macro opens it", async () => {
    render(LibraryTab);
    await userEvent.click(screen.getByText("Fill weekly timesheet"));
    await settle();
    expect(core.argsOf("load_macro")).toEqual([{ id: B }]);
    expect(relay.view?.id).toBe(B);
    expect(relay.tab).toBe("steps");
  });

  test("Enter or Space on a row opens it", async () => {
    render(LibraryTab);
    rows()[2].focus();
    await userEvent.keyboard("{Enter}");
    await settle();
    rows()[1].focus();
    await userEvent.keyboard(" ");
    await settle();
    expect(core.argsOf("load_macro")).toEqual([{ id: C }, { id: B }]);
  });

  test("Duplicate copies that macro, without opening the row first", async () => {
    render(LibraryTab);
    await userEvent.click(screen.getByRole("button", { name: "Duplicate Batch rename photos" }));
    await settle();
    expect(core.commands()).toEqual(["duplicate_macro", "list_macros", "load_macro", "screenshot", "get_triggers", "get_data_file"]);
    expect(core.argsOf("duplicate_macro")).toEqual([{ id: C }]);
    expect(rows()).toHaveLength(5);
    expect(rows()[3]).toHaveTextContent("Batch rename photos (copy)"); // right after the original
    expect(rows()[3]).toHaveClass("active"); // and opened
    expect(relay.view?.name).toBe("Batch rename photos (copy)");
  });

  test("Delete moves that macro to the trash", async () => {
    render(LibraryTab);
    await userEvent.click(screen.getByRole("button", { name: "Delete Batch rename photos" }));
    await settle();
    expect(core.argsOf("delete_macro")).toEqual([{ id: C }]);
    expect(core.argsOf("load_macro")).toEqual([]);
    expect(rows()).toHaveLength(3);
    expect(screen.queryByText("Batch rename photos")).toBeNull();
  });

  test("Enter on Delete deletes instead of opening", async () => {
    render(LibraryTab);
    screen.getByRole("button", { name: "Delete Fill weekly timesheet" }).focus();
    await userEvent.keyboard("{Enter}");
    await settle();
    expect(core.argsOf("delete_macro")).toEqual([{ id: B }]);
    expect(core.argsOf("load_macro")).toEqual([]);
  });

  test("the open macro shows the name being typed", async () => {
    render(LibraryTab);
    relay.rename("Typing…");
    await settle();
    expect(rows()[0]).toHaveTextContent("Typing…");
  });

  test("Duplicate and Delete are hidden, and Import… is off, during a session", async () => {
    render(LibraryTab);
    core.emit({ type: "session", mode: "playing", macro_id: A });
    await settle();
    expect(screen.queryByRole("button", { name: /^Duplicate/ })).toBeNull();
    expect(screen.queryByRole("button", { name: /^Delete/ })).toBeNull();
    expect(screen.getByRole("button", { name: /Import/ })).toBeDisabled();
  });

  test("Import… asks for files and imports them", async () => {
    render(LibraryTab);
    core.dialog.open = ["C:\\macros\\Weekly report.rly"];
    await userEvent.click(screen.getByRole("button", { name: /Import/ }));
    await settle();
    expect(core.commands()).toEqual(["plugin:dialog|open", "import_macros", "list_macros", "load_macro", "screenshot", "get_triggers", "get_data_file"]);
    expect(rows()[0]).toHaveTextContent("Weekly report"); // at the top
    expect(rows()[0]).toHaveClass("active");
    expect(relay.toast).toMatchObject({ kind: "info", message: "Imported 1 macro" });
  });

  test("the browser preview can't change the library", async () => {
    core.uninstall();
    try {
      await freshStore({ backend: browserBackend() });
      render(LibraryTab);
      expect(screen.queryByRole("button", { name: /^Duplicate/ })).toBeNull();
      expect(screen.queryByRole("button", { name: /Import/ })).toBeNull();
    } finally {
      core.install();
    }
  });
});
