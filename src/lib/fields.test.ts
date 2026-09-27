import { describe, expect, test } from "vitest";
import { clamp, commitNumber, toMs } from "./fields";

function field(value: string): HTMLInputElement {
  const input = document.createElement("input");
  input.type = "number";
  input.value = value;
  return input;
}

describe("commitNumber", () => {
  const tolerance = (v: number) => clamp(Math.round(v), 0, 255);

  test("an accepted value is saved and shown as saved", () => {
    const input = field("12.6");
    expect(commitNumber(input, 8, tolerance)).toBe(13);
    expect(input.value).toBe("13");
  });

  test("a clamped value shows the limit; if that's the current value, nothing is saved", () => {
    const input = field("300");
    expect(commitNumber(input, 255, tolerance)).toBeNull();
    expect(input.value).toBe("255");
  });

  test("a refused or empty value puts the current one back", () => {
    const positive = (v: number) => (v >= 0 ? toMs(v) : null);
    const seconds = (ms: number) => (ms / 1000).toFixed(1);
    for (const typed of ["-1", ""]) {
      const input = field(typed);
      expect(commitNumber(input, 1400, positive, seconds)).toBeNull();
      expect(input.value).toBe("1.4");
    }
  });
});

test("toMs rounds to whole milliseconds", () => {
  expect(toMs(1.2346)).toBe(1235);
  expect(toMs(2.5)).toBe(2500);
});
