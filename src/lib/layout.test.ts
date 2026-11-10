import { describe, expect, test } from "vitest";
import { inspectorWidth, NO_PANES } from "./layout";

describe("the inspector's width", () => {
  test("the design's by default", () => {
    expect(inspectorWidth(NO_PANES, 900)).toEqual({ width: 340, max: 580 });
  });

  test("the user's, kept so the list keeps its room, and never below the narrowest", () => {
    expect(inspectorWidth({ inspector_w: 420 }, 900).width).toBe(420);
    expect(inspectorWidth({ inspector_w: 700 }, 900).width).toBe(580);
    expect(inspectorWidth({ inspector_w: 200 }, 900).width).toBe(280);
  });

  test("a smaller window squeezes it without forgetting it; before measuring, as it is", () => {
    const panes = { inspector_w: 500 };
    expect(inspectorWidth(panes, 700).width).toBe(380);
    expect(inspectorWidth(panes, 1200).width).toBe(500);
    expect(inspectorWidth(panes, 0)).toEqual({ width: 500, max: Infinity });
  });
});
