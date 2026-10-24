// The numbered callouts beside the clicks on the preview: next to their
// click, nudged apart so none covers another, and kept inside the stage.

/** Where a callout's circle goes (stage px), and whether it sits left of its click. */
export type Callout = { x: number; y: number; bx: number; by: number; left: boolean };

/** A callout's circle, and the gap kept between two. */
const SIZE = 20;
const APART = 24;
const OFFSET = 18;

export function placeCallouts(points: { x: number; y: number }[], w: number, h: number): Callout[] {
  const inside = (v: number, max: number) => Math.min(max - SIZE / 2 - 2, Math.max(SIZE / 2 + 2, v));
  const placed: Callout[] = [];
  points.forEach(({ x, y }, i) => {
    const left = x > w * 0.65;
    let bx = inside(x + (left ? -OFFSET : OFFSET), w);
    let by = inside(y + (i % 2 ? OFFSET : -OFFSET), h);
    // Away from the edge it's nearest, so the nudges have room.
    const dir = by > h / 2 ? -1 : 1;
    for (let tries = 0; tries < 12 && placed.some((o) => Math.abs(o.bx - bx) < APART && Math.abs(o.by - by) < APART); tries++) {
      const next = by + dir * APART;
      if (next < SIZE / 2 || next > h - SIZE / 2) bx = inside(bx + (left ? -APART : APART), w);
      else by = next;
    }
    placed.push({ x, y, bx, by, left });
  });
  return placed;
}
