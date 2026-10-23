// The steps list's rows: one per step, with each cursor move folded into the
// step it leads to (unless Show moves is on), and what each row says.
import type { EditOp, Rect, Step } from "../types";

/** A row: its step's index, and the move folded into it (the step just before), if any. */
export type Row = { index: number; move: number | null };

/**
 * The rows for `steps`. A move leads to the step after it, so it folds into
 * that one; a move with nothing after it (the trip to the stop button) stays
 * a row of its own, so it can still be deleted.
 */
export function stepRows(steps: Step[], showMoves: boolean): Row[] {
  const rows: Row[] = [];
  steps.forEach((s, i) => {
    if (s.kind === "move" && !showMoves && steps[i + 1] && steps[i + 1].kind !== "move") return;
    const before = steps[i - 1];
    const folds = !showMoves && s.kind !== "move" && before?.kind === "move";
    rows.push({ index: i, move: folds ? i - 1 : null });
  });
  return rows;
}

/** The row showing step `index` (as its own step or as its folded move), or -1. */
export const rowOf = (rows: Row[], index: number) => rows.findIndex((r) => r.index === index || r.move === index);

/** The steps a row stands for, its folded move first. */
export const rowSteps = (r: Row) => (r.move == null ? [r.index] : [r.move, r.index]);

/** Deleting these rows, their folded moves too, as one edit: from the last step, so the others keep their index. */
export function deleteRows(rows: Row[]): EditOp {
  const indices = [...new Set(rows.flatMap(rowSteps))].sort((a, b) => b - a);
  const ops: EditOp[] = indices.map((index) => ({ op: "delete_step", index }));
  return ops.length === 1 ? ops[0] : { op: "batch", ops };
}

/** Where positions are shown: on the screen, or relative to the window the macro plays in. */
export type Frame = { anchor: Rect | null };

const signed = (n: number) => (n < 0 ? `${n}` : `+${n}`);
const at = (f: Frame, x: number, y: number) => (f.anchor ? `${signed(x - f.anchor.x)}, ${signed(y - f.anchor.y)}` : `${x}, ${y}`);
const seconds = (ms: number) => `${(ms / 1000).toFixed(ms < 10_000 ? 2 : 1)} s`;
const COUNT = ["", "Click", "Double click", "Triple click"];

/** "1 notch", "2 notches", "0.5 notch": a wheel notch is 120; precise wheels send less. */
export function notches(delta: number): string {
  const n = Math.abs(delta) / 120;
  const shown = Number.isInteger(n) ? String(n) : n < 0.05 ? n.toPrecision(1) : n.toFixed(1);
  return `${shown} ${Number(shown) > 1 ? "notches" : "notch"}`;
}

/** What a row says: its kind in a word, what it does, and a short aside (a length, a count). */
export type RowText = { kind: string; what: string; aside: string };

export function rowText(s: Step, move: Step | null, f: Frame): RowText {
  const there = move ? seconds(move.end - move.t) : "";
  switch (s.kind) {
    case "click": {
      const kind = s.btn === "Left" ? (COUNT[s.count] ?? `${s.count}× click`) : `${s.btn} click`;
      return { kind, what: s.label || at(f, s.x, s.y), aside: there };
    }
    case "drag":
      return { kind: s.btn === "Left" ? "Drag" : `${s.btn} drag`, what: s.label || `${at(f, s.x, s.y)} → ${at(f, s.to_x, s.to_y)}`, aside: there };
    case "scroll": {
      const dir = s.horizontal ? (s.delta > 0 ? "right" : "left") : s.delta > 0 ? "up" : "down";
      return { kind: "Scroll", what: `${dir} ${notches(s.delta)}`, aside: there };
    }
    case "keys":
      return { kind: "Keys", what: s.combo.join(" + "), aside: "" };
    case "type":
      return { kind: "Type", what: `“${s.text}”`, aside: `${[...s.text].length} chars` };
    case "text":
      return { kind: "Text", what: `“${s.text}”`, aside: "" };
    case "wait":
      return { kind: "Wait", what: s.label ? `${seconds(s.dur)} · ${s.label}` : seconds(s.dur), aside: "" };
    case "pixel_wait":
      return { kind: "Wait for", what: `pixel ${at(f, s.x, s.y)}${s.label ? ` · ${s.label}` : ""}`, aside: `≤ ${s.timeout_ms / 1000} s` };
    case "find_image":
      return { kind: "Find", what: s.label || "image", aside: `≤ ${s.timeout_ms / 1000} s` };
    case "move":
      return { kind: "Move", what: `${at(f, s.x, s.y)} → ${at(f, s.to_x, s.to_y)}`, aside: seconds(s.end - s.t) };
  }
}

/** Whether a row matches what's typed in Find a step: its kind or what it says. */
export function rowMatches(t: RowText, query: string): boolean {
  const q = query.trim().toLowerCase();
  return !q || `${t.kind} ${t.what}`.toLowerCase().includes(q);
}
