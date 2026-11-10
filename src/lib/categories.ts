import type { Step } from "./types";

/** A step's color block: the pointer (blue, a circle), the keyboard (yellow, a square),
 *  logic (green, a triangle), or a move (pale blue, a ring). */
export type Category = "pointer" | "keys" | "logic" | "move";

export function category(kind: Step["kind"]): Category {
  switch (kind) {
    case "click":
    case "drag":
    case "scroll":
      return "pointer";
    case "move":
      return "move";
    case "keys":
    case "type":
    case "text":
      return "keys";
    case "wait":
    case "pixel_wait":
    case "find_image":
      return "logic";
  }
}
