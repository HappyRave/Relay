// The editor's control bar (Design: "Control Bar Options", 1b "Anchored
// transport, single row"): the transport is pinned to the middle column, and
// every control is one height, so nothing drifts at any size. As the bar gets
// narrower, the settings on its right compact instead of wrapping.
import type { Repeat } from "./types";

/** The bar at scale 1: 16 + a 14 px label + 8 + a 48 px control + 16. */
export const BAR_H = 102;

/**
 * Labels sit above the controls at every width.
 * - `wide`: the four speeds, − count + and the loop button.
 * - `mid`: the speed is one button that cycles.
 * - `narrow`: the repeat folds into one button (×n) that cycles too.
 */
export type BarMode = "wide" | "mid" | "narrow";

/**
 * The bar's widths (CSS px, at scale 1) from which each mode fits, measured
 * in the app with the design's 24 px padding: wide needs 1273, mid 968,
 * narrow 746 (so the default 940 px bar is narrow, the smallest editor's
 * 756 px one too).
 */
export const WIDE_FROM = 1290;
export const MID_FROM = 980;
/** The narrow bar's own width: the bar is never scaled to less room than this. */
export const NARROW_MIN_W = 750;

/** Which controls fit a bar `width` px wide (0: not measured, as in jsdom: all of them). */
export function barMode(width: number): BarMode {
  if (width <= 0 || width >= WIDE_FROM) return "wide";
  return width >= MID_FROM ? "mid" : "narrow";
}

export const SPEEDS = [0.5, 1, 2, 4];

/** The next speed for the cycling button: 0.5× → 1× → 2× → 4× → 0.5×. */
export function nextSpeed(speed: number): number {
  return SPEEDS.find((s) => s > speed) ?? SPEEDS[0];
}

/** The folded repeat's steps; a count in between goes to the next one. */
export const REPEAT_STEPS: Repeat[] = [{ count: 1 }, { count: 2 }, { count: 3 }, { count: 5 }, { count: 10 }, "forever"];

/** The next repeat for the folded button: 1 → 2 → 3 → 5 → 10 → forever → 1. */
export function nextRepeat(repeat: Repeat): Repeat {
  if (repeat === "forever") return { count: 1 };
  return REPEAT_STEPS.find((r) => r === "forever" || r.count > repeat.count) ?? { count: 1 };
}

/**
 * How much the bar is scaled: with the row's height (1 at the default 80 px,
 * up to 2), but never so much that the bar has less room than the narrow
 * mode needs (a narrow window can't take big controls).
 */
export function barScale(height: number, width: number): number {
  const byHeight = height / BAR_H;
  const byWidth = width > 0 ? width / NARROW_MIN_W : Infinity;
  return Math.max(1, Math.min(2, byHeight, byWidth));
}
