import { describe, expect, it } from "vitest";
import { currentStepIndex, jumpTarget, keyChips, moveBars, moveSegments, pauseSpans, pct, ruler, startedCount } from "./lanes";
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

const move = (t: number, end: number, pause = 0): Step => ({ kind: "move", t, end, pause, items: [], x: 0, y: 0, to_x: 1, to_y: 1, samples: 2 });
const click = (t: number, pause = 0): Step => ({ kind: "click", t, end: t + 60, pause, items: [], x: 1, y: 1, btn: "Left", count: 1, label: "" });

describe("moveBars", () => {
  it("is one bar per move or drag: a click is no movement, and a jump has no bar", () => {
    const drag: Step = { kind: "drag", t: 1500, end: 1800, pause: 0, items: [], x: 0, y: 0, to_x: 9, to_y: 9, btn: "Left", label: "" };
    const bars = moveBars([move(0, 800), click(800), move(860, 1400), drag, move(1900, 1900)], 2000);
    expect(bars.map((b) => [b.l, +b.w.toFixed(6)])).toEqual([
      [0, 40],
      [43, 27],
      [75, 15],
    ]);
  });

  it("a very short move still shows", () => {
    expect(moveBars([move(1000, 1001)], 2000)).toEqual([{ l: 50, w: 0.4 }]);
  });
});

describe("pauseSpans", () => {
  it("spans each pause before its step, however short, and nothing where there's none", () => {
    const spans = pauseSpans([move(0, 800), click(800), move(960, 1400, 100), click(1416, 16)], 2000);
    expect(spans.map((p) => [+p.l.toFixed(6), +p.w.toFixed(6)])).toEqual([
      [43, 5],
      [70, 0.8],
    ]);
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

  it("on a measured ruler, picks the smallest step that leaves each label 48 px", () => {
    // The default timeline's ruler is about 830 px: a 10 s macro gets a tick a second.
    expect(ruler(10000, 830)).toHaveLength(10);
    // Narrower, or a longer macro: fewer ticks.
    expect(ruler(10000, 300).map((r) => r.label)).toEqual(["0s", "2s", "4s", "6s", "8s"]);
    expect(ruler(60000, 830)[1].label).toBe("5s");
    expect(ruler(3_600_000, 830)[1].label).toBe("300s");
    // Wider: every second even for 20 s.
    expect(ruler(20000, 1400)).toHaveLength(20);
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
