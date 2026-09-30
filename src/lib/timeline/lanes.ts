// Presentation math for the four-lane timeline (Mouse, Clicks, Keys, Logic).
import type { MovePoint, Step } from "../types";

export const pct = (t: number, duration: number) => Math.max(0, Math.min(100, (t / duration) * 100));

export interface Span {
  l: number;
  w: number;
}

/** Bars for the mouse moving: one per MOVE or DRAG step. A jump (a move with a single sample) has none. */
export function moveBars(steps: Step[], duration: number): Span[] {
  return steps
    .filter((s) => (s.kind === "move" || s.kind === "drag") && s.end > s.t)
    .map((s) => ({ l: pct(s.t, duration), w: Math.max(0.4, pct(s.end, duration) - pct(s.t, duration)) }));
}

/** The pauses before steps, when nothing happens, however short. */
export function pauseSpans(steps: Step[], duration: number): Span[] {
  return steps
    .filter((s) => s.pause > 0)
    .map((s) => ({ l: pct(s.t - s.pause, duration), w: pct(s.t, duration) - pct(s.t - s.pause, duration) }));
}

/**
 * Bars for the mouse moving while recording, when the MOVE steps aren't
 * known yet (the steps are grouped as actions come): samples closer than
 * 150 ms join one bar.
 */
export function moveSegments(moves: MovePoint[], duration: number): Span[] {
  const segs: { s: number; e: number }[] = [];
  let seg: { s: number; e: number } | null = null;
  for (const m of moves) {
    if (seg && m.t - seg.e < 150) seg.e = m.t;
    else {
      if (seg) segs.push(seg);
      seg = { s: m.t, e: m.t };
    }
  }
  if (seg) segs.push(seg);
  return segs
    .filter((x) => x.e > x.s)
    .map((x) => ({ l: pct(x.s, duration), w: Math.max(0.4, pct(x.e, duration) - pct(x.s, duration)) }));
}

export interface RulerTick {
  l: number;
  label: string;
}

/** Ruler steps, and the room a label needs between ticks (px). */
const STEPS = [1000, 2000, 5000, 10000, 15000, 30000, 60000, 120000, 300000, 600000];
const TICK_ROOM = 48;

/**
 * Ruler ticks: on a ruler `width` px wide, the smallest step that leaves each
 * label room; not measured (0), every 1 s, 5 s or 15 s by the macro's length.
 */
export function ruler(duration: number, width = 0): RulerTick[] {
  const step =
    width > 0
      ? (STEPS.find((s) => (s / duration) * width >= TICK_ROOM) ?? STEPS[STEPS.length - 1])
      : duration <= 16000
        ? 1000
        : duration <= 60000
          ? 5000
          : 15000;
  const out: RulerTick[] = [];
  for (let t = 0; t < duration; t += step) out.push({ l: pct(t, duration), label: t / 1000 + "s" });
  return out;
}

export interface KeyChip extends Span {
  t: number;
  label: string;
}

/** How many of `items` (sorted by `t`) have started at `cur`. */
export function startedCount(items: { t: number }[], cur: number): number {
  let lo = 0;
  let hi = items.length;
  while (lo < hi) {
    const mid = (lo + hi) >> 1;
    if (items[mid].t <= cur) lo = mid + 1;
    else hi = mid;
  }
  return lo;
}

/** Chips on the Keys lane; a TYPE chip spans its characters, a KEYS chip grows up to the next chip. */
export function keyChips(steps: Step[], duration: number): KeyChip[] {
  const ks = steps.filter((x) => x.kind === "keys" || x.kind === "type");
  return ks.map((x, i) => {
    const nx = ks[i + 1];
    const gap = nx ? pct(nx.t, duration) - pct(x.t, duration) - 0.3 : 100 - pct(x.t, duration);
    const w = x.kind === "type" ? pct(x.end, duration) - pct(x.t, duration) : Math.min(gap, 9);
    return {
      l: pct(x.t, duration),
      w: Math.max(0.8, w),
      t: x.t,
      label: x.kind === "type" ? x.text : x.combo.join(" + "),
    };
  });
}

/** Index of the last step starting at or before `cur`, or -1 (steps are sorted by start). */
export function currentStepIndex(steps: Step[], cur: number): number {
  return startedCount(steps, cur) - 1;
}

/** Previous/next step start used by the transport's step buttons (60 ms / 1 ms thresholds). */
export function jumpTarget(steps: Step[], cur: number, dir: -1 | 1, duration: number): number {
  const ts = steps.map((s) => s.t);
  if (dir < 0) {
    const prev = ts.filter((x) => x < cur - 60).pop();
    return prev ?? 0;
  }
  return ts.find((x) => x > cur + 1) ?? duration;
}
