// The Settings and Triggers tabs: every control, and what it saves.
import { beforeEach, describe, expect, test, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import SettingsTab from "./SettingsTab.svelte";
import TriggersTab from "./TriggersTab.svelte";
import { core, freshStore, settle } from "../../../test/app";
import { browserBackend } from "../../../lib/ipc/backend";
import type { RelayStore } from "../../../lib/state/relay.svelte";
import type { MacroTriggers } from "../../../lib/types";

let relay: RelayStore;
const A = "00000000-0000-0000-0000-000000000001";
beforeEach(async () => {
  relay = await freshStore();
});

const toggle = (name: string) => screen.getByRole("switch", { name });
const change = (el: HTMLElement, value: string) => fireEvent.change(el, { target: { value } });

describe("Settings tab", () => {
  const options = () => core.lastArgs("set_playback_options")?.options as Record<string, unknown>;
  const settings = () => core.lastArgs("update_settings")?.settings as Record<string, unknown>;

  test.each([
    ["Humanize", "humanize", true],
    ["Stop on key press", "stop_on_key", true],
  ])("the %s switch saves the macro's %s", async (name, field, initial) => {
    render(SettingsTab);
    expect(toggle(name)).toHaveAttribute("aria-checked", String(initial));
    await userEvent.click(toggle(name));
    await settle();
    expect(core.lastArgs("set_playback_options")?.id).toBe(A);
    expect(options()[field]).toBe(!initial);
    expect(toggle(name)).toHaveAttribute("aria-checked", String(!initial));
  });

  test("the jitter slider previews while dragging and saves on release", async () => {
    render(SettingsTab);
    const slider = screen.getByRole("slider", { name: "Jitter" });
    expect(slider).toHaveValue("40");
    await fireEvent.input(slider, { target: { value: "120" } });
    expect(screen.getByText("Randomize delays ±120 ms")).toBeInTheDocument();
    expect(core.argsOf("set_playback_options")).toEqual([]);
    await fireEvent.change(slider, { target: { value: "120" } });
    await settle();
    expect(options().jitter_ms).toBe(120);
  });

  test("Coordinates: Screen or Window", async () => {
    render(SettingsTab);
    expect(screen.getByRole("radio", { name: "Screen" })).toHaveAttribute("aria-checked", "true");
    await userEvent.click(screen.getByRole("radio", { name: "Window" }));
    await settle();
    expect(options().coord_mode).toBe("window");
    await userEvent.click(screen.getByRole("radio", { name: "Screen" }));
    await settle();
    expect(options().coord_mode).toBe("screen");
  });

  test.each([
    ["Capture mouse path", "capture_moves"],
    ["Capture keystrokes", "capture_keys"],
    ["3-second countdown", "countdown"],
    ["Esc stops recording", "esc_stops_recording"],
    ["Ignore simulated input", "ignore_injected"],
    ["Click labels", "show_click_labels"],
    ["Close to tray", "close_to_tray"],
  ])("the %s switch saves %s", async (name, field) => {
    render(SettingsTab);
    expect(toggle(name)).toHaveAttribute("aria-checked", "true");
    await userEvent.click(toggle(name));
    await settle();
    expect(settings()).toEqual({ ...core.settings, [field]: false });
    expect(toggle(name)).toHaveAttribute("aria-checked", "false");
    await userEvent.click(toggle(name));
    await settle();
    expect(settings()[field]).toBe(true);
  });

  test("Mouse path: Full path or Trail only", async () => {
    render(SettingsTab);
    await userEvent.click(screen.getByRole("radio", { name: "Trail only" }));
    await settle();
    expect(settings().path_mode).toBe("trail");
    await userEvent.click(screen.getByRole("radio", { name: "Full path" }));
    await settle();
    expect(settings().path_mode).toBe("full");
  });

  test.each([
    ["Always", "always"],
    ["While recording or playing", "sessions"],
    ["Never", "never"],
  ])("Keep on top: %s", async (label, value) => {
    await relay.updateSettings({ keep_on_top: value === "always" ? "never" : "always" });
    render(SettingsTab);
    await userEvent.click(screen.getByRole("radio", { name: label }));
    await settle();
    expect(settings().keep_on_top).toBe(value);
    expect(screen.getByRole("radio", { name: label })).toHaveAttribute("aria-checked", "true");
  });

  test("Start with Windows", async () => {
    render(SettingsTab);
    expect(toggle("Start with Windows")).toHaveAttribute("aria-checked", "false");
    await userEvent.click(toggle("Start with Windows"));
    await settle();
    expect(core.argsOf("set_autostart")).toEqual([{ enabled: true }]);
    expect(toggle("Start with Windows")).toHaveAttribute("aria-checked", "true");
  });

  test("a setting that fails to save flips back and says why", async () => {
    render(SettingsTab);
    core.fail("update_settings", "Couldn't save settings: access denied");
    await userEvent.click(toggle("3-second countdown"));
    await settle();
    expect(toggle("3-second countdown")).toHaveAttribute("aria-checked", "true");
    expect(relay.error).toBe("Couldn't save settings: access denied");
  });

  test("lists the global hotkeys", () => {
    render(SettingsTab);
    for (const [what, key] of [
      ["Start / stop recording", "F9"],
      ["Play / pause", "F10"],
      ["Stop everything", "Esc"],
      ["Toggle compact player", "Ctrl + Shift + M"],
      ["Emergency kill switch", "Ctrl + Alt + End"],
    ]) {
      expect(screen.getByText(what)).toBeInTheDocument();
      expect(screen.getByText(key)).toBeInTheDocument();
    }
  });

  test("warns that recordings store passwords", () => {
    render(SettingsTab);
    expect(screen.getByText(/passwords included/)).toBeInTheDocument();
  });

  test("the browser preview has no window settings", async () => {
    core.uninstall();
    try {
      await freshStore({ backend: browserBackend() });
      render(SettingsTab);
      expect(screen.queryByRole("radiogroup", { name: "Keep on top" })).toBeNull();
      expect(screen.queryByRole("switch", { name: "Close to tray" })).toBeNull();
      expect(screen.queryByRole("switch", { name: "Start with Windows" })).toBeNull();
      expect(screen.getByRole("switch", { name: "Humanize" })).toBeInTheDocument();
    } finally {
      core.install();
    }
  });
});

describe("Triggers tab", () => {
  const sent = () => core.lastArgs("set_triggers")?.triggers as MacroTriggers;

  test("loads the running programs for suggestions", async () => {
    const { container } = render(TriggersTab);
    await settle();
    expect(core.commands()).toEqual(["list_processes"]);
    const options = [...container.querySelectorAll("#relay-processes option")].map((o) => o.getAttribute("value"));
    expect(options).toEqual(["chrome.exe", "EXCEL.EXE", "notepad.exe"]);
  });

  test("shows nothing until the triggers have loaded", () => {
    relay.triggerStatus = null;
    render(TriggersTab);
    expect(screen.queryByRole("switch")).toBeNull();
  });

  describe("hotkey", () => {
    test("shows the combo and whether it's on", () => {
      render(TriggersTab);
      expect(screen.getByRole("button", { name: /Ctrl \+ Alt \+ 1/ })).toBeInTheDocument();
      expect(toggle("Hotkey trigger")).toHaveAttribute("aria-checked", "true");
      expect(screen.getByText("Run from anywhere")).toBeInTheDocument();
    });

    test("setting a combo turns it on", async () => {
      render(TriggersTab);
      await userEvent.click(screen.getByRole("button", { name: /Ctrl \+ Alt \+ 1/ }));
      await fireEvent.keyDown(window, { key: "7", code: "Digit7", ctrlKey: true, altKey: true });
      await settle();
      expect(core.argsOf("set_triggers")).toEqual([{ id: A, triggers: { ...core.triggers.get(A) } }]);
      expect(sent().hotkey).toEqual({ enabled: true, combo: "Ctrl + Alt + 7" });
    });

    test("Backspace clears it and turns it off", async () => {
      render(TriggersTab);
      await userEvent.click(screen.getByRole("button", { name: /Ctrl \+ Alt \+ 1/ }));
      await fireEvent.keyDown(window, { key: "Backspace", code: "Backspace" });
      await settle();
      expect(sent().hotkey).toEqual({ enabled: false, combo: "" });
      expect(screen.getByText("Set…")).toBeInTheDocument();
    });

    test("the switch turns it off and on; without a combo it stays off", async () => {
      render(TriggersTab);
      await userEvent.click(toggle("Hotkey trigger"));
      await settle();
      expect(sent().hotkey).toEqual({ enabled: false, combo: "Ctrl + Alt + 1" });
      await userEvent.click(toggle("Hotkey trigger"));
      await settle();
      expect(sent().hotkey).toEqual({ enabled: true, combo: "Ctrl + Alt + 1" });
      await relay.setTriggers({ hotkey: { enabled: false, combo: "" } });
      await settle();
      await userEvent.click(toggle("Hotkey trigger"));
      await settle();
      expect(sent().hotkey).toEqual({ enabled: false, combo: "" });
    });

    test("a combo another app owns is explained", async () => {
      core.hotkeyErrors.set(A, "Ctrl + Alt + 1 is taken by another app");
      await relay.loadMacro(A);
      render(TriggersTab);
      const why = screen.getByText("Ctrl + Alt + 1 is taken by another app");
      expect(why).toHaveClass("warn");
    });

    test("a refused combo is put back", async () => {
      render(TriggersTab);
      core.fail("set_triggers", "F9 is one of Relay's own hotkeys", "hotkey");
      await userEvent.click(screen.getByRole("button", { name: /Ctrl \+ Alt \+ 1/ }));
      await fireEvent.keyDown(window, { key: "F9", code: "F9" });
      await settle();
      expect(screen.getByRole("button", { name: /Ctrl \+ Alt \+ 1/ })).toBeInTheDocument();
      expect(relay.error).toBe("F9 is one of Relay's own hotkeys");
    });
  });

  describe("schedule", () => {
    test("the switch, and the next run", async () => {
      render(TriggersTab);
      expect(screen.getByText("No schedule")).toBeInTheDocument();
      await userEvent.click(toggle("Schedule trigger"));
      await settle();
      expect(sent().schedule.enabled).toBe(true);
      expect(screen.getByText(/^Next run: /)).toBeInTheDocument();
    });

    test("each day button toggles that day", async () => {
      render(TriggersTab);
      const days = screen.getByRole("group", { name: "Days" }).querySelectorAll("button");
      expect([...days].map((d) => d.getAttribute("aria-pressed"))).toEqual(["true", "true", "true", "true", "true", "false", "false"]);
      await userEvent.click(days[5]); // Saturday on
      await settle();
      expect(sent().schedule.schedule.days).toEqual([true, true, true, true, true, true, false]);
      await userEvent.click(days[0]); // Monday off
      await settle();
      expect(sent().schedule.schedule.days).toEqual([false, true, true, true, true, true, false]);
      expect(days[0]).toHaveAttribute("aria-pressed", "false");
    });

    test("the time", async () => {
      render(TriggersTab);
      const time = screen.getByLabelText("Time");
      expect(time).toHaveValue("09:00");
      await change(time, "17:45");
      await settle();
      expect(sent().schedule.schedule.time).toBe("17:45");
      core.clearCalls();
      await change(time, ""); // cleared: ignored
      await settle();
      expect(core.argsOf("set_triggers")).toEqual([]);
    });
  });

  describe("app launch", () => {
    test("the program name is trimmed; the switch needs one", async () => {
      render(TriggersTab);
      await userEvent.click(toggle("App launch trigger"));
      await settle();
      expect(sent().app_launch).toEqual({ enabled: false, exe: "", delay_ms: 2000 });
      await change(screen.getByLabelText("Program"), "  EXCEL.EXE ");
      await settle();
      expect(sent().app_launch.exe).toBe("EXCEL.EXE");
      await userEvent.click(toggle("App launch trigger"));
      await settle();
      expect(sent().app_launch).toEqual({ enabled: true, exe: "EXCEL.EXE", delay_ms: 2000 });
    });

    test("clearing the program turns it off", async () => {
      await relay.setTriggers({ app_launch: { enabled: true, exe: "notepad.exe", delay_ms: 0 } });
      render(TriggersTab);
      await change(screen.getByLabelText("Program"), "   ");
      await settle();
      expect(sent().app_launch).toEqual({ enabled: false, exe: "", delay_ms: 0 });
    });

    test("the delay, in seconds", async () => {
      render(TriggersTab);
      expect(screen.getByText("Runs 2 s after it starts")).toBeInTheDocument();
      const delay = screen.getByLabelText("Delay in seconds");
      await change(delay, "3.5");
      await settle();
      expect(sent().app_launch.delay_ms).toBe(3500);
      await change(delay, "-2");
      await settle();
      expect(sent().app_launch.delay_ms).toBe(0);
      core.clearCalls();
      await change(delay, "");
      await settle();
      expect(core.argsOf("set_triggers")).toEqual([]);
      expect(delay).toHaveValue(0); // put back
    });
  });

  describe("pixel", () => {
    test("the switch, position and color", async () => {
      render(TriggersTab);
      await userEvent.click(toggle("Pixel trigger"));
      await settle();
      expect(sent().pixel.enabled).toBe(true);
      await change(screen.getByLabelText("X"), "-100");
      await settle();
      await change(screen.getByLabelText("Y"), "250");
      await settle();
      await change(screen.getByLabelText("Color"), "#a1b2c3");
      await settle();
      expect(sent().pixel).toEqual({ enabled: true, x: -100, y: 250, color: "#A1B2C3", tolerance: 8 });
      expect(screen.getByText("-100, 250 becomes #A1B2C3")).toBeInTheDocument();
    });

    test("bad input is put back and not sent", async () => {
      render(TriggersTab);
      const color = screen.getByLabelText("Color");
      await change(color, "blue");
      expect(color).toHaveValue("#EC3013");
      const x = screen.getByLabelText("X");
      await change(x, "");
      expect(x).toHaveValue(0);
      await settle();
      expect(core.argsOf("set_triggers")).toEqual([]);
    });

    test("Pick watches the pixel under the cursor after a countdown", async () => {
      render(TriggersTab);
      vi.useFakeTimers();
      core.hold("pick_pixel");
      await fireEvent.click(screen.getByRole("button", { name: "Pick" }));
      await settle();
      expect(screen.getByRole("button", { name: /Point…/ })).toHaveTextContent("Point… 3");
      await vi.advanceTimersByTimeAsync(2000);
      expect(screen.getByRole("button", { name: /Point…/ })).toHaveTextContent("Point… 1");
      core.held[0].resolve({ x: 11, y: 22, color: "#FFFFFF" });
      await settle();
      expect(sent().pixel).toMatchObject({ x: 11, y: 22, color: "#FFFFFF" });
      expect(screen.getByRole("button", { name: "Pick" })).toBeEnabled();
    });
  });

  test("when paused by the kill switch, Resume turns triggers back on", async () => {
    core.emit({ type: "triggers_paused", paused: true });
    render(TriggersTab);
    await settle();
    expect(screen.getByText("Triggers are paused (kill switch)")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Resume" }));
    await settle();
    expect(core.argsOf("set_triggers_paused")).toEqual([{ paused: false }]);
    expect(screen.queryByText("Triggers are paused (kill switch)")).toBeNull();
  });

  test("the browser preview can't pick pixels", async () => {
    core.uninstall();
    try {
      await freshStore({ backend: browserBackend() });
      render(TriggersTab);
      await settle();
      expect(screen.getByRole("button", { name: "Pick" })).toBeDisabled();
    } finally {
      core.install();
    }
  });
});
