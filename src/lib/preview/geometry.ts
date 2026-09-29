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

/** The preview drawing's aspect ratio in the default layout (600 × 302, under its bar). */
export const PREVIEW_ASPECT = 600 / 302;
/** How far the preview zooms in. */
export const MAX_ZOOM = 16;

/**
 * The whole `desktop` at the drawing's `aspect` (width over height): the
 * smallest rect of that shape around it, centered, so none of it is ever
 * hidden, whatever the shape of the pane.
 */
export function fitDesktop(desktop: Rect, aspect: number): Rect {
  let [w, h] = [desktop.w, desktop.h];
  if (w / h > aspect) h = w / aspect;
  else w = h * aspect;
  return { x: desktop.x + (desktop.w - w) / 2, y: desktop.y + (desktop.h - h) / 2, w, h };
}

/** A zoom into the preview: `scale` (1: the whole desktop) around the view's center (`cx`, `cy`, desktop px). */
export interface Zoom {
  scale: number;
  cx: number;
  cy: number;
}

/**
 * The part of the desktop the preview shows: all of it at scale 1, or a
 * zoomed-in part, kept over the desktop (a view smaller than the desktop
 * never shows past its edges).
 */
export function zoomedView(desktop: Rect, aspect: number, zoom: Zoom | null): Rect {
  const base = fitDesktop(desktop, aspect);
  if (!zoom || zoom.scale <= 1) return base;
  const [w, h] = [base.w / zoom.scale, base.h / zoom.scale];
  const keep = (c: number, size: number, lo: number, len: number) =>
    size >= len ? lo + len / 2 : Math.min(Math.max(c, lo + size / 2), lo + len - size / 2);
  const [cx, cy] = [keep(zoom.cx, w, desktop.x, desktop.w), keep(zoom.cy, h, desktop.y, desktop.h)];
  return { x: cx - w / 2, y: cy - h / 2, w, h };
}

/** The zoom that shows `view`, so a pan past the edge doesn't pile up. */
const zoomOf = (view: Rect, scale: number): Zoom | null =>
  scale <= 1 ? null : { scale, cx: view.x + view.w / 2, cy: view.y + view.h / 2 };

/**
 * Zooming in (`factor` > 1) or out by `factor` around the desktop point `at`,
 * which stays where it is on screen; back at scale 1, no zoom (null).
 */
export function zoomAround(desktop: Rect, aspect: number, zoom: Zoom | null, at: { x: number; y: number }, factor: number): Zoom | null {
  const from = zoom?.scale ?? 1;
  const scale = Math.min(MAX_ZOOM, Math.max(1, from * factor));
  const view = zoomedView(desktop, aspect, zoom);
  const f = scale / from;
  const [cx, cy] = [view.x + view.w / 2, view.y + view.h / 2];
  const next = { scale, cx: at.x + (cx - at.x) / f, cy: at.y + (cy - at.y) / f };
  return zoomOf(zoomedView(desktop, aspect, next), scale);
}

/** Moving a zoomed-in view by (`dx`, `dy`) desktop px, kept over the desktop. */
export function pan(desktop: Rect, aspect: number, zoom: Zoom | null, dx: number, dy: number): Zoom | null {
  if (!zoom) return null;
  return zoomOf(zoomedView(desktop, aspect, { ...zoom, cx: zoom.cx + dx, cy: zoom.cy + dy }), zoom.scale);
}
