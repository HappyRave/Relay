// Small display helpers shared by the compact bar and the expanded widget.
import type { Mode, Step } from "../types";

export const BADGE: Record<Mode, string> = {
  idle: "Preview",
  countdown: "Get ready",
  recording: "● Rec",
  playing: "Playing",
  paused: "Paused",
};

/** The last `max` characters of `text`, after "…" when some are left out. */
export function tail(text: string, max: number): string {
  const chars = [...text];
  return chars.length > max ? "…" + chars.slice(-max).join("") : text;
}

const COUNT_NAME = ["", "Click", "Double click", "Triple click"];

/** A step's title, as in the steps list: "Double click · Filename field", "Ctrl + A", "Wait 0.7 s". */
export function stepTitle(s: Step): string {
  switch (s.kind) {
    case "click": {
      const what =
        s.btn === "Left"
          ? (COUNT_NAME[s.count] ?? `${s.count}× click`)
          : `${s.btn} click`;
      return what + (s.label ? " · " + s.label : "");
    }
    case "drag":
      return `${s.btn === "Left" ? "Drag" : s.btn + " drag"}${s.label ? " · " + s.label : ""}`;
    case "scroll":
      return `Scroll ${s.horizontal ? (s.delta > 0 ? "right" : "left") : s.delta > 0 ? "up" : "down"}`;
    case "keys":
      return s.combo.join(" + ");
    case "type":
      return "“" + s.text + "”";
    case "pixel_wait":
      return `Wait for pixel ${s.x}, ${s.y} = ${s.color}`;
    case "wait":
      return "Wait " + (s.dur / 1000).toFixed(1) + " s";
  }
}
