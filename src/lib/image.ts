// Find image steps and the image trigger carry their image as a base64 PNG.
import type { FoundImage } from "./ipc/bindings/FoundImage";
import type { MonitorInfo } from "./ipc/bindings/MonitorInfo";
import type { Rect } from "./types";

/** The width and height of a base64 PNG (from its header), or [0, 0] if it isn't one. */
export function pngSize(png: string): [number, number] {
  let bytes: string;
  try {
    bytes = atob(png.slice(0, 32));
  } catch {
    return [0, 0];
  }
  if (!bytes.startsWith("\x89PNG") || bytes.length < 24) return [0, 0];
  const u32 = (i: number) => ((bytes.charCodeAt(i) << 24) | (bytes.charCodeAt(i + 1) << 16) | (bytes.charCodeAt(i + 2) << 8) | bytes.charCodeAt(i + 3)) >>> 0;
  return [u32(16), u32(20)];
}

/** A base64 PNG as an image URL. */
export const pngUrl = (png: string) => `data:image/png;base64,${png}`;

const sameRect = (a: Rect, b: Rect) => a.x === b.x && a.y === b.y && a.w === b.w && a.h === b.h;

/**
 * Where to look, as a choice: -1 everywhere, a monitor's index, or -2 an
 * area that's neither (set in a file), with the options to show.
 */
export function areaChoice(monitors: MonitorInfo[], area: Rect | null): { value: number; options: [number, string][] } {
  const i = area ? monitors.findIndex((m) => sameRect(m.rect, area)) : -1;
  const value = area && i < 0 ? -2 : i;
  const options: [number, string][] = [[-1, "All screens"], ...monitors.map((_, i): [number, string] => [i, `Screen ${i + 1}`])];
  return { value, options: value === -2 ? [...options, [-2, "Custom"]] : options };
}

/** What "Test" found, in words. */
export function testResult(m: FoundImage | null, threshold: number): string {
  if (!m) return "Not found";
  return m.score >= threshold ? `Found at ${m.x}, ${m.y} (${m.score}%)` : `Not found: the best match is ${m.score}%`;
}
