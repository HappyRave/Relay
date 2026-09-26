// Small display helpers shared by the compact bar and the expanded widget.
import type { Mode } from "../types";

export const BADGE: Record<Mode, string> = {
  idle: "Preview",
  countdown: "Get ready",
  recording: "● Rec",
  playing: "Playing",
  paused: "Paused",
};
