// The native window's commands, and that they're no-ops in a plain browser.
import { afterEach, beforeEach, describe, expect, test } from "vitest";
import { fitEditor, fitWindow, hideToTray, isTauri, resetLayout, savePanes, savedWindow, startDragging } from "./window";
import { core } from "../../test/fake-core";

beforeEach(() => core.reset());

describe("in the app", () => {
  test("each helper sends its command", async () => {
    expect(isTauri()).toBe(true);
    await fitWindow(604, 68, false);
    await fitEditor();
    core.window.expanded = false;
    expect(await savedWindow()).toEqual({ expanded: false, panes: { preview_w: null, transport_h: null, timeline_h: null } });
    await savePanes({ preview_w: 480, transport_h: null, timeline_h: null });
    expect(core.window.panes).toEqual({ preview_w: 480, transport_h: null, timeline_h: null });
    await resetLayout();
    await startDragging();
    await hideToTray();
    expect(core.calls).toEqual([
      { cmd: "fit_window", args: { width: 604, height: 68, expanded: false } },
      { cmd: "fit_window", args: { width: 0, height: 0, expanded: true } },
      { cmd: "window_prefs", args: {} },
      { cmd: "save_panes", args: { panes: { preview_w: 480, transport_h: null, timeline_h: null } } },
      { cmd: "reset_layout", args: {} },
      { cmd: "plugin:window|start_dragging", args: { label: "main" } },
      { cmd: "hide_to_tray", args: {} },
    ]);
  });
});

describe("in a plain browser", () => {
  beforeEach(() => core.uninstall());
  afterEach(() => core.install());

  test("nothing is sent (there's no IPC to send it on), and the widget opens expanded", async () => {
    expect(isTauri()).toBe(false);
    // Each would throw if it tried to invoke a command.
    await expect(fitWindow(1, 2, false)).resolves.toBeUndefined();
    expect(await savedWindow()).toEqual({ expanded: true, panes: { preview_w: null, transport_h: null, timeline_h: null } });
    await expect(savePanes({ preview_w: 1, transport_h: 3, timeline_h: 2 })).resolves.toBeUndefined();
    await expect(resetLayout()).resolves.toBeUndefined();
    await expect(startDragging()).resolves.toBeUndefined();
    await expect(hideToTray()).resolves.toBeUndefined();
  });
});
