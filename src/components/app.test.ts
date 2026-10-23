// The app shell: the widget in its two sizes, the export dialog, and the
// demo desktop of the browser preview.
import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import App from "../App.svelte";
import Widget from "./Widget.svelte";
import ExportDialog from "./ExportDialog.svelte";
import ExpandedWidget from "./ExpandedWidget.svelte";
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
      "fit_window",
      "get_settings",
      "list_macros",
      "load_macro",
      "screenshot",
      "get_triggers",
      "get_data_file",
      "get_autostart",
    ]);
    expect(core.lastArgs("fit_window")).toEqual({ width: 0, height: 0, expanded: true });
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

  test("the compact player tells Rust its size; the editor fills the window Rust sizes", async () => {
    const onresize = (w: number, h: number) => sizes.push([w, h]);
    const sizes: [number, number][] = [];
    const { container } = render(Widget, { onresize });
    await settle();
    // The editor only says it's the editor: its size is Rust's (the user's, or the default).
    expect(core.argsOf("fit_window")).toEqual([{ width: 0, height: 0, expanded: true }]);
    expect(container.querySelector(".widget")).toHaveClass("fill");
    resize(1200.4, 800.6); // the user resized the window
    await settle();
    expect(core.argsOf("fit_window")).toHaveLength(1);
    expect(sizes).toEqual([[1200, 801]]);

    relay.expanded = false;
    await settle();
    expect(container.querySelector(".widget")).not.toHaveClass("fill");
    resize(604.4, 67.6);
    await settle();
    expect(core.lastArgs("fit_window")).toEqual({ width: 604, height: 68, expanded: false });
    relay.expanded = true;
    await settle();
    expect(core.lastArgs("fit_window")).toEqual({ width: 0, height: 0, expanded: true });
  });

  test("switching to the compact player sends its size even if no resize is seen", async () => {
    // Expanded, then compact again before the page laid out the bigger window:
    // the widget measures what it did before, and the observer doesn't fire.
    const box = vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockReturnValue(new DOMRect(0, 0, 604.4, 67.6));
    try {
      render(Widget);
      await settle();
      relay.expanded = false;
      await settle();
      expect(core.argsOf("fit_window")).toEqual([
        { width: 0, height: 0, expanded: true },
        { width: 604, height: 68, expanded: false },
      ]);
    } finally {
      box.mockRestore();
    }
  });

  test("the compact player shows toasts too", async () => {
    relay.expanded = false;
    render(Widget);
    relay.notify("Saved");
    await settle();
    expect(screen.getByRole("status")).toHaveTextContent("Saved");
  });
});

describe("the editor's dividers", () => {
  beforeEach(async () => {
    relay = await freshStore({ init: false });
    core.window.panes = { preview_w: 520, transport_h: 153, timeline_h: 200 };
    await relay.init();
    await settle();
    core.clearCalls();
  });
  const drag = async (el: Element, axis: "clientX" | "clientY", from: number, to: number) => {
    await fireEvent.pointerDown(el, { button: 0, pointerId: 1, [axis]: from });
    await fireEvent.pointerMove(el, { pointerId: 1, [axis]: to });
    await fireEvent.pointerUp(el, { pointerId: 1, [axis]: to });
    await settle();
  };
  const style = (c: Element, selector: string) => (c.querySelector(selector) as HTMLElement).style;

  test("open where the user left them", () => {
    const { container } = render(ExpandedWidget);
    expect(relay.panes).toEqual({ preview_w: 520, transport_h: 153, timeline_h: 200 });
    expect(style(container, ".main").gridTemplateColumns).toBe("520px 2px minmax(0, 1fr)");
    expect(style(container, ".transport-pane").height).toBe("153px");
    expect(style(container, ".timeline-pane").height).toBe("200px");
    const names = screen.getAllByRole("separator").map((s) => s.getAttribute("aria-label"));
    expect(names).toEqual(["Resize preview", "Resize buttons", "Resize timeline"]);
  });

  test("a taller button row scales its controls", () => {
    const { container } = render(ExpandedWidget);
    // 153 px is 1.5 × the default 102 (jsdom measures no width to hold it back).
    expect(style(container, ".transport-scale").transform).toBe("scale(1.5)");
  });

  test("dragging one is saved once, when the drag ends", async () => {
    render(ExpandedWidget);
    await drag(screen.getByRole("separator", { name: "Resize preview" }), "clientX", 500, 560);
    expect(core.argsOf("save_panes")).toEqual([{ panes: { preview_w: 580, transport_h: 153, timeline_h: 200 } }]);
    // Up: the button row gets taller.
    await drag(screen.getByRole("separator", { name: "Resize buttons" }), "clientY", 300, 280);
    expect(core.lastArgs("save_panes")).toEqual({ panes: { preview_w: 580, transport_h: 173, timeline_h: 200 } });
    // Down: the timeline gets shorter (jsdom measures nothing, so it can't grow past its size here).
    await drag(screen.getByRole("separator", { name: "Resize timeline" }), "clientY", 400, 430);
    expect(core.lastArgs("save_panes")).toEqual({ panes: { preview_w: 580, transport_h: 173, timeline_h: 170 } });
    expect(core.window.panes).toEqual({ preview_w: 580, transport_h: 173, timeline_h: 170 });
  });

  test("double-clicking one puts it back to the default, and saves that", async () => {
    const { container } = render(ExpandedWidget);
    await fireEvent.dblClick(screen.getByRole("separator", { name: "Resize buttons" }));
    await settle();
    expect(core.lastArgs("save_panes")).toEqual({ panes: { preview_w: 520, transport_h: null, timeline_h: 200 } });
    expect(style(container, ".transport-pane").height).toBe("102px");
    expect(style(container, ".transport-scale").transform).toBe("");
  });
});

describe("Export dialog", () => {
  beforeEach(async () => {
    relay = await freshStore();
    relay.exportOpen = true;
  });

  test("is a modal with the formats, AutoHotkey for later", () => {
    render(ExportDialog);
    const dialog = screen.getByRole("dialog", { name: "Export macro" });
    expect(dialog).toHaveAttribute("open");
    const formats = [...dialog.querySelectorAll(".fmt")] as HTMLButtonElement[];
    expect(formats.map((f) => f.querySelector(".label")!.textContent)).toEqual([
      "Relay macro",
      "JSON events",
      "Standalone program",
      "AutoHotkey v2Coming later",
    ]);
    expect(formats.map((f) => f.disabled)).toEqual([false, false, false, true]);
    expect(formats[0]).toHaveClass("selected");
    expect(screen.getByText("export-invoice-to-pdf.rly")).toBeInTheDocument();
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
