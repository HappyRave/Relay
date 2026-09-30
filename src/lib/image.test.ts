import { describe, expect, test } from "vitest";
import { areaChoice, pngSize, pngUrl, testResult } from "./image";
import { png } from "../test/app";

const monitor = (x: number, primary: boolean) => ({
  name: `\\.\DISPLAY${primary ? 1 : 2}`,
  rect: { x, y: 0, w: 1920, h: 1080 },
  work: { x, y: 0, w: 1920, h: 1040 },
  dpi: 96,
  primary,
});

describe("images", () => {
  test("a PNG's size comes from its header, and anything else is 0 × 0", () => {
    expect(pngSize(png(512, 7))).toEqual([512, 7]);
    expect(pngSize("R0lGODlhAQABAAAAACw=")).toEqual([0, 0]); // a GIF
    expect(pngSize("not base64!")).toEqual([0, 0]);
    expect(pngSize("iVBORw0K")).toEqual([0, 0]); // cut short
    expect(pngUrl("iVBOR")).toBe("data:image/png;base64,iVBOR");
  });

  test("where to look: everywhere, a monitor, or an area set in the file", () => {
    const monitors = [monitor(0, true), monitor(1920, false)];
    const options = [
      [-1, "All screens"],
      [0, "Screen 1"],
      [1, "Screen 2"],
    ];
    expect(areaChoice(monitors, null)).toEqual({ value: -1, options });
    expect(areaChoice(monitors, { x: 1920, y: 0, w: 1920, h: 1080 })).toEqual({ value: 1, options });
    expect(areaChoice(monitors, { x: 5, y: 5, w: 10, h: 10 })).toEqual({ value: -2, options: [...options, [-2, "Custom"]] });
  });

  test("what Test found, in words", () => {
    const at = { x: -40, y: 12, w: 20, h: 10 };
    expect(testResult({ ...at, score: 90 }, 85)).toBe("Found at -40, 12 (90%)");
    expect(testResult({ ...at, score: 85 }, 85)).toBe("Found at -40, 12 (85%)");
    expect(testResult({ ...at, score: 84 }, 85)).toBe("Not found: the best match is 84%");
    expect(testResult(null, 85)).toBe("Not found");
  });
});
