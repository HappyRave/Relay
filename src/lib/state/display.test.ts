import { describe, expect, test } from "vitest";
import { tail } from "./display";

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
