// The native window's commands, and that they're no-ops in a plain browser.
import { afterEach, beforeEach, describe, expect, test } from "vitest";
import { fitWindow, hideToTray, isTauri, savedExpanded, startDragging } from "./window";
import { core } from "../../test/fake-core";

beforeEach(() => core.reset());

describe("in the app", () => {
  test("each helper sends its command", async () => {
    expect(isTauri()).toBe(true);
    await fitWindow(944, 612, true);
    core.window.expanded = false;
    expect(await savedExpanded()).toBe(false);
    await startDragging();
    await hideToTray();
    expect(core.calls).toEqual([
      { cmd: "fit_window", args: { width: 944, height: 612, expanded: true } },
      { cmd: "window_prefs", args: {} },
      { cmd: "plugin:window|start_dragging", args: { label: "main" } },
      { cmd: "hide_to_tray", args: {} },
    ]);
  });
});

describe("in a plain browser", () => {
  beforeEach(() => core.uninstall());
  afterEach(() => core.install());

  test("nothing is sent, and the widget opens expanded", async () => {
    expect(isTauri()).toBe(false);
    await fitWindow(1, 2, false);
    expect(await savedExpanded()).toBe(true);
    await startDragging();
    await hideToTray();
    expect(core.calls).toEqual([]);
  });
});
