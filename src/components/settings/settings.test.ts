// The Settings page: its sections, every control, and what it saves.
import { beforeEach, describe, expect, test } from "vitest";
import { fireEvent, render, screen, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import SettingsPage from "./SettingsPage.svelte";
import { core, freshStore, settle } from "../../test/app";
import { DEFAULT_SETTINGS } from "../../lib/defaults";
import { browserBackend } from "../../lib/ipc/backend";
import type { RelayStore } from "../../lib/state/relay.svelte";

let relay: RelayStore;
beforeEach(async () => {
  relay = await freshStore();
  relay.place = "settings";
});

const toggle = (name: string) => screen.getByRole("switch", { name });
const group = (name: string) => within(screen.getByRole("radiogroup", { name }));
const saved = () => core.lastArgs("update_settings")?.settings as Record<string, unknown>;
const open = async (section: string) => {
  render(SettingsPage);
  if (section !== "Recording") await userEvent.click(screen.getByRole("button", { name: section }));
};

describe("Settings", () => {
  test("five sections, Recording first; one shows at a time", async () => {
    render(SettingsPage);
    const nav = within(screen.getByRole("navigation", { name: "Settings sections" }));
    expect(nav.getAllByRole("button").map((b) => b.textContent?.trim())).toEqual([
      "Recording",
      "Keys",
      "Window",
      "Appearance",
      "Sound",
    ]);
    expect(nav.getByRole("button", { name: "Recording" })).toHaveAttribute("aria-current", "page");
    expect(screen.getByRole("region", { name: "Recording" })).toBeInTheDocument();
    await userEvent.click(nav.getByRole("button", { name: "Sound" }));
    expect(nav.getByRole("button", { name: "Sound" })).toHaveAttribute("aria-current", "page");
    expect(nav.getByRole("button", { name: "Recording" })).not.toHaveAttribute("aria-current");
    expect(screen.queryByRole("switch", { name: "Capture keystrokes" })).toBeNull();
  });

  test("Back goes to the Library", async () => {
    render(SettingsPage);
    await userEvent.click(screen.getByRole("button", { name: "Back to the Library" }));
    expect(relay.place).toBe("library");
  });

  test.each([
    ["Recording", "Capture the mouse path", "capture_moves", true],
    ["Recording", "Capture keystrokes", "capture_keys", true],
    ["Recording", "Screenshot", "capture_screen", true],
    ["Recording", "Count down 3, 2, 1", "countdown", true],
    ["Recording", "Esc stops recording", "esc_stops_recording", true],
    ["Recording", "Ignore simulated input", "ignore_injected", true],
    ["Window", "Hide the window while recording", "hide_while_recording", true],
    ["Window", "Hide the window while playing", "hide_while_playing", false],
    ["Window", "Close to the tray", "close_to_tray", true],
    ["Appearance", "Show click labels", "show_click_labels", true],
    ["Sound", "Sounds", "sounds", false],
  ])("%s: the %s switch saves %s", async (section, name, field, initial) => {
    await open(section);
    expect(toggle(name)).toHaveAttribute("aria-checked", String(initial));
    await userEvent.click(toggle(name));
    await settle();
    expect(saved()).toEqual({ ...DEFAULT_SETTINGS, [field]: !initial });
    expect(toggle(name)).toHaveAttribute("aria-checked", String(!initial));
  });

  test.each([
    ["Window", "Keep the player on top", "While working", "keep_on_top", "sessions"],
    ["Window", "Keep the player on top", "Never", "keep_on_top", "never"],
    ["Appearance", "Theme", "Dark", "theme", "dark"],
    ["Appearance", "Theme", "Light", "theme", "light"],
    ["Appearance", "Spacing", "Compact", "spacing", "compact"],
    ["Appearance", "Spacing", "Large", "spacing", "large"],
    ["Appearance", "Text size", "150%", "text_size", 150],
    ["Appearance", "Text size", "200%", "text_size", 200],
    ["Appearance", "Player size", "Large", "player_size", "large"],
    ["Appearance", "Reduce motion", "Always", "reduce_motion", "always"],
    ["Appearance", "Mouse path", "Played part", "path_mode", "trail"],
  ])("%s: %s → %s saves %s", async (section, name, label, field, value) => {
    await open(section);
    await userEvent.click(group(name).getByRole("radio", { name: label }));
    await settle();
    expect(saved()).toEqual({ ...DEFAULT_SETTINGS, [field]: value });
    expect(group(name).getByRole("radio", { name: label })).toHaveAttribute("aria-checked", "true");
  });

  test("the defaults are chosen", async () => {
    await open("Appearance");
    for (const [name, label] of [
      ["Theme", "Like Windows"],
      ["Spacing", "Comfortable"],
      ["Text size", "100%"],
      ["Player size", "Compact"],
      ["Reduce motion", "Like Windows"],
      ["Mouse path", "Whole path"],
    ]) {
      expect(group(name).getByRole("radio", { name: label })).toHaveAttribute("aria-checked", "true");
    }
  });

  describe("Frosted glass", () => {
    test("on by default on Windows 11; turning it off saves it off", async () => {
      await open("Appearance");
      expect(toggle("Frosted glass")).toHaveAttribute("aria-checked", "true");
      expect(screen.getByText("On by default on Windows 11.")).toBeInTheDocument();
      await userEvent.click(toggle("Frosted glass"));
      await settle();
      expect(saved()).toEqual({ ...DEFAULT_SETTINGS, glass: false });
    });

    test("off by default on Windows 10, and it can be turned on", async () => {
      relay.systemLook = { glass_default: false, transparency: true };
      await open("Appearance");
      expect(toggle("Frosted glass")).toHaveAttribute("aria-checked", "false");
      expect(screen.getByText(/Off by default on Windows 10/)).toBeInTheDocument();
      await userEvent.click(toggle("Frosted glass"));
      await settle();
      expect(saved()).toEqual({ ...DEFAULT_SETTINGS, glass: true });
    });

    test("solid, and not settable, while Windows' transparency effects are off", async () => {
      await relay.updateSettings({ glass: true });
      relay.systemLook = { glass_default: true, transparency: false };
      await open("Appearance");
      expect(toggle("Frosted glass")).toHaveAttribute("aria-checked", "false");
      expect(toggle("Frosted glass")).toBeDisabled();
      expect(screen.getByText("Windows' transparency effects are off, so Relay is solid.")).toBeInTheDocument();
    });
  });

  test("Start with Windows", async () => {
    await open("Window");
    expect(toggle("Start with Windows")).toHaveAttribute("aria-checked", "false");
    await userEvent.click(toggle("Start with Windows"));
    await settle();
    expect(core.argsOf("set_autostart")).toEqual([{ enabled: true }]);
    expect(toggle("Start with Windows")).toHaveAttribute("aria-checked", "true");
  });

  test("a setting that fails to save flips back and says why", async () => {
    await open("Recording");
    core.hold("update_settings");
    await userEvent.click(toggle("Count down 3, 2, 1"));
    await settle();
    expect(toggle("Count down 3, 2, 1")).toHaveAttribute("aria-checked", "false"); // shown at once
    core.held[0].reject({ code: "io", message: "Couldn't save settings: access denied" });
    await settle();
    expect(toggle("Count down 3, 2, 1")).toHaveAttribute("aria-checked", "true");
    expect(screen.getByRole("alert")).toHaveTextContent("Couldn't save settings: access denied");
  });

  test("warns that recordings store passwords", () => {
    render(SettingsPage);
    expect(screen.getByText(/passwords included/)).toBeInTheDocument();
  });

  test("Keys lists Relay's keys, as set", async () => {
    await open("Keys");
    for (const [what, keys] of [
      ["Record", "Ctrl + Shift + F9"],
      ["Play / pause", "Ctrl + Shift + F10"],
      ["Stop", "Esc"],
      ["Show the player", "Ctrl + Shift + M"],
      ["Stop everything", "Ctrl + Alt + Backspace"],
    ]) {
      expect(screen.getByText(what)).toBeInTheDocument();
      expect(screen.getByRole("img", { name: keys })).toBeInTheDocument();
    }
  });

  describe("setting a key", () => {
    const row = (name: string) => screen.getByText(name).closest<HTMLElement>(".setting")!;
    const change = (name: string) => userEvent.click(within(row(name)).getByRole("button", { name: "Change" }));
    const press = (key: string, init: KeyboardEventInit = {}) => fireEvent.keyDown(window, { key, ...init });

    test("Change, then a combination: saved, said, and the hotkeys let go of the keys meanwhile", async () => {
      await open("Keys");
      await change("Record");
      expect(core.hotkeysHeld).toBe(true);
      expect(within(row("Record")).getByText("Press keys…")).toBeInTheDocument();
      press("F8", { code: "F8" });
      await settle();
      expect(core.lastArgs("set_relay_key")).toEqual({ key: "record", combo: "F8" });
      expect(core.hotkeysHeld).toBe(false);
      expect(within(row("Record")).getByRole("img", { name: "F8" })).toBeInTheDocument();
      expect(within(row("Record")).getByRole("status")).toHaveTextContent("Set to F8.");
    });

    test("a key apps use, or that types a character here, is set with a warning", async () => {
      await open("Keys");
      await change("Record");
      press("F9", { code: "F9" });
      await settle();
      expect(relay.settings.keys.record).toBe("F9");
      expect(within(row("Record")).getByRole("status")).toHaveTextContent("F9 recalculates in Excel");
      core.altgr = { "Ctrl + Alt + 7": "{" };
      await change("Show the player");
      press("7", { code: "Digit7", ctrlKey: true, altKey: true });
      await settle();
      expect(within(row("Show the player")).getByRole("status")).toHaveTextContent(
        "Ctrl + Alt + 7 types “{” on your keyboard (AltGr + 7). You couldn't type it while this is set.",
      );
    });

    test("one that can't be is refused, saying why, and nothing changes", async () => {
      await open("Keys");
      await change("Play / pause");
      press("F9", { code: "F9", ctrlKey: true, shiftKey: true });
      await settle();
      expect(within(row("Play / pause")).getByRole("status")).toHaveTextContent(
        "Ctrl + Shift + F9 is already Relay's “Record” key",
      );
      expect(relay.settings.keys.play).toBe("Ctrl + Shift + F10");
      expect(screen.queryByRole("alert")).toBeNull(); // said in the row, not as an error message
    });

    test("Backspace sets None, except for Stop everything; Ctrl + Alt + Backspace is a combination", async () => {
      await open("Keys");
      await change("Record");
      press("Backspace", { code: "Backspace" });
      await settle();
      expect(relay.settings.keys.record).toBeNull();
      expect(within(row("Record")).getByText("None")).toBeInTheDocument();
      expect(within(row("Record")).getByRole("status")).toHaveTextContent("Set to None: use the buttons instead.");
      await change("Stop everything");
      press("Backspace", { code: "Backspace" });
      await settle();
      expect(within(row("Stop everything")).getByRole("status")).toHaveTextContent("can't be turned off");
      await change("Stop everything");
      press("End", { code: "End", ctrlKey: true, altKey: true });
      await settle();
      expect(relay.settings.keys.kill).toBe("Ctrl + Alt + End");
      expect(within(row("Stop everything")).getByRole("status")).toHaveTextContent("Remote Desktop sends this as Ctrl + Alt + Del");
      await change("Stop everything");
      press("Backspace", { code: "Backspace", ctrlKey: true, altKey: true });
      await settle();
      expect(relay.settings.keys.kill).toBe("Ctrl + Alt + Backspace");
    });

    test("Esc cancels, and so does Cancel", async () => {
      await open("Keys");
      await change("Record");
      press("Escape", { code: "Escape" });
      await settle();
      expect(within(row("Record")).queryByText("Press keys…")).toBeNull();
      await change("Record");
      await userEvent.click(within(row("Record")).getByRole("button", { name: "Cancel" }));
      expect(core.argsOf("set_relay_key")).toEqual([]);
      expect(core.hotkeysHeld).toBe(false);
    });

    test("the rest of the UI names the keys as set", async () => {
      await relay.setRelayKey("record", "F8");
      await open("Recording");
      expect(screen.getByText("Turn off to record Esc; stop with the player's ■ or F8.")).toBeInTheDocument();
    });
  });

  test("the browser preview has no Window section", async () => {
    core.uninstall();
    try {
      await freshStore({ backend: browserBackend() });
      render(SettingsPage);
      expect(screen.queryByRole("button", { name: "Window" })).toBeNull();
      expect(screen.getByRole("button", { name: "Appearance" })).toBeInTheDocument();
    } finally {
      core.install();
    }
  });
});
