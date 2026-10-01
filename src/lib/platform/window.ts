// The native window, managed by Rust (placement, zoom, the tray). Everything
// is a no-op in a plain browser (`npm run dev`), where the widget renders on
// a demo desktop instead.
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { Panes } from "../ipc/bindings/Panes";
import type { WindowPrefsView } from "../ipc/bindings/WindowPrefsView";
import { NO_PANES } from "../layout";

export const isTauri = (): boolean => "__TAURI_INTERNALS__" in window;

/**
 * Tells Rust the compact player's size in CSS px. Rust keeps the
 * bottom-center where it was, keeps the widget inside the monitor's work
 * area, zooms it down on small screens and remembers the mode.
 */
export async function fitWindow(width: number, height: number, expanded: boolean): Promise<void> {
  if (isTauri()) await invoke("fit_window", { width, height, expanded });
}

/** Switches the window to the editor, whose size is Rust's (the user's, or the default). */
export const fitEditor = () => fitWindow(0, 0, true);

/** Whether the widget was expanded last time (defaults to expanded), and where the editor's dividers were. */
export async function savedWindow(): Promise<WindowPrefsView> {
  if (!isTauri()) return { expanded: true, panes: NO_PANES };
  return invoke<WindowPrefsView>("window_prefs");
}

/** Saves where the user put the editor's dividers (window.json). */
export async function savePanes(panes: Panes): Promise<void> {
  if (isTauri()) await invoke("save_panes", { panes });
}

/** *Reset layout*: the editor's default size and dividers. */
export async function resetLayout(): Promise<void> {
  if (isTauri()) await invoke("reset_layout");
}

export async function startDragging(): Promise<void> {
  if (isTauri()) await getCurrentWindow().startDragging();
}

/** The close button: hides to the tray (or quits, if that option is off). */
export async function hideToTray(): Promise<void> {
  if (isTauri()) await invoke("hide_to_tray");
}
