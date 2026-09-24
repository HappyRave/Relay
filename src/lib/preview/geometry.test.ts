import { describe, expect, it } from "vitest";
import { PREVIEW_ASPECT, cumulativeLengths, fitView, lastIndexAtOrBefore, pathD } from "./geometry";

const moves = [
  { t: 0, x: 0, y: 0 },
  { t: 10, x: 3, y: 4 },
  { t: 20, x: 3, y: 10 },
];

describe("preview geometry", () => {
  it("builds polyline path data", () => {
    expect(pathD(moves)).toBe("M0 0 L3 4 L3 10");
    expect(pathD(moves.slice(0, 1))).toBe("");
  });
  it("accumulates segment lengths", () => {
    expect(Array.from(cumulativeLengths(moves))).toEqual([0, 5, 11]);
  });
  it("binary-searches the last move at or before a time", () => {
    expect(lastIndexAtOrBefore(moves, -1)).toBe(-1);
    expect(lastIndexAtOrBefore(moves, 0)).toBe(0);
    expect(lastIndexAtOrBefore(moves, 15)).toBe(1);
    expect(lastIndexAtOrBefore(moves, 99)).toBe(2);
  });
});

describe("fitView", () => {
  const desktop = { x: 0, y: 0, w: 3440, h: 1440 };

  it("shows the whole desktop when there is nothing to fit", () => {
    expect(fitView(desktop, [])).toEqual(desktop);
  });

  it("zooms to the activity at the preview's aspect, inside the desktop", () => {
    const v = fitView(desktop, [{ x: 200, y: 200 }, { x: 520, y: 210 }]);
    expect(v.w).toBeGreaterThanOrEqual(960);
    expect(v.w / v.h).toBeCloseTo(PREVIEW_ASPECT, 1);
    expect(v.x).toBe(0); // clamped to the desktop's left edge
    expect(v.x + v.w).toBeGreaterThan(520);
    expect(v.y).toBeGreaterThanOrEqual(0);
  });

  it("includes extra rectangles such as the anchor window", () => {
    const v = fitView(desktop, [{ x: 2000, y: 700 }], [{ x: 1500, y: 300, w: 1200, h: 800 }]);
    expect(v.x).toBeLessThanOrEqual(1500);
    expect(v.x + v.w).toBeGreaterThanOrEqual(2700);
    expect(v.y + v.h).toBeGreaterThanOrEqual(1100);
  });

  it("falls back to the desktop when the activity covers it", () => {
    expect(fitView(desktop, [{ x: 0, y: 0 }, { x: 3440, y: 1440 }])).toEqual(desktop);
  });
});
