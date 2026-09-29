// The expanded widget's header, transport and tab bar.
import { beforeEach, describe, expect, test, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import Header from "./Header.svelte";
import Transport from "./Transport.svelte";
import SidePanel from "./SidePanel.svelte";
import { core, freshStore, settle } from "../../test/app";
import { browserBackend } from "../../lib/ipc/backend";
import type { RelayStore } from "../../lib/state/relay.svelte";

let relay: RelayStore;
const A = "00000000-0000-0000-0000-000000000001";
beforeEach(async () => {
  relay = await freshStore();
});

describe("Header", () => {
  test("shows the macro's name and size", () => {
    render(Header);
    expect(screen.getByRole("textbox", { name: "Macro name" })).toHaveValue("Export invoice to PDF");
    expect(screen.getByText("12 steps · 301 path samples")).toBeInTheDocument();
  });

  test("typing renames the macro, saved after a pause", async () => {
    vi.useFakeTimers();
    render(Header);
    const name = screen.getByRole("textbox", { name: "Macro name" });
    await fireEvent.input(name, { target: { value: "Invoice → PDF" } });
    expect(relay.name).toBe("Invoice → PDF");
    await vi.advanceTimersByTimeAsync(300);
    await settle();
    expect(core.argsOf("edit_macro")).toEqual([{ id: A, op: { op: "rename", name: "Invoice → PDF" } }]);
  });

  test("while recording, the name is the new recording's, and locked; also with nothing open", async () => {
    render(Header);
    core.emit({ type: "session", mode: "recording", macro_id: null });
    await settle();
    expect(screen.getByRole("textbox", { name: "Macro name" })).toBeDisabled();
    expect(screen.getByRole("textbox", { name: "Macro name" })).toHaveValue("New recording");
    core.emit({ type: "session", mode: "idle", macro_id: null });
    relay.view = null;
    await settle();
    expect(screen.getByRole("textbox", { name: "Macro name" })).toBeDisabled();
  });

  test("the name is locked during playback too (Rust would refuse a rename)", async () => {
    render(Header);
    core.emit({ type: "session", mode: "paused", macro_id: A });
    await settle();
    expect(screen.getByRole("textbox", { name: "Macro name" })).toBeDisabled();
    expect(screen.getByRole("textbox", { name: "Macro name" })).toHaveValue("Export invoice to PDF");
  });

  test("clearing the name isn't saved, and leaving the field puts it back", async () => {
    vi.useFakeTimers();
    render(Header);
    const name = screen.getByRole("textbox", { name: "Macro name" });
    await fireEvent.input(name, { target: { value: " " } });
    expect(name).toHaveValue(" ");
    await vi.advanceTimersByTimeAsync(1000);
    await fireEvent.blur(name);
    expect(name).toHaveValue("Export invoice to PDF");
    expect(core.argsOf("edit_macro")).toEqual([]);
  });

  test("Ctrl + Z in the name field is the field's own undo", async () => {
    relay.start();
    await relay.edit({ op: "delete_step", index: 0 });
    render(Header);
    const name = screen.getByRole("textbox", { name: "Macro name" });
    name.focus();
    await fireEvent.keyDown(name, { key: "z", ctrlKey: true });
    await settle();
    expect(core.argsOf("undo_edit")).toEqual([]);
    relay.dispose();
  });

  test("Undo and Redo are enabled only when there's something to undo or redo", async () => {
    render(Header);
    const undo = screen.getByRole("button", { name: "Undo" });
    const redo = screen.getByRole("button", { name: "Redo" });
    expect(undo).toBeDisabled();
    expect(redo).toBeDisabled();
    await relay.edit({ op: "delete_step", index: 0 });
    await settle();
    expect(undo).toBeEnabled();
    core.clearCalls();
    await userEvent.click(undo);
    await settle();
    expect(core.argsOf("undo_edit")).toEqual([{ id: A, redo: false }]);
    expect(redo).toBeEnabled();
    expect(undo).toBeDisabled();
    await userEvent.click(redo);
    await settle();
    expect(core.argsOf("undo_edit")).toEqual([
      { id: A, redo: false },
      { id: A, redo: true },
    ]);
  });

  test("with no macro open, there's nothing to export", async () => {
    relay.view = null;
    render(Header);
    expect(screen.getByRole("button", { name: /Export/ })).toBeDisabled();
  });

  test("Export opens the export dialog", async () => {
    render(Header);
    await userEvent.click(screen.getByRole("button", { name: /Export/ }));
    expect(relay.exportOpen).toBe(true);
  });

  test("Compact player collapses the widget", async () => {
    render(Header);
    await userEvent.click(screen.getByRole("button", { name: "Compact player" }));
    expect(relay.expanded).toBe(false);
  });

  test("× hides to the tray, or says Quit when Close to tray is off", async () => {
    render(Header);
    await userEvent.click(screen.getByRole("button", { name: "Hide to tray" }));
    expect(core.commands()).toEqual(["hide_to_tray"]);
    await relay.updateSettings({ close_to_tray: false });
    await settle();
    expect(screen.getByRole("button", { name: "Quit" })).toHaveAttribute("title", "Quit");
  });

  test("the browser preview has no undo, redo or close", async () => {
    core.uninstall();
    try {
      await freshStore({ backend: browserBackend() });
      render(Header);
      expect(screen.queryByRole("button", { name: "Undo" })).toBeNull();
      expect(screen.queryByRole("button", { name: "Redo" })).toBeNull();
      expect(screen.queryByRole("button", { name: "Hide to tray" })).toBeNull();
      expect(screen.getByRole("button", { name: /Export/ })).toBeInTheDocument();
      expect(screen.getByRole("textbox", { name: "Macro name" })).toHaveAttribute("readonly");
    } finally {
      core.install();
    }
  });
});

describe("Transport", () => {
  test("shows the playhead and the length", async () => {
    render(Transport);
    expect(screen.getByText("00:00.00")).toBeInTheDocument();
    expect(screen.getByText("/ 00:10.15")).toBeInTheDocument();
    relay.seek(1234);
    await settle();
    expect(screen.getByText("00:01.23")).toBeInTheDocument();
  });

  test("says “/ recording” while recording", async () => {
    render(Transport);
    core.emit({ type: "session", mode: "recording", macro_id: null });
    await settle();
    expect(screen.getByText("/ recording")).toBeInTheDocument();
  });

  test("every zone has its label, on one baseline, at every width", () => {
    for (const mode of ["wide", "mid", "narrow"] as const) {
      const { container, unmount } = render(Transport, { mode });
      const labels = [...container.querySelectorAll(".label")].map((l) => l.firstChild!.textContent);
      expect(labels).toEqual(["Time", "Playback", "Speed", "Repeat"]);
      unmount();
    }
  });

  test("wide, it has the four speeds and − count +", () => {
    render(Transport, { mode: "wide" });
    expect(screen.getAllByRole("radio")).toHaveLength(4);
    expect(screen.getByRole("button", { name: "More repeats" })).toBeInTheDocument();
  });

  test("mid: the speed is one button that cycles", async () => {
    render(Transport, { mode: "mid" });
    expect(screen.queryByRole("radio")).toBeNull();
    const speed = screen.getByRole("button", { name: "Speed 1×" });
    expect(speed).toHaveAttribute("title", "Speed: click for 2×");
    await userEvent.click(speed);
    await settle();
    expect((core.lastArgs("set_playback_options")!.options as { speed: number }).speed).toBe(2);
    expect(screen.getByRole("button", { name: "Speed 2×" })).toBeInTheDocument();
    // − count + and Loop forever are still there.
    for (const name of ["Fewer repeats", "More repeats", "Loop forever"]) expect(screen.getByRole("button", { name })).toBeInTheDocument();
  });

  test("narrow: the repeat folds into one button that steps through 1, 2, 3, 5, 10 and forever", async () => {
    render(Transport, { mode: "narrow" });
    expect(screen.queryByRole("button", { name: "More repeats" })).toBeNull();
    const folded = () => screen.getByRole("button", { name: /^Repeat/ });
    expect(folded()).toHaveAccessibleName("Repeat 3 times");
    expect(folded()).toHaveTextContent("×3");
    await userEvent.click(folded());
    await settle();
    expect(repeat()).toEqual({ count: 5 });
    await userEvent.click(folded());
    await userEvent.click(folded());
    await settle();
    expect(repeat()).toBe("forever");
    expect(folded()).toHaveAccessibleName("Repeat forever");
    expect(folded()).toHaveTextContent("∞");
    await userEvent.click(folded());
    await settle();
    expect(repeat()).toEqual({ count: 1 });
    expect(folded()).toHaveAccessibleName("Repeat 1 time");
  });

  test("mid and narrow controls are off while recording too", async () => {
    render(Transport, { mode: "narrow" });
    core.emit({ type: "session", mode: "recording", macro_id: null });
    await settle();
    expect(screen.getByRole("button", { name: /^Speed/ })).toBeDisabled();
    expect(screen.getByRole("button", { name: /^Repeat/ })).toBeDisabled();
  });

  test("Previous and Next step move the playhead between steps", async () => {
    render(Transport);
    await userEvent.click(screen.getByRole("button", { name: "Next step" }));
    expect(relay.cur).toBe(850);
    await userEvent.click(screen.getByRole("button", { name: "Next step" }));
    expect(relay.cur).toBe(1750);
    await userEvent.click(screen.getByRole("button", { name: "Previous step" }));
    expect(relay.cur).toBe(850);
    await settle();
    expect(core.argsOf("seek").at(-1)).toEqual({ t: 850 });
  });

  test("Stop sends stop_session", async () => {
    render(Transport);
    await userEvent.click(screen.getByRole("button", { name: "Stop" }));
    expect(core.commands()).toEqual(["stop_session"]);
  });

  test("Record and Play are here too", async () => {
    render(Transport);
    await userEvent.click(screen.getByRole("button", { name: "Record" }));
    await userEvent.click(screen.getByRole("button", { name: "Play" }));
    expect(core.commands()).toEqual(["toggle_record", "toggle_play"]);
  });

  test.each([
    ["0.5×", 0.5],
    ["1×", 1],
    ["2×", 2],
    ["4×", 4],
  ])("speed %s is saved as %d", async (label, speed) => {
    render(Transport);
    await userEvent.click(screen.getByRole("radio", { name: label }));
    await settle();
    expect((core.lastArgs("set_playback_options")!.options as { speed: number }).speed).toBe(speed);
    expect(screen.getByRole("radio", { name: label })).toHaveAttribute("aria-checked", "true");
  });

  test("speed and repeat are off while recording", async () => {
    render(Transport);
    core.emit({ type: "session", mode: "recording", macro_id: null });
    await settle();
    for (const r of screen.getAllByRole("radio")) expect(r).toBeDisabled();
    for (const name of ["Fewer repeats", "More repeats", "Loop forever"]) expect(screen.getByRole("button", { name })).toBeDisabled();
  });

  const repeat = () => (core.lastArgs("set_playback_options")!.options as { repeat: unknown }).repeat;

  test("+ and − change the repeat count, from 1 to 99", async () => {
    render(Transport);
    expect(screen.getByText("3")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "More repeats" }));
    await settle();
    expect(repeat()).toEqual({ count: 4 });
    expect(screen.getByText("4")).toBeInTheDocument();
    for (let i = 0; i < 3; i++) await userEvent.click(screen.getByRole("button", { name: "Fewer repeats" }));
    await settle();
    expect(repeat()).toEqual({ count: 1 });
    expect(screen.getByRole("button", { name: "Fewer repeats" })).toBeDisabled();
    await relay.setPlayback({ repeat: { count: 99 } });
    await settle();
    expect(screen.getByRole("button", { name: "More repeats" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Fewer repeats" })).toBeEnabled();
  });

  test("Loop forever, and back to the count it had", async () => {
    render(Transport);
    const forever = screen.getByRole("button", { name: "Loop forever" });
    expect(forever).toHaveAttribute("aria-pressed", "false");
    await userEvent.click(forever);
    await settle();
    expect(repeat()).toBe("forever");
    expect(forever).toHaveAttribute("aria-pressed", "true");
    expect(screen.getByText("∞")).toBeInTheDocument();
    await userEvent.click(forever);
    await settle();
    expect(repeat()).toEqual({ count: 3 });
  });

  test("while looping forever, + is off and − goes back to the remembered count", async () => {
    render(Transport);
    await userEvent.click(screen.getByRole("button", { name: "Loop forever" }));
    await settle();
    expect(screen.getByRole("button", { name: "More repeats" })).toBeDisabled();
    await userEvent.click(screen.getByRole("button", { name: "Fewer repeats" }));
    await settle();
    expect(repeat()).toEqual({ count: 3 });
    expect(screen.getByText("3")).toBeInTheDocument();
  });

  test("at 1, − still leaves forever", async () => {
    await relay.setPlayback({ repeat: { count: 1 } });
    render(Transport);
    await userEvent.click(screen.getByRole("button", { name: "Loop forever" }));
    await settle();
    expect(screen.getByRole("button", { name: "Fewer repeats" })).toBeEnabled();
    await userEvent.click(screen.getByRole("button", { name: "Fewer repeats" }));
    await settle();
    expect(repeat()).toEqual({ count: 1 });
  });

  test("the remembered count is kept per macro, across the Transport being rebuilt", async () => {
    const { unmount } = render(Transport);
    await userEvent.click(screen.getByRole("button", { name: "Loop forever" })); // A had 3
    await settle();
    unmount();
    await relay.loadMacro("00000000-0000-0000-0000-000000000002");
    await relay.setPlayback({ repeat: { count: 7 } });
    await relay.toggleForever();
    await relay.loadMacro(A);
    render(Transport);
    await userEvent.click(screen.getByRole("button", { name: "Loop forever" }));
    await settle();
    expect(repeat()).toEqual({ count: 3 });
  });
});

describe("SidePanel", () => {
  test("four tabs, Steps first", () => {
    render(SidePanel);
    const tabs = screen.getAllByRole("tab");
    expect(tabs.map((t) => t.textContent?.trim())).toEqual(["Steps", "Library", "Triggers", "Settings"]);
    expect(screen.getByRole("tab", { name: "Steps" })).toHaveAttribute("aria-selected", "true");
    expect(screen.getByRole("tabpanel")).toHaveTextContent("12 steps");
  });

  test.each([
    ["Library", "New recordings are saved here automatically."],
    ["Triggers", "Run from anywhere"],
    ["Settings", "Global hotkeys"],
    ["Steps", "+ Wait"],
  ])("the %s tab shows its page", async (tab, text) => {
    render(SidePanel);
    await userEvent.click(screen.getByRole("tab", { name: tab }));
    expect(relay.tab).toBe(tab.toLowerCase());
    expect(screen.getByRole("tab", { name: tab })).toHaveAttribute("aria-selected", "true");
    expect(screen.getByRole("tabpanel")).toHaveTextContent(text);
  });

  test("toasts show under the tabs", async () => {
    render(SidePanel);
    relay.notify("Saved invoice.rly");
    await settle();
    expect(screen.getByRole("status")).toHaveTextContent("Saved invoice.rly");
  });
});
