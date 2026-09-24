// Thin wrappers over the Tauri window API. Everything is a no-op in a plain
// browser (`npm run dev`), where the widget renders on a demo desktop instead.
import { getCurrentWindow, LogicalSize, PhysicalPosition } from "@tauri-apps/api/window";

export const isTauri = (): boolean => "__TAURI_INTERNALS__" in window;

/**
 * Resizes the window to the widget's logical size, keeping the bottom-center
 * point fixed (the design's `transform-origin: 50% 100%`). M6 moves this to
 * Rust with a single SetWindowPos and work-area clamping.
 */
export async function fitWindow(width: number, height: number): Promise<void> {
  if (!isTauri()) return;
  const win = getCurrentWindow();
  // Use the inner size for the delta: the outer size includes invisible resize
  // borders, which don't change between modes.
  const [pos, size, scale] = await Promise.all([win.outerPosition(), win.innerSize(), win.scaleFactor()]);
  const w = Math.round(width * scale);
  const h = Math.round(height * scale);
  if (w === size.width && h === size.height) return;
  await win.setSize(new LogicalSize(width, height));
  await win.setPosition(new PhysicalPosition(pos.x + Math.round((size.width - w) / 2), pos.y + size.height - h));
}

export async function startDragging(): Promise<void> {
  if (isTauri()) await getCurrentWindow().startDragging();
}

export async function closeWindow(): Promise<void> {
  if (isTauri()) await getCurrentWindow().close();
}
