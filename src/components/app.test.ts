// The app shell: the main window or the player, the export dialog, and the
// demo desktop of the browser preview.
import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";
import { fireEvent, render, screen, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import App from "../App.svelte";
import ExportDialog from "./ExportDialog.svelte";
import { core, freshStore, settle } from "../test/app";
import { setMedia } from "../test/media";
import { browserBackend } from "../lib/ipc/backend";
import type { RelayStore } from "../lib/state/relay.svelte";

let relay: RelayStore;
const A = "00000000-0000-0000-0000-000000000001";

/** Resize observers the test can trigger: every one sees the page at `w` × `h`. */
let observers: ((w: number, h: number) => void)[] = [];
const resize = (w: number, h: number) => observers.forEach((o) => o(w, h));
const RealResizeObserver = globalThis.ResizeObserver;
beforeEach(() => {
  observers = [];
  globalThis.ResizeObserver = class {
    constructor(cb: ResizeObserverCallback) {
      observers.push((w, h) => cb([{ borderBoxSize: [{ inlineSize: w, blockSize: h }] } as unknown as ResizeObserverEntry], this));
    }
    observe() {}
    unobserve() {}
    disconnect() {}
  };
});
afterEach(() => {
  globalThis.ResizeObserver = RealResizeObserver;
});

describe("App", () => {
  beforeEach(async () => {
    relay = await freshStore({ init: false, settings: { intro: "done" } });
  });

  test("starts the store and shows the window on the Library once it's ready", async () => {
    const { container } = render(App);
    await settle();
    expect(core.commands()).toEqual([
      "window_prefs",
      "subscribe_engine",
      "get_settings",
      "system_look",
      "list_macros",
      "list_runs",
      "load_macro",
      "screenshot",
      "get_triggers",
      "get_data_file",
      "get_autostart",
    ]);
    expect(container.querySelector(".app-window")).toHaveAttribute("data-place", "library");
    expect(screen.getByText("Export invoice to PDF")).toBeInTheDocument();
  });

  test("puts the look on the page: the theme, spacing, glass and motion", async () => {
    render(App);
    await settle();
    const html = document.documentElement;
    expect(html).toHaveAttribute("data-theme", "light");
    expect(html).not.toHaveAttribute("data-glass");
    await relay.updateSettings({ theme: "dark", spacing: "large", glass: false, reduce_motion: "always" });
    await settle();
    expect(html).toHaveAttribute("data-theme", "dark");
    expect(html).toHaveAttribute("data-spacing", "large");
    expect(html).toHaveAttribute("data-glass", "off");
    expect(html).toHaveAttribute("data-motion", "calm");
  });

  test("follows Windows' dark mode and animation setting while it runs", async () => {
    render(App);
    await settle();
    const html = document.documentElement;
    setMedia("(prefers-color-scheme: dark)", true);
    setMedia("(prefers-reduced-motion: reduce)", true);
    await settle();
    expect(html).toHaveAttribute("data-theme", "dark");
    expect(html).toHaveAttribute("data-motion", "calm");
    setMedia("(prefers-color-scheme: dark)", false);
    await settle();
    expect(html).toHaveAttribute("data-theme", "light");
  });

  test("in player mode the main window still shows the window (Rust hides it)", async () => {
    core.window.player_mode = true;
    const { container } = render(App);
    await settle();
    expect(relay.playerMode).toBe(true);
    expect(container.querySelector(".app-window")).not.toBeNull();
    expect(screen.queryByRole("group", { name: "Relay player" })).toBeNull();
  });

  test("in the player's window, the player: it loads what's current, not the first macro, and no run history", async () => {
    core.uninstall();
    core.install("player");
    try {
      core.current = "00000000-0000-0000-0000-000000000003";
      const { container } = render(App);
      await settle();
      expect(relay.role).toBe("player");
      expect(screen.getByRole("group", { name: "Relay player" })).toBeInTheDocument();
      expect(container.querySelector(".app-window")).toBeNull();
      expect(core.commands()).toEqual([
        "window_prefs",
        "subscribe_engine",
        "get_settings",
        "system_look",
        "list_macros",
        "current_macro",
        "load_macro",
        "screenshot",
        "get_triggers",
        "get_data_file",
      ]);
      expect(core.lastArgs("load_macro")).toEqual({ id: "00000000-0000-0000-0000-000000000003" });
    } finally {
      core.uninstall();
      core.install();
    }
  });

  test("the export dialog opens over the widget", async () => {
    render(App);
    await settle();
    await relay.openMacro(A);
    await settle();
    await userEvent.click(screen.getByRole("button", { name: "More" }));
    await userEvent.click(screen.getByRole("menuitem", { name: "Export…" }));
    expect(screen.getByRole("dialog", { name: "Export “Export invoice to PDF”" })).toBeInTheDocument();
  });

  test("unmounting stops the store", async () => {
    const { unmount } = render(App);
    await settle();
    unmount();
    await relay.edit({ op: "delete_step", index: 0 });
    core.clearCalls();
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "z", ctrlKey: true, bubbles: true }));
    await settle();
    expect(core.commands()).toEqual([]);
  });
});

