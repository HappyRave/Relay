// Relay's native windows, managed by Rust (placement, zoom, the tray, which
// one shows). Everything is a no-op in a plain browser (`npm run dev`), where
// the window renders on a demo desktop instead.
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { Panes } from "../ipc/bindings/Panes";
import type { WindowPrefsView } from "../ipc/bindings/WindowPrefsView";
import { NO_PANES } from "../layout";

export const isTauri = (): boolean => "__TAURI_INTERNALS__" in window;

/** Which of Relay's windows this page is in: the main window or the player. */
export function windowLabel(): "main" | "player" {
  return isTauri() && getCurrentWindow().label === "player" ? "player" : "main";
}

/** Whether the player stood in for the window last time, and where the editor's divider was. */
export async function savedWindow(): Promise<WindowPrefsView> {
  if (!isTauri()) return { player_mode: false, panes: NO_PANES };
  return invoke<WindowPrefsView>("window_prefs");
}

/** The player takes the window's place (or gives it back). Rust tells both windows. */
export async function setPlayerMode(on: boolean): Promise<void> {
  if (isTauri()) await invoke("set_player_mode", { on });
}

/** The player's expand key: the window, on the player's macro. */
export async function showWindow(): Promise<void> {
  if (isTauri()) await invoke("show_window");
}

/** Saves where the user put the editor's divider (window.json). */
export async function savePanes(panes: Panes): Promise<void> {
  if (isTauri()) await invoke("save_panes", { panes });
}

export async function minimize(): Promise<void> {
  if (isTauri()) await getCurrentWindow().minimize();
}

export async function startDragging(): Promise<void> {
  if (isTauri()) await getCurrentWindow().startDragging();
}

/** The close button: hides to the tray (or quits, if that option is off). */
export async function hideToTray(): Promise<void> {
  if (isTauri()) await invoke("hide_to_tray");
}
