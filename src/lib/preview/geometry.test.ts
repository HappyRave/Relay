import { describe, expect, it } from "vitest";
import {
  MAX_ZOOM,
  PREVIEW_ASPECT,
  cumulativeLengths,
  fitDesktop,
  lastIndexAtOrBefore,
  pan,
  pathD,
  zoomAround,
  zoomedView,
} from "./geometry";

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

describe("the preview's view", () => {
  // Three monitors side by side, as on a wide desk.
  const desktop = { x: 0, y: 0, w: 6400, h: 1600 };

  it("fits the whole desktop in any shape of pane, centered", () => {
    expect(fitDesktop(desktop, 4)).toEqual(desktop);
    // A tall pane: the desktop's full width, centered vertically.
    expect(fitDesktop(desktop, 2)).toEqual({ x: 0, y: -800, w: 6400, h: 3200 });
    // A wide pane: the desktop's full height, centered horizontally.
    expect(fitDesktop(desktop, 8)).toEqual({ x: -3200, y: 0, w: 12800, h: 1600 });
    expect(fitDesktop({ x: -1920, y: 0, w: 3840, h: 1080 }, PREVIEW_ASPECT).w).toBe(3840);
  });

  it("without a zoom, it's the fitted desktop", () => {
    expect(zoomedView(desktop, 2, null)).toEqual(fitDesktop(desktop, 2));
    expect(zoomedView(desktop, 2, { scale: 1, cx: 5, cy: 5 })).toEqual(fitDesktop(desktop, 2));
  });

  it("zooms around the pointer, which stays in place, up to the maximum", () => {
    const z = zoomAround(desktop, 4, null, { x: 1600, y: 400 }, 2)!;
    expect(z.scale).toBe(2);
    const v = zoomedView(desktop, 4, z);
    expect([v.w, v.h]).toEqual([3200, 800]);
    // The point is a quarter of the way in, before and after.
    expect([(1600 - v.x) / v.w, (400 - v.y) / v.h]).toEqual([0.25, 0.25]);
    expect(zoomAround(desktop, 4, { scale: 12, cx: 3200, cy: 800 }, { x: 3200, y: 800 }, 4)!.scale).toBe(MAX_ZOOM);
  });

  it("zooming out to the whole desktop drops the zoom", () => {
    const z = zoomAround(desktop, 4, null, { x: 100, y: 100 }, 1.5);
    expect(zoomAround(desktop, 4, z, { x: 100, y: 100 }, 1 / 1.5)).toBeNull();
    expect(zoomAround(desktop, 4, null, { x: 100, y: 100 }, 0.5)).toBeNull();
  });

  it("a zoomed-in view never shows past the desktop's edges, even when zoomed near one or panned past it", () => {
    const z = zoomAround(desktop, 4, null, { x: 10, y: 10 }, 4)!;
    const v = zoomedView(desktop, 4, z);
    expect(v.x).toBeGreaterThanOrEqual(0);
    expect(v.y).toBeGreaterThanOrEqual(0);
    const panned = pan(desktop, 4, z, -5000, -5000)!;
    expect(zoomedView(desktop, 4, panned)).toMatchObject({ x: 0, y: 0 });
    // Panning back moves at once: the pan past the edge didn't pile up.
    expect(zoomedView(desktop, 4, pan(desktop, 4, panned, 100, 0))).toMatchObject({ x: 100 });
    expect(pan(desktop, 4, null, 100, 0)).toBeNull();
    // And the right and bottom edges.
    const far = zoomedView(desktop, 4, pan(desktop, 4, z, 50_000, 50_000));
    expect([far.x + far.w, far.y + far.h]).toEqual([6400, 1600]);
  });

  it("in a dimension where the zoomed view is still bigger than the desktop, it stays centered", () => {
    // A tall pane (aspect 1): at 2×, the view is 3200 × 3200, taller than the desktop.
    const v = zoomedView(desktop, 1, { scale: 2, cx: 500, cy: 0 });
    expect(v).toEqual({ x: 0, y: -800, w: 3200, h: 3200 });
  });
});
