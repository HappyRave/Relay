// Which step the open step editor belongs to, across edits. A step has no id:
// its row number and its event indices both change when a step before it is
// inserted or deleted, so the editor follows what the user sees instead.
import type { EditOp, Step } from "../types";

/** What a step is to the user: its kind and fields, without its timing or event indices. */
export function signature(s: Step): string {
  const { t: _t, end: _end, pause: _pause, items: _items, ...rest } = s;
  if (rest.kind === "type") {
    const { chars: _chars, ...text } = rest;
    return JSON.stringify(text);
  }
  return JSON.stringify(rest);
}

/**
 * The row of the open step after `before` became `after` (by `op`, or an
 * undo or redo without one), or -1 once it's gone. An edit of the open step
 * itself keeps its row (it may have changed what the step looks like);
 * otherwise the step that looks the same, nearest to where it was.
 */
export function followStep(before: Step[], after: Step[], open: number, op?: EditOp): number {
  if (open < 0 || !before[open]) return -1;
  if (op && "index" in op && op.index === open) return op.op === "delete_step" || open >= after.length ? -1 : open;
  const sig = signature(before[open]);
  let best = -1;
  after.forEach((s, i) => {
    if (signature(s) === sig && (best < 0 || Math.abs(i - open) < Math.abs(best - open))) best = i;
  });
  return best;
}
