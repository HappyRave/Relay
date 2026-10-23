// The native window's commands, and that they're no-ops in a plain browser.
import { afterEach, beforeEach, describe, expect, test } from "vitest";
import { fitEditor, fitWindow, hideToTray, isTauri, minimize, savePanes, savedWindow, startDragging } from "./window";
import { core } from "../../test/fake-core";

beforeEach(() => core.reset());

describe("in the app", () => {
  test("each helper sends its command", async () => {
    expect(isTauri()).toBe(true);
    await fitWindow(604, 68, false);
    await fitEditor();
    core.window.expanded = false;
    expect(await savedWindow()).toEqual({ expanded: false, panes: { inspector_w: null } });
    await savePanes({ inspector_w: 420 });
    expect(core.window.panes).toEqual({ inspector_w: 420 });
    await startDragging();
    await minimize();
    await hideToTray();
    expect(core.calls).toEqual([
      { cmd: "fit_window", args: { width: 604, height: 68, expanded: false } },
      { cmd: "fit_window", args: { width: 0, height: 0, expanded: true } },
      { cmd: "window_prefs", args: {} },
      { cmd: "save_panes", args: { panes: { inspector_w: 420 } } },
      { cmd: "plugin:window|start_dragging", args: { label: "main" } },
      { cmd: "plugin:window|minimize", args: { label: "main" } },
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
    expect(await savedWindow()).toEqual({ expanded: true, panes: { inspector_w: null } });
    await expect(savePanes({ inspector_w: 1 })).resolves.toBeUndefined();
    await expect(startDragging()).resolves.toBeUndefined();
    await expect(minimize()).resolves.toBeUndefined();
    await expect(hideToTray()).resolves.toBeUndefined();
  });
});
