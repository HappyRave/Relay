import { describe, expect, test } from "vitest";
import { followStep, signature } from "./selection";
import type { Step } from "../types";

const wait = (t: number, dur: number, label = "", items = [t]): Step => ({ kind: "wait", t, end: t + dur, pause: 0, items, dur, label });
const click = (t: number, label: string, items = [t, t + 1]): Step => ({
  kind: "click",
  t,
  end: t + 60,
  pause: 0,
  items,
  x: 10,
  y: 20,
  btn: "Left",
  count: 1,
  label,
});

describe("signature", () => {
  test("ignores timing and event indices, not what the step does", () => {
    expect(signature(click(0, "Save", [1, 2]))).toBe(signature(click(900, "Save", [7, 8])));
    expect(signature(click(0, "Save"))).not.toBe(signature(click(0, "Open")));
    expect(signature(wait(0, 500))).not.toBe(signature(wait(0, 700)));
  });

  test("typed text is compared without its keystroke times", () => {
    const typed = (t: number): Step => ({ kind: "type", t, end: t + 90, pause: 0, items: [t], text: "hi", chars: [{ t, ch: "h" }, { t: t + 90, ch: "i" }] });
    expect(signature(typed(0))).toBe(signature(typed(500)));
  });
});

describe("followStep", () => {
  const before = [click(0, "File"), wait(100, 500, "Dialog"), click(700, "Save")];

  test("nothing open stays closed", () => {
    expect(followStep(before, before, -1)).toBe(-1);
  });

  test("an edit of the open step keeps its row, even when it looks different", () => {
    const after = [before[0], wait(100, 900, "Dialog"), before[2]];
    expect(followStep(before, after, 1, { op: "set_wait_duration", index: 1, dur: 900 })).toBe(1);
  });

  test("deleting the open step closes it", () => {
    expect(followStep(before, [before[0], before[2]], 1, { op: "delete_step", index: 1 })).toBe(-1);
  });

  test("deleting or inserting before it moves it along", () => {
    const shifted = [wait(100, 500, "Dialog", [3]), click(700, "Save", [4, 5])];
    expect(followStep(before, shifted, 2, { op: "delete_step", index: 0 })).toBe(1);
    const inserted = [before[0], wait(61, 500, "Inserted"), ...before.slice(1)];
    expect(followStep(before, inserted, 2, { op: "insert_wait", at: 0, dur: 500, label: "Inserted" })).toBe(3);
  });

  test("of two look-alikes, the nearest one wins; an undo that removed it closes it", () => {
    const twins = [click(0, "Tab"), click(100, "Tab"), click(200, "Tab")];
    expect(followStep(twins, twins.slice(1), 2)).toBe(1);
    expect(followStep(before, [before[0]], 2)).toBe(-1);
  });
});
