// The Settings page: its sections, every control, and what it saves.
import { beforeEach, describe, expect, test } from "vitest";
import { render, screen, within } from "@testing-library/svelte";
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

  test("Keys lists Relay's keys", async () => {
    await open("Keys");
    for (const [what, keys] of [
      ["Record", "F9"],
      ["Play / pause", "F10"],
      ["Stop", "Esc"],
      ["Show the player", "Ctrl + Shift + M"],
      ["Stop everything", "Ctrl + Alt + End"],
    ]) {
      expect(screen.getByText(what)).toBeInTheDocument();
      expect(screen.getByRole("img", { name: keys })).toBeInTheDocument();
    }
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
