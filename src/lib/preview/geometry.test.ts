import { describe, expect, it } from "vitest";
import { cumulativeLengths, lastIndexAtOrBefore, pathD } from "./geometry";

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