describe("Export dialog", () => {
  beforeEach(async () => {
    relay = await freshStore();
    relay.exportOpen = true;
  });

  test("is a modal with the formats as radios", () => {
    render(ExportDialog);
    const dialog = screen.getByRole("dialog", { name: "Export “Export invoice to PDF”" });
    expect(dialog).toHaveAttribute("open");
    const formats = within(screen.getByRole("radiogroup", { name: "Format" })).getAllByRole("radio");
    expect(formats.map((f) => f.querySelector(".label")!.textContent)).toEqual([
      "Relay macro",
      "Standalone program",
      "AutoHotkey v2",
      "JSON events",
    ]);
    expect(formats.map((f) => f.getAttribute("aria-checked"))).toEqual(["true", "false", "false", "false"]);
    expect(formats.map((f) => f.tabIndex)).toEqual([0, -1, -1, -1]);
    expect(screen.getByText("export-invoice-to-pdf.rly")).toBeInTheDocument();
  });

  test("arrow keys move the choice", async () => {
    render(ExportDialog);
    screen.getByRole("radio", { name: /Relay macro/ }).focus();
    await userEvent.keyboard("{ArrowDown}");
    expect(relay.exportFmt).toBe("exe");
    expect(screen.getByRole("radio", { name: /Standalone program/ })).toHaveFocus();
    await userEvent.keyboard("{ArrowUp}{ArrowUp}");
    expect(relay.exportFmt).toBe("json");
  });

  test("choosing JSON changes the file name", async () => {
    render(ExportDialog);
    await userEvent.click(screen.getByText("JSON events"));
    expect(relay.exportFmt).toBe("json");
    expect(screen.getByText("export-invoice-to-pdf.json")).toBeInTheDocument();
  });

  test("a standalone program is an .exe, and says how it plays", async () => {
    render(ExportDialog);
    expect(screen.queryByText(/saved options/)).not.toBeInTheDocument();
    await userEvent.click(screen.getByText("Standalone program"));
    expect(relay.exportFmt).toBe("exe");
    expect(screen.getByText("export-invoice-to-pdf.exe")).toBeInTheDocument();
    expect(screen.getByText("Plays with this macro's saved options. It isn't signed, so Windows may warn on another PC.")).toBeInTheDocument();
    core.dialog.save = "C:\\Users\\me\\invoice.exe";
    await userEvent.click(screen.getByRole("button", { name: "Save…" }));
    await settle();
    expect(core.lastArgs("export_macro")).toEqual({ id: A, format: "exe", path: "C:\\Users\\me\\invoice.exe" });
    expect(relay.toast?.message).toBe("Saved invoice.exe");
  });

  test("an AutoHotkey script is an .ahk, and says what it needs", async () => {
    render(ExportDialog);
    await userEvent.click(screen.getByText("AutoHotkey v2"));
    expect(relay.exportFmt).toBe("ahk");
    expect(screen.getByText("export-invoice-to-pdf.ahk")).toBeInTheDocument();
    expect(
      screen.getByText("Needs AutoHotkey v2. Clicks at screen coordinates; speed, repeat and Humanize are at the top of the script."),
    ).toBeInTheDocument();
    core.dialog.save = "C:\\Users\\me\\invoice.ahk";
    await userEvent.click(screen.getByRole("button", { name: "Save…" }));
    await settle();
    expect(core.lastArgs("plugin:dialog|save")).toMatchObject({
      options: { defaultPath: "export-invoice-to-pdf.ahk", filters: [{ name: "AutoHotkey script", extensions: ["ahk"] }] },
    });
    expect(core.lastArgs("export_macro")).toEqual({ id: A, format: "ahk", path: "C:\\Users\\me\\invoice.ahk" });
    expect(relay.toast?.message).toBe("Saved invoice.ahk");
  });

  test("Save… asks where, writes the file and closes", async () => {
    render(ExportDialog);
    core.dialog.save = "C:\\Users\\me\\invoice.rly";
    await userEvent.click(screen.getByRole("button", { name: "Save…" }));
    await settle();
    expect(core.commands()).toEqual(["plugin:dialog|save", "export_macro"]);
    expect(core.lastArgs("export_macro")).toEqual({ id: A, format: "rly", path: "C:\\Users\\me\\invoice.rly" });
    expect(relay.exportOpen).toBe(false);
    expect(relay.toast?.message).toBe("Saved invoice.rly");
  });

  test("cancelling the save dialog keeps the export dialog", async () => {
    render(ExportDialog);
    await userEvent.click(screen.getByRole("button", { name: "Save…" }));
    await settle();
    expect(relay.exportOpen).toBe(true);
  });

  test("a failed export keeps the dialog open and says why", async () => {
    render(ExportDialog);
    core.dialog.save = "C:\\Windows\\invoice.rly";
    core.fail("export_macro", "Couldn't save: access denied");
    await userEvent.click(screen.getByRole("button", { name: "Save…" }));
    await settle();
    expect(relay.exportOpen).toBe(true);
    expect(screen.getByRole("dialog")).toHaveAttribute("open");
    expect(relay.error).toBe("Couldn't save: access denied");
  });

  test("with no macro open, there's nothing to save", () => {
    relay.view = null;
    render(ExportDialog);
    expect(screen.getByRole("button", { name: "Save…" })).toBeDisabled();
  });

  test("Cancel closes it", async () => {
    render(ExportDialog);
    await userEvent.click(screen.getByRole("button", { name: "Cancel" }));
    expect(relay.exportOpen).toBe(false);
    expect(core.commands()).toEqual([]);
  });

  test("a click on the backdrop closes it; a click inside, even on its padding, doesn't", async () => {
    render(ExportDialog);
    const dialog = screen.getByRole("dialog");
    vi.spyOn(dialog, "getBoundingClientRect").mockReturnValue({ left: 250, top: 150, right: 690, bottom: 450, width: 440, height: 300 } as DOMRect);
    await fireEvent.click(screen.getByText("Export “Export invoice to PDF”"), { clientX: 300, clientY: 170 });
    expect(relay.exportOpen).toBe(true);
    await fireEvent.click(dialog, { clientX: 255, clientY: 440 }); // the padding: the target is the dialog itself
    expect(relay.exportOpen).toBe(true);
    await fireEvent.click(dialog, { clientX: 100, clientY: 440 }); // outside its box: the backdrop
    expect(relay.exportOpen).toBe(false);
  });

  test("the browser preview explains that exporting needs the app", async () => {
    core.uninstall();
    try {
      await freshStore({ backend: browserBackend() });
      render(ExportDialog);
      expect(screen.getByRole("button", { name: "Save…" })).toBeDisabled();
      expect(screen.getByText("Exporting needs the Relay app")).toBeInTheDocument();
    } finally {
      core.install();
    }
  });
});

