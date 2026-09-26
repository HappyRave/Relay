// The app shell: the widget in its two sizes, the export dialog, and the
// demo desktop of the browser preview.
import { afterEach, beforeEach, describe, expect, test } from "vitest";
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

/** A ResizeObserver the test can trigger. */
let resize: (w: number, h: number) => void = () => {};
const RealResizeObserver = globalThis.ResizeObserver;
beforeEach(() => {
  globalThis.ResizeObserver = class {
    constructor(cb: ResizeObserverCallback) {
      resize = (w, h) => cb([{ borderBoxSize: [{ inlineSize: w, blockSize: h }] } as unknown as ResizeObserverEntry], this);
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

  test("Cancel closes it", async () => {
    render(ExportDialog);
    await userEvent.click(screen.getByRole("button", { name: "Cancel" }));
    expect(relay.exportOpen).toBe(false);
    expect(core.commands()).toEqual([]);
  });

  test("a click on the backdrop closes it; a click inside doesn't", async () => {
    render(ExportDialog);
    const dialog = screen.getByRole("dialog");
    await fireEvent.click(screen.getByText("Export macro"));
    expect(relay.exportOpen).toBe(true);
    await fireEvent.click(dialog);
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
    expect(core.calls).toEqual([]);
  });

  test("the widget scales down to fit a small window", async () => {
    const { container } = render(App);
    await settle();
    resize(944, 616);
    await settle();
    const host = container.querySelector(".host") as HTMLElement;
    expect(host.style.transform).toMatch(/scale\((0\.\d+|1)\)/);
  });
});
