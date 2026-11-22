import { describe, expect, test } from "vitest";
import { placeCallouts } from "./callouts";

const overlap = (a: { bx: number; by: number }, b: { bx: number; by: number }) => Math.abs(a.bx - b.bx) < 24 && Math.abs(a.by - b.by) < 24;

describe("callouts", () => {
  test("beside their click, left of it near the right edge", () => {
    const [a, b] = placeCallouts([{ x: 50, y: 60 }, { x: 260, y: 60 }], 300, 150);
    expect([a.bx, a.left]).toEqual([68, false]);
    expect([b.bx, b.left]).toEqual([242, true]);
  });

  test("clicks in the same place get callouts that don't cover each other", () => {
    const spots = placeCallouts(Array.from({ length: 6 }, () => ({ x: 150, y: 75 })), 300, 150);
    for (let i = 0; i < spots.length; i++) for (let j = i + 1; j < spots.length; j++) expect(overlap(spots[i], spots[j])).toBe(false);
  });

  test("kept inside the stage", () => {
    for (const c of placeCallouts([{ x: 0, y: 0 }, { x: 300, y: 150 }, { x: 299, y: 1 }], 300, 150)) {
      expect(c.bx).toBeGreaterThanOrEqual(12);
      expect(c.bx).toBeLessThanOrEqual(288);
      expect(c.by).toBeGreaterThanOrEqual(12);
      expect(c.by).toBeLessThanOrEqual(138);
    }
  });
});