describe("the browser preview", () => {
  beforeEach(async () => {
    core.uninstall();
    relay = await freshStore({ init: false, backend: browserBackend() });
  });
  afterEach(() => core.install());

  test("puts the window on a demo desktop with a hint", async () => {
    const { container } = render(App);
    await settle();
    await settle();
    expect(container.querySelector(".desktop")).not.toBeNull();
    expect(screen.getByText("Try it")).toBeInTheDocument();
    expect(screen.getByText("Relay.")).toBeInTheDocument();
    expect(container.querySelector(".app-window")).toHaveAttribute("data-place", "library");
    expect(screen.getByText("Export invoice to PDF")).toBeInTheDocument();
    expect(screen.queryByRole("alert")).toBeNull(); // nothing needed the app
  });

  test("the window scales down to fit a small browser window", async () => {
    const was = [window.innerWidth, window.innerHeight];
    const setWindow = (w: number, h: number) => {
      Object.defineProperty(window, "innerWidth", { value: w, configurable: true });
      Object.defineProperty(window, "innerHeight", { value: h, configurable: true });
    };
    try {
      setWindow(1400, 900);
      const { container } = render(App);
      await settle();
      resize(944, 612);
      await settle();
      const host = container.querySelector(".host") as HTMLElement;
      expect(host.style.transform).toContain("scale(1)"); // it fits
      setWindow(800, 600);
      resize(944, 612);
      await settle();
      // The width is what limits it: (800 − 32) / 944, less than (600 − 52 − 16) / 612.
      expect(host.style.transform).toContain(`scale(${768 / 944})`);
    } finally {
      setWindow(was[0], was[1]);
    }
  });
});

describe("the browser preview's player", () => {
  beforeEach(async () => {
    core.uninstall();
    relay = await freshStore({ init: false, backend: browserBackend() });
  });
  afterEach(() => core.install());

  test("player mode shows the player on the demo desktop, and its expand key the window again", async () => {
    const { container } = render(App);
    await settle();
    await userEvent.click(screen.getByRole("button", { name: "Switch to the player" }));
    await settle();
    expect(screen.getByRole("group", { name: "Relay player" })).toBeInTheDocument();
    expect(container.querySelector(".app-window")).toBeNull();
    await userEvent.click(screen.getByRole("button", { name: "Open the window" }));
    await settle();
    expect(container.querySelector(".app-window")).toHaveAttribute("data-place", "macro");
  });
});
