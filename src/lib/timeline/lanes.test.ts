import { describe, expect, it } from "vitest";
import { currentStepIndex, jumpTarget, keyChips, moveSegments, pct, ruler, startedCount } from "./lanes";
import type { Step } from "../types";

const keys = (t: number, combo: string): Step => ({ kind: "keys", t, end: t + 350, pause: 0, combo: combo.split(" + "), items: [] });
const type = (t: number, text: string): Step => ({ kind: "type", t, end: t + 85 * text.length, pause: 0, text, chars: [], items: [] });

describe("pct", () => {
  it("clamps to 0..100", () => {
    expect(pct(-5, 100)).toBe(0);
    expect(pct(50, 100)).toBe(50);
    expect(pct(500, 100)).toBe(100);
  });
});

describe("moveSegments", () => {
  it("joins samples closer than 150 ms and drops single samples", () => {
    const moves = [0, 16, 32, 400, 1000, 1016].map((t) => ({ t, x: 0, y: 0 }));
    const segs = moveSegments(moves, 2000);
    expect(segs).toHaveLength(2);
    expect(segs[0].l).toBe(0);
    expect(segs[0].w).toBeCloseTo(1.6);
    expect(segs[1].l).toBe(50);
  });
});

describe("ruler", () => {
  it("picks the tick step from the duration", () => {
    expect(ruler(3000).map((r) => r.label)).toEqual(["0s", "1s", "2s"]);
    expect(ruler(20000)).toHaveLength(4);
    expect(ruler(61000)[1].label).toBe("15s");
  });
});

describe("keyChips", () => {
  it("sizes TYPE chips by their text and caps KEYS chips at 9%", () => {
    const chips = keyChips([keys(0, "Ctrl + A"), type(5000, "hello")], 10000);
    expect(chips[0]).toMatchObject({ label: "Ctrl + A", t: 0, w: 9 });
    expect(chips[1].label).toBe("hello");
    expect(chips[1].w).toBeCloseTo(4.25);
    expect(startedCount(chips, 100)).toBe(1);
  });
});

describe("step navigation", () => {
  const steps = [keys(1000, "A"), keys(2000, "B"), keys(3000, "C")];
  it("finds the current step", () => {
    expect(currentStepIndex(steps, 500)).toBe(-1);
    expect(currentStepIndex(steps, 2500)).toBe(1);
    expect(currentStepIndex(steps, 3000)).toBe(2);
    expect(startedCount([], 100)).toBe(0);
  });
  it("jumps to the previous and next steps", () => {
    expect(jumpTarget(steps, 2030, -1, 5000)).toBe(1000);
    expect(jumpTarget(steps, 2100, -1, 5000)).toBe(2000);
    expect(jumpTarget(steps, 2000, 1, 5000)).toBe(3000);
    expect(jumpTarget(steps, 3000, 1, 5000)).toBe(5000);
    expect(jumpTarget(steps, 500, -1, 5000)).toBe(0);
  });
});
