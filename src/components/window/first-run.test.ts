// The first run: three screens for a new install, the keys screen once after an upgrade.
import { beforeEach, describe, expect, test } from "vitest";
import { render, screen, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import AppWindow from "./AppWindow.svelte";
import { core, freshStore, settle } from "../../test/app";
import { browserBackend } from "../../lib/ipc/backend";
import type { RelayStore } from "../../lib/state/relay.svelte";

let relay: RelayStore;
const button = (name: string) => screen.getByRole("button", { name });
const LEGACY = { record: "F9", play: "F10", player: "Ctrl + Shift + M", kill: "Ctrl + Alt + End" };

describe("a new install", () => {
  beforeEach(async () => {
    relay = await freshStore();
  });

  test("opens on the first run, and Skip goes to the Library for good", async () => {
    const { container } = render(AppWindow);
    expect(container.querySelector(".app-window")).toHaveAttribute("data-place", "first-run");
    expect(screen.getByRole("heading", { name: /Do it once/ })).toBeInTheDocument();
    expect(screen.getByRole("img", { name: "Step 1 of 3" })).toBeInTheDocument();
    await userEvent.click(button("Skip"));
    await settle();
    expect(core.settings.intro).toBe("done");
    expect(container.querySelector(".app-window")).toHaveAttribute("data-place", "library");
  });

  test("the keys: Ctrl + Shift + F9 and F10 preselected, and kept as they are", async () => {
    render(AppWindow);
    await userEvent.click(button("Next"));
    expect(screen.getByRole("heading", { name: "Your keys" })).toBeInTheDocument();
    expect(screen.getByRole("radio", { name: /Ctrl \+ Shift \+ F9 and Ctrl \+ Shift \+ F10/ })).toHaveAttribute("aria-checked", "true");
    expect(screen.getByText(/It works over Remote Desktop too/)).toBeInTheDocument();
    core.clearCalls();
    await userEvent.click(button("Next"));
    await settle();
    expect(core.argsOf("set_relay_key")).toEqual([]); // already those
    expect(screen.getByRole("heading", { name: "Your first macro" })).toBeInTheDocument();
    expect(screen.getAllByRole("img", { name: "Ctrl + Shift + F9" })).toHaveLength(2);
  });

  test("choosing F9 and F10 sets them", async () => {
    render(AppWindow);
    await userEvent.click(button("Next"));
    await userEvent.click(screen.getByRole("radio", { name: /^F9 and F10/ }));
    core.clearCalls();
    await userEvent.click(button("Next"));
    await settle();
    expect(core.argsOf("set_relay_key")).toEqual([
      { key: "play", combo: "F10" },
      { key: "record", combo: "F9" },
    ]);
    expect([relay.settings.keys.record, relay.settings.keys.play]).toEqual(["F9", "F10"]);
  });

  test("keys Relay refuses say why, and the first run waits", async () => {
    render(AppWindow);
    await userEvent.click(button("Next"));
    await userEvent.click(screen.getByRole("radio", { name: /^F9 and F10/ }));
    core.fail("set_relay_key", "F10 already runs “Fill weekly timesheet”", "hotkey");
    await userEvent.click(button("Next"));
    await settle();
    expect(screen.getByRole("alert")).toHaveTextContent("F10 already runs “Fill weekly timesheet”");
    expect(screen.getByRole("heading", { name: "Your keys" })).toBeInTheDocument();
  });

  test("Record now ends the first run and records; Later just ends it", async () => {
    render(AppWindow);
    await userEvent.click(button("Next"));
    await userEvent.click(button("Next"));
    await settle();
    core.clearCalls();
    await userEvent.click(button("Record now"));
    await settle();
    expect(core.commands()).toEqual(["update_settings", "toggle_record"]);
    expect(core.lastArgs("update_settings")).toMatchObject({ settings: { intro: "done" } });
  });

  test("I'll choose my own opens Settings → Keys at the end", async () => {
    render(AppWindow);
    await userEvent.click(button("Next"));
    await userEvent.click(screen.getByRole("radio", { name: /I'll choose my own/ }));
    await userEvent.click(button("Next"));
    await userEvent.click(button("Later"));
    await settle();
    expect([relay.place, relay.settingsSection]).toEqual(["settings", "keys"]);
  });

  test("Back goes back", async () => {
    render(AppWindow);
    await userEvent.click(button("Next"));
    await userEvent.click(button("Back"));
    expect(screen.getByRole("heading", { name: /Do it once/ })).toBeInTheDocument();
  });
});

describe("an upgrade", () => {
  beforeEach(async () => {
    relay = await freshStore({ settings: { keys: LEGACY, intro: "keys" } });
  });

  test("shows the keys screen once, with its keys preselected, and Done ends it", async () => {
    render(AppWindow);
    expect(screen.getByRole("heading", { name: "Your keys" })).toBeInTheDocument();
    expect(screen.queryByRole("img", { name: /Step/ })).toBeNull();
    expect(screen.getByRole("radio", { name: /^F9 and F10/ })).toHaveAttribute("aria-checked", "true");
    core.clearCalls();
    await userEvent.click(button("Done"));
    await settle();
    expect(core.commands()).toEqual(["update_settings"]);
    expect(core.settings.intro).toBe("done");
    expect(core.settings.keys).toEqual(LEGACY);
  });

  test("Ctrl + Alt + End doesn't reach Relay over Remote Desktop: one click takes Ctrl + Alt + Backspace", async () => {
    render(AppWindow);
    const notice = screen.getByText(/To stop:/).closest<HTMLElement>(".notice")!;
    expect(notice).toHaveTextContent("Remote Desktop takes Ctrl + Alt + End for itself.");
    await userEvent.click(within(notice).getByRole("button", { name: "Use Ctrl + Alt + Backspace" }));
    await settle();
    expect(core.settings.keys.kill).toBe("Ctrl + Alt + Backspace");
    expect(notice).toHaveTextContent("It works over Remote Desktop too.");
  });
});

describe("no first run", () => {
  test("once it's done, or in the browser preview", async () => {
    relay = await freshStore({ settings: { intro: "done" } });
    const { container, unmount } = render(AppWindow);
    expect(container.querySelector(".app-window")).toHaveAttribute("data-place", "library");
    unmount();
    core.uninstall();
    try {
      relay = await freshStore({ backend: browserBackend() });
      const r = render(AppWindow);
      expect(r.container.querySelector(".app-window")).toHaveAttribute("data-place", "library");
    } finally {
      core.install();
    }
  });
});
