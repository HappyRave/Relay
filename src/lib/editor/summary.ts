// The summary next to Play: how the macro will play, in a few chips, and in
// full for screen readers and the tooltip.
import type { PlaybackOptions } from "../types";

export type PlaySummary = { chips: string[]; label: string; keysDontStop: boolean };

const speedName = (speed: number) => (speed === 1 ? "normal speed" : `${speed} times as fast`);

/** `rows`: the data file's rows, when the macro has one (it then plays once per row). */
export function playSummary(p: PlaybackOptions, rows: number | null): PlaySummary {
  const count = p.repeat === "forever" ? null : p.repeat.count;
  const chips = [rows != null ? `Each row (${rows})` : count == null ? "∞" : `×${count}`, `${p.speed}×`];
  if (p.humanize) chips.push(`±${p.jitter_ms} ms`);
  const times =
    rows != null
      ? `Plays once per row of its data file (${rows})`
      : count == null
        ? "Loops until stopped"
        : `Repeats ${count} ${count === 1 ? "time" : "times"}`;
  const parts = [times, speedName(p.speed)];
  if (p.humanize) parts.push(`timing varies by up to ${p.jitter_ms} ms`);
  parts.push(p.stop_on_key ? "any key stops it" : "keys don't stop it");
  return { chips, label: `${parts.join(", ")}. Open the play options.`, keysDontStop: !p.stop_on_key };
}
