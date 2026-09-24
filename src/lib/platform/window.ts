// The native window, managed by Rust (placement, zoom, the tray). Everything
// is a no-op in a plain browser (`npm run dev`), where the widget renders on
// a demo desktop instead.
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

export const isTauri = (): boolean => "__TAURI_INTERNALS__" in window;

/**
 * Tells Rust the widget's size in CSS px. Rust keeps the bottom-center where
 * it was, keeps the widget inside the monitor's work area, zooms it down on
 * small screens and remembers the mode.
 */
export async function fitWindow(width: number, height: number, expanded: boolean): Promise<void> {
  if (isTauri()) await invoke("fit_window", { width, height, expanded });
}

/** Whether the widget was expanded last time (defaults to expanded). */
export async function savedExpanded(): Promise<boolean> {
  if (!isTauri()) return true;
  return (await invoke<{ expanded: boolean }>("window_prefs")).expanded;
}

export async function startDragging(): Promise<void> {
  if (isTauri()) await getCurrentWindow().startDragging();
}

/** The close button: hides to the tray (or quits, if that option is off). */
export async function hideToTray(): Promise<void> {
  if (isTauri()) await invoke("hide_to_tray");
}
