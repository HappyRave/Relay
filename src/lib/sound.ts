// Relay's sounds: short, dry clicks at a low volume (Windows' own volume
// applies). Off by default (Settings → Sound); they never replace words.

export type SoundName = "record" | "stop" | "done" | "error" | "step";

/** Each sound's notes (Hz), and how long each lasts (s). */
export const SOUNDS: Record<SoundName, { notes: number[]; len: number }> = {
  /** Two quick rising clicks. */
  record: { notes: [660, 990], len: 0.05 },
  /** Two quick falling clicks. */
  stop: { notes: [990, 660], len: 0.05 },
  /** Three rising notes. */
  done: { notes: [784, 1047, 1319], len: 0.06 },
  /** Two low notes. */
  error: { notes: [220, 196], len: 0.09 },
  /** One very short tick. */
  step: { notes: [1400], len: 0.02 },
};

const VOLUME = 0.06;
const GAP = 0.03;

let audio: AudioContext | null = null;

/** Plays `name`; nothing where there's no audio (tests, a browser without it). */
export function playSound(name: SoundName): void {
  if (typeof AudioContext === "undefined") return;
  audio ??= new AudioContext();
  void audio.resume().catch(() => {});
  const { notes, len } = SOUNDS[name];
  let t = audio.currentTime;
  for (const f of notes) {
    const osc = audio.createOscillator();
    const gain = audio.createGain();
    osc.type = "square";
    osc.frequency.value = f;
    gain.gain.setValueAtTime(0.0001, t);
    gain.gain.exponentialRampToValueAtTime(VOLUME, t + 0.005);
    gain.gain.exponentialRampToValueAtTime(0.0001, t + len);
    osc.connect(gain).connect(audio.destination);
    osc.start(t);
    osc.stop(t + len + 0.02);
    t += len + GAP;
  }
}
