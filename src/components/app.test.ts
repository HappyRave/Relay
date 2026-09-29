// The app shell: the widget in its two sizes, the export dialog, and the
// demo desktop of the browser preview.
import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import App from "../App.svelte";
import Widget from "./Widget.svelte";
import ExportDialog from "./ExportDialog.svelte";
import { core, freshStore, settle } from "../test/app";
import { browserBackend } from "../lib/ipc/backend";
import type { RelayStore } from "../lib/state/relay.svelte";

let relay: RelayStore;
const A = "00000000-0000-0000-0000-000000000001";

/** Resize observers the test can trigger: every one sees the widget at `w` × `h`. */
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
    relay = await freshStore({ init: false });
  });

  test("starts the store and shows the expanded widget once it's ready", async () => {
    const { container } = render(App);
    await settle();
    expect(core.commands()).toEqual([
      "window_prefs",
      "subscribe_engine",
      "get_settings",
      "list_macros",
      "load_macro",
      "screenshot",
      "get_triggers",
      "get_autostart",
    ]);
    expect(container.querySelector(".expanded")).not.toBeNull();
    expect(screen.getByRole("textbox", { name: "Macro name" })).toHaveValue("Export invoice to PDF");
  });

  test("opens compact when it was left compact", async () => {
    core.window.expanded = false;
    const { container } = render(App);
    await settle();
    expect(container.querySelector(".compact")).not.toBeNull();
    expect(container.querySelector(".expanded")).toBeNull();
  });

  test("the export dialog opens over the widget", async () => {
    render(App);
    await settle();
    await userEvent.click(screen.getByRole("button", { name: "Export" }));
    expect(screen.getByRole("dialog", { name: "Export macro" })).toBeInTheDocument();
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

describe("Widget", () => {
  beforeEach(async () => {
    relay = await freshStore();
  });

  test("switches between expanded and compact", async () => {
    const { container } = render(Widget);
    expect(container.querySelector(".expanded")).not.toBeNull();
    await userEvent.click(screen.getByRole("button", { name: "Compact player" }));
    expect(container.querySelector(".compact")).not.toBeNull();
    await userEvent.click(screen.getByRole("button", { name: "Expand" }));
    expect(container.querySelector(".expanded")).not.toBeNull();
  });

  test("tells Rust its size, so the window fits it", async () => {
    const onresize = (w: number, h: number) => sizes.push([w, h]);
    const sizes: [number, number][] = [];
    render(Widget, { onresize });
    await settle();
    resize(944.4, 612.6);
    await settle();
    expect(core.argsOf("fit_window")).toEqual([{ width: 944, height: 613, expanded: true }]);
    expect(sizes).toEqual([[944, 613]]);
    relay.expanded = false;
    await settle();
    resize(604, 68);
    await settle();
    expect(core.lastArgs("fit_window")).toEqual({ width: 604, height: 68, expanded: false });
  });

  test("the compact player shows toasts too", async () => {
    relay.expanded = false;
    render(Widget);
    relay.notify("Saved");
    await settle();
    expect(screen.getByRole("status")).toHaveTextContent("Saved");
  });
});

describe("Export dialog", () => {
  beforeEach(async () => {
    relay = await freshStore();
    relay.exportOpen = true;
  });

  test("is a modal with the formats, two of them for later", () => {
    render(ExportDialog);
    const dialog = screen.getByRole("dialog", { name: "Export macro" });
    expect(dialog).toHaveAttribute("open");
    const formats = [...dialog.querySelectorAll(".fmt")] as HTMLButtonElement[];
    expect(formats.map((f) => f.querySelector(".label")!.textContent)).toEqual([
      "Relay macro",
      "JSON events",
      "AutoHotkey v2Coming later",
      "Standalone .exeComing later",
    ]);
    expect(formats.map((f) => f.disabled)).toEqual([false, false, true, true]);
    expect(formats[0]).toHaveClass("selected");
    expect(screen.getByText("export-invoice-to-pdf.rly")).toBeInTheDocument();
  });

  test("choosing JSON changes the file name", async () => {
    render(ExportDialog);
    await userEvent.click(screen.getByText("JSON events"));
    expect(relay.exportFmt).toBe("json");
    expect(screen.getByText("export-invoice-to-pdf.json")).toBeInTheDocument();
  });

  test("a format for later can't be chosen", async () => {
    render(ExportDialog);
    await fireEvent.click(screen.getByText("AutoHotkey v2").closest("button")!);
    expect(relay.exportFmt).toBe("rly");
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
    await fireEvent.click(screen.getByText("Export macro"), { clientX: 300, clientY: 170 });
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

  test("puts the widget on a demo desktop with a hint", async () => {
    const { container } = render(App);
    await settle();
    await settle();
    expect(container.querySelector(".desktop")).not.toBeNull();
    expect(screen.getByText("Try it")).toBeInTheDocument();
    expect(screen.getByText("Relay.")).toBeInTheDocument();
    expect(container.querySelector(".expanded")).not.toBeNull();
    expect(screen.getByRole("textbox", { name: "Macro name" })).toHaveValue("Export invoice to PDF");
    expect(screen.queryByRole("alert")).toBeNull(); // nothing needed the app
  });

  test("the widget scales down to fit a small window", async () => {
    const was = [window.innerWidth, window.innerHeight];
    const setWindow = (w: number, h: number) => {
      Object.defineProperty(window, "innerWidth", { value: w, configurable: true });
      Object.defineProperty(window, "innerHeight", { value: h, configurable: true });
    };
    try {
      setWindow(1400, 900);
      const { container } = render(App);
      await settle();
      resize(944, 616);
      await settle();
      const host = container.querySelector(".host") as HTMLElement;
      expect(host.style.transform).toContain("scale(1)"); // it fits
      setWindow(800, 600);
      resize(944, 616);
      await settle();
      // The width is what limits it: (800 − 32) / 944, less than (600 − 52 − 16) / 616.
      expect(host.style.transform).toContain(`scale(${768 / 944})`);
    } finally {
      setWindow(was[0], was[1]);
    }
  });
});
