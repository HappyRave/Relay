// Small display helpers for the compact player.
import type { Mode } from "../types";

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
