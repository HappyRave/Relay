import { describe, expect, test } from "vitest";
import { stepTitle, tail } from "./display";
import type { Step } from "../types";

describe("tail", () => {
  test("keeps short text whole, and the end of long text", () => {
    expect(tail("invoice", 16)).toBe("invoice");
    expect(tail("a".repeat(16), 16)).toBe("a".repeat(16));
    expect(tail("0123456789abcdefXYZ", 16)).toBe("…3456789abcdefXYZ");
  });

  test("counts characters, not UTF-16 units", () => {
    expect(tail("😀😀😀", 2)).toBe("…😀😀");
  });
});

describe("stepTitle", () => {
  const base = { t: 0, end: 0, items: [0] };
  test.each([
    [{ ...base, kind: "click", btn: "Left", count: 2, x: 1, y: 2, label: "Filename field" }, "Double click · Filename field"],
    [{ ...base, kind: "click", btn: "Right", count: 1, x: 1, y: 2, label: "" }, "Right click"],
    [{ ...base, kind: "keys", combo: ["Ctrl", "A"] }, "Ctrl + A"],
    [{ ...base, kind: "wait", dur: 700, label: "Dialog opens" }, "Wait 0.7 s"],
  ])("%#: %s", (step, title) => {
    expect(stepTitle(step as unknown as Step)).toBe(title);
  });
});
