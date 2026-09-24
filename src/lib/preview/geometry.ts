// Geometry for the SVG preview: the mouse path and the cursor at a time.
import type { MovePoint, Rect } from "../types";

/** SVG path data for a polyline through all moves. */
export function pathD(moves: MovePoint[]): string {
  if (moves.length < 2) return "";
  let d = "M" + moves[0].x + " " + moves[0].y;
  for (let i = 1; i < moves.length; i++) d += " L" + moves[i].x + " " + moves[i].y;
  return d;
}

/** Cumulative path length at each move, so the "done" path can be drawn with a dash. */
export function cumulativeLengths(moves: MovePoint[]): Float64Array {
  const out = new Float64Array(moves.length);
  for (let i = 1; i < moves.length; i++) {
    out[i] = out[i - 1] + Math.hypot(moves[i].x - moves[i - 1].x, moves[i].y - moves[i - 1].y);
  }
  return out;
}

/** Index of the last move with `t <= time` (binary search), or -1. */
export function lastIndexAtOrBefore(moves: MovePoint[], time: number): number {
  let lo = 0;
  let hi = moves.length - 1;
  let ans = -1;
  while (lo <= hi) {
    const mid = (lo + hi) >> 1;
    if (moves[mid].t <= time) {
      ans = mid;
      lo = mid + 1;
    } else hi = mid - 1;
  }
  return ans;
}

/** The preview's aspect ratio (600 × 338). */
export const PREVIEW_ASPECT = 600 / 338;

/**
 * The part of the desktop to show: everything the macro touches, padded, at
 * the preview's aspect ratio, never narrower than `minWidth` (so a macro that
 * stays in one spot isn't blown up) and kept inside the desktop when it fits.
 */
export function fitView(desktop: Rect, points: { x: number; y: number }[], extra: Rect[] = [], minWidth = 960): Rect {
  const xs = points.map((p) => p.x).concat(extra.flatMap((r) => [r.x, r.x + r.w]));
  const ys = points.map((p) => p.y).concat(extra.flatMap((r) => [r.y, r.y + r.h]));
  if (!xs.length) return desktop;
  const minX = Math.min(...xs);
  const maxX = Math.max(...xs);
  const minY = Math.min(...ys);
  const maxY = Math.max(...ys);
  const pad = Math.max(60, 0.12 * Math.max(maxX - minX, maxY - minY));
  let w = Math.max(maxX - minX + 2 * pad, Math.min(minWidth, desktop.w));
  let h = Math.max(maxY - minY + 2 * pad, w / PREVIEW_ASPECT);
  w = Math.max(w, h * PREVIEW_ASPECT);
  if (w >= desktop.w && h >= desktop.h) return desktop;
  const clamp = (lo: number, size: number, dLo: number, dSize: number) =>
    size >= dSize ? dLo + (dSize - size) / 2 : Math.min(Math.max(lo, dLo), dLo + dSize - size);
  const x = clamp((minX + maxX) / 2 - w / 2, w, desktop.x, desktop.w);
  const y = clamp((minY + maxY) / 2 - h / 2, h, desktop.y, desktop.h);
  return { x: Math.round(x), y: Math.round(y), w: Math.round(w), h: Math.round(h) };
}
