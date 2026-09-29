import { describe, expect, test } from "vitest";
import { BAR_H, MID_FROM, NARROW_MIN_W, WIDE_FROM, barMode, barScale, nextRepeat, nextSpeed } from "./transport";

describe("barMode", () => {
  test("a wide bar has everything; narrower, the settings compact", () => {
    expect(barMode(1600)).toBe("wide");
    expect(barMode(WIDE_FROM)).toBe("wide");
    expect(barMode(WIDE_FROM - 1)).toBe("mid");
    expect(barMode(1100)).toBe("mid");
    expect(barMode(MID_FROM)).toBe("mid");
    expect(barMode(MID_FROM - 1)).toBe("narrow");
    expect(barMode(940)).toBe("narrow"); // the default editor
    expect(barMode(756)).toBe("narrow"); // the smallest editor
  });

  test("not measured (jsdom): everything", () => {
    expect(barMode(0)).toBe("wide");
  });
});

describe("barScale", () => {
  test("the bar grows evenly with its row, up to twice its size", () => {
    expect(barScale(BAR_H, 940)).toBe(1);
    expect(barScale(153, 1400)).toBe(1.5);
    expect(barScale(400, 4000)).toBe(2);
  });

  test("never leaving the bar less room than the narrow mode needs", () => {
    expect(barScale(204, 1050)).toBe(1050 / NARROW_MIN_W);
    expect(barScale(204, 750)).toBe(1);
    expect(barScale(60, 940)).toBe(1); // never below its size
  });

  test("not measured: only the height counts", () => {
    expect(barScale(153, 0)).toBe(1.5);
  });
});

describe("the cycling buttons", () => {
  test("speed: 0.5× → 1× → 2× → 4× → 0.5×", () => {
    expect([0.5, 1, 2, 4].map(nextSpeed)).toEqual([1, 2, 4, 0.5]);
    expect(nextSpeed(1.5)).toBe(2); // a speed from a hand-edited file
  });

  test("repeat: 1 → 2 → 3 → 5 → 10 → forever → 1, and a count in between goes to the next step", () => {
    expect([1, 2, 3, 5, 10].map((count) => nextRepeat({ count }))).toEqual([
      { count: 2 },
      { count: 3 },
      { count: 5 },
      { count: 10 },
      "forever",
    ]);
    expect(nextRepeat("forever")).toEqual({ count: 1 });
    expect(nextRepeat({ count: 4 })).toEqual({ count: 5 });
    expect(nextRepeat({ count: 42 })).toBe("forever");
  });
});
