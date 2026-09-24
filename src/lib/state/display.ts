// Small display helpers shared by the compact bar and the expanded widget.
import type { Mode } from "../types";

export const BADGE: Record<Mode, string> = {
  idle: "Preview",
  count: "Get ready",
  rec: "● Rec",
  play: "Playing",
  pause: "Paused",
};

export const isRecordingMode = (m: Mode) => m === "rec" || m === "count";
