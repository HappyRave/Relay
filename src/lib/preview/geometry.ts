// Geometry for the SVG preview: the mouse path and the cursor at a time.
import type { MovePoint } from "../types";

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
