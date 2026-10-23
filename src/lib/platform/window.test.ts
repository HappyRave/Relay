// The native window's commands, and that they're no-ops in a plain browser.
import { afterEach, beforeEach, describe, expect, test } from "vitest";
import {
  hideToTray,
  isTauri,
  minimize,
  savePanes,
  savedWindow,
  setPlayerMode,
  showWindow,
  startDragging,
  windowLabel,
} from "./window";
import { core } from "../../test/fake-core";

beforeEach(() => core.reset());

describe("in the app", () => {
  test("each helper sends its command", async () => {
    expect(isTauri()).toBe(true);
    expect(windowLabel()).toBe("main");
    core.window.player_mode = true;
    expect(await savedWindow()).toEqual({ player_mode: true, panes: { inspector_w: null } });
    await setPlayerMode(false);
    await showWindow();
    await savePanes({ inspector_w: 420 });
    expect(core.window.panes).toEqual({ inspector_w: 420 });
    await startDragging();
    await minimize();
    await hideToTray();
    expect(core.calls).toEqual([
      { cmd: "window_prefs", args: {} },
      { cmd: "set_player_mode", args: { on: false } },
      { cmd: "show_window", args: {} },
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

  test("nothing is sent (there's no IPC to send it on), and the window isn't the player", async () => {
    expect(isTauri()).toBe(false);
    expect(windowLabel()).toBe("main");
    // Each would throw if it tried to invoke a command.
    await expect(setPlayerMode(true)).resolves.toBeUndefined();
    await expect(showWindow()).resolves.toBeUndefined();
    expect(await savedWindow()).toEqual({ player_mode: false, panes: { inspector_w: null } });
    await expect(savePanes({ inspector_w: 1 })).resolves.toBeUndefined();
    await expect(startDragging()).resolves.toBeUndefined();
    await expect(minimize()).resolves.toBeUndefined();
    await expect(hideToTray()).resolves.toBeUndefined();
  });
});
