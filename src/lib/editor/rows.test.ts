import { describe, expect, test } from "vitest";
import samples from "../dev/sample-views.json";
import { deleteRows, rowMatches, rowOf, rowText, stepRows } from "./rows";
import type { MacroView, Step } from "../types";

const invoice = (samples[0] as unknown as { view: MacroView }).view;
const steps = invoice.steps;
const screen = { anchor: null };

describe("rows", () => {
  test("each move folds into the step it leads to", () => {
    const rows = stepRows(steps, false);
    expect(rows).toHaveLength(12);
    expect(rows[0]).toEqual({ index: 1, move: 0 }); // Click · File menu, and the move to it
    expect(rows[2]).toEqual({ index: 4, move: null }); // the wait: no move before it
    expect(rows.map((r) => r.index)).not.toContain(0);
  });

  test("Show moves gives every step its own row", () => {
    expect(stepRows(steps, true)).toEqual(steps.map((_, index) => ({ index, move: null })));
  });

  test("a move with nothing after it keeps its own row", () => {
    const tail = [...steps, { ...steps[0], t: 99_000, end: 99_500 }] as Step[];
    const rows = stepRows(tail, false);
    expect(rows.at(-1)).toEqual({ index: steps.length, move: null });
    // Two moves in a row: the first has no step of its own to fold into.
    const two = [steps[0], steps[0], steps[1]] as Step[];
    expect(stepRows(two, false)).toEqual([
      { index: 0, move: null },
      { index: 2, move: 1 },
    ]);
  });

  test("a step's row, whether it's the row's step or its move", () => {
    const rows = stepRows(steps, false);
    expect(rowOf(rows, 1)).toBe(0);
    expect(rowOf(rows, 0)).toBe(0);
    expect(rowOf(rows, 4)).toBe(2);
    expect(rowOf(rows, 99)).toBe(-1);
  });

  test("deleting rows deletes their moves too, from the last step, as one edit", () => {
    const rows = stepRows(steps, false);
    expect(deleteRows([rows[2]])).toEqual({ op: "delete_step", index: 4 });
    // Click · File menu (steps 0 and 1) and Double click · Filename field (5 and 6).
    expect(deleteRows([rows[0], rows[3]])).toEqual({
      op: "batch",
      ops: [6, 5, 1, 0].map((index) => ({ op: "delete_step", index })),
    });
  });

  test("what each row says", () => {
    const rows = stepRows(steps, false);
    const text = (i: number) => rowText(steps[rows[i].index], rows[i].move == null ? null : steps[rows[i].move!], screen);
    expect(text(0)).toEqual({ kind: "Click", what: "File menu", aside: "0.85 s" });
    expect(text(2)).toEqual({ kind: "Wait", what: "0.70 s · Dialog opens", aside: "" });
    expect(text(4)).toMatchObject({ kind: "Keys", what: "Ctrl + A" });
    expect(text(5)).toEqual({ kind: "Type", what: "“invoice_0924”", aside: "12 chars" });
    expect(text(7)).toEqual({ kind: "Wait for", what: "pixel 1248, 680 · Save button turns grey", aside: "≤ 5 s" });
  });

  test("positions relative to the window the macro plays in", () => {
    const pixel = steps[11];
    expect(rowText(pixel, null, { anchor: { x: 1000, y: 700, w: 800, h: 600 } }).what).toBe("pixel +248, -20 · Save button turns grey");
  });

  test("Find a step matches the kind or what it says", () => {
    const t = { kind: "Wait for", what: "pixel 1248, 680", aside: "" };
    expect(rowMatches(t, "")).toBe(true);
    expect(rowMatches(t, "WAIT")).toBe(true);
    expect(rowMatches(t, "1248")).toBe(true);
    expect(rowMatches(t, "click")).toBe(false);
  });
});
