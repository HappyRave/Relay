import { afterEach, describe, expect, test, vi } from "vitest";
import { SOUNDS, playSound } from "./sound";

describe("sounds", () => {
  afterEach(() => vi.unstubAllGlobals());

  test("each is a few short notes, rising for a start or a finish, falling for a stop, low for a failure", () => {
    expect(SOUNDS.record.notes).toEqual([660, 990]);
    expect(SOUNDS.stop.notes).toEqual([990, 660]);
    expect(SOUNDS.done.notes).toEqual([784, 1047, 1319]);
    expect(Math.max(...SOUNDS.error.notes)).toBeLessThan(250);
    expect(SOUNDS.step.notes).toHaveLength(1);
    for (const s of Object.values(SOUNDS)) expect(s.len).toBeLessThanOrEqual(0.1);
  });

  test("without audio, nothing happens", () => {
    expect(() => playSound("done")).not.toThrow();
  });

  test("with audio, one oscillator per note, started one after the other", () => {
    const starts: number[] = [];
    const node = () => ({ connect: (n: unknown) => n, frequency: { value: 0 }, gain: { setValueAtTime() {}, exponentialRampToValueAtTime() {} }, start: (t: number) => starts.push(t), stop() {} });
    vi.stubGlobal(
      "AudioContext",
      class {
        currentTime = 1;
        destination = {};
        resume = () => Promise.resolve();
        createOscillator = node;
        createGain = node;
      },
    );
    playSound("done");
    expect(starts).toHaveLength(3);
    expect(starts[1] - starts[0]).toBeCloseTo(0.09);
  });
});
