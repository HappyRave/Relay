// Presentation math for the four-lane timeline (Mouse, Clicks, Keys, Logic).
import type { MovePoint, Step } from "../types";

export const pct = (t: number, duration: number) => Math.max(0, Math.min(100, (t / duration) * 100));

export interface Span {
  l: number;
  w: number;
}

/** Bars for continuous mouse movement: samples closer than 150 ms join one bar. */
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

/** Ruler ticks every 1 s, 5 s or 15 s depending on the macro length. */
export function ruler(duration: number): RulerTick[] {
  const step = duration <= 16000 ? 1000 : duration <= 60000 ? 5000 : 15000;
  const out: RulerTick[] = [];
  for (let t = 0; t < duration; t += step) out.push({ l: pct(t, duration), label: t / 1000 + "s" });
  return out;
}

export interface KeyChip extends Span {
  label: string;
  past: boolean;
}

/** Chips on the Keys lane; a TYPE chip spans its characters, a KEYS chip grows up to the next chip. */
export function keyChips(steps: Step[], duration: number, cur: number): KeyChip[] {
  const ks = steps.filter((x) => x.kind === "keys" || x.kind === "type");
  return ks.map((x, i) => {
    const nx = ks[i + 1];
    const gap = nx ? pct(nx.t, duration) - pct(x.t, duration) - 0.3 : 100 - pct(x.t, duration);
    const w = x.kind === "type" ? pct(x.end, duration) - pct(x.t, duration) : Math.min(gap, 9);
    return {
      l: pct(x.t, duration),
      w: Math.max(0.8, w),
      label: x.kind === "type" ? x.text : x.combo.join(" + "),
      past: x.t <= cur,
    };
  });
}

/** Index of the last step starting at or before `cur`, or -1. */
export function currentStepIndex(steps: Step[], cur: number): number {
  let idx = -1;
  for (let i = 0; i < steps.length; i++) if (steps[i].t <= cur) idx = i;
  return idx;
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
