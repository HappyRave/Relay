import { describe, expect, test } from "vitest";
import { playSummary } from "./summary";
import { DEFAULT_PLAYBACK } from "../defaults";

describe("the play summary", () => {
  test("repeats, speed and varied timing as chips, all of it in words", () => {
    const s = playSummary({ ...DEFAULT_PLAYBACK, repeat: { count: 3 } }, null);
    expect(s.chips).toEqual(["×3", "1×", "±40 ms"]);
    expect(s.label).toBe("Repeats 3 times, normal speed, timing varies by up to 40 ms, any key stops it. Open the play options.");
    expect(s.keysDontStop).toBe(false);
  });

  test("forever, faster, exact timing, and keys that don't stop it", () => {
    const s = playSummary({ ...DEFAULT_PLAYBACK, repeat: "forever", speed: 2, humanize: false, stop_on_key: false }, null);
    expect(s.chips).toEqual(["∞", "2×"]);
    expect(s.label).toBe("Loops until stopped, 2 times as fast, keys don't stop it. Open the play options.");
    expect(s.keysDontStop).toBe(true);
  });

  test("with a data file, once per row", () => {
    const s = playSummary({ ...DEFAULT_PLAYBACK, humanize: false }, 200);
    expect(s.chips).toEqual(["Each row (200)", "1×"]);
    expect(s.label).toMatch(/^Plays once per row of its data file \(200\), normal speed/);
    expect(playSummary({ ...DEFAULT_PLAYBACK, repeat: { count: 1 } }, null).label).toMatch(/^Repeats 1 time,/);
  });
});
