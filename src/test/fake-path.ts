// relay-core's path reshaping (crates/relay-core/src/path.rs) for the fake
// core, step for step, so it lands on the same pixels: fake-core.test.ts
// checks it against Rust's answers in path-cases.json.

export type Point = [number, number];

/** How far a point may be from the simplified path before it counts as a corner. */
const SMOOTH_TOLERANCE_PX = 8;

const dist = (a: Point, b: Point) => {
  const dx = a[0] - b[0];
  const dy = a[1] - b[1];
  return Math.sqrt(dx * dx + dy * dy);
};

/** Rust's `f64::round`: halves go away from zero. */
const round = (v: number) => Math.sign(v) * Math.round(Math.abs(v)) + 0;

function offSegment(p: Point, a: Point, b: Point): number {
  const dx = b[0] - a[0];
  const dy = b[1] - a[1];
  const len2 = dx * dx + dy * dy;
  if (len2 === 0) return dist(p, a);
  const k = Math.min(1, Math.max(0, ((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / len2));
  return dist(p, [a[0] + k * dx, a[1] + k * dy]);
}

/** Ramer–Douglas–Peucker, keeping both ends. */
export function simplify(points: Point[], eps: number): Point[] {
  if (points.length < 3) return points.slice();
  const keep = points.map((_, i) => i === 0 || i === points.length - 1);
  const spans: [number, number][] = [[0, points.length - 1]];
  while (spans.length) {
    const [a, b] = spans.pop()!;
    let far = -1;
    let best = -Infinity;
    for (let i = a + 1; i < b; i++) {
      const d = offSegment(points[i], points[a], points[b]);
      // Like Rust's `max_by`: the last of equal ones.
      if (d >= best) [far, best] = [i, d];
    }
    if (far >= 0 && best > eps) {
      keep[far] = true;
      spans.push([a, far], [far, b]);
    }
  }
  return points.filter((_, i) => keep[i]);
}

function chaikin(poly: Point[]): Point[] {
  if (poly.length < 3) return poly.slice();
  const out: Point[] = [poly[0]];
  for (let i = 0; i + 1 < poly.length; i++) {
    const [a, b] = [poly[i], poly[i + 1]];
    out.push([0.75 * a[0] + 0.25 * b[0], 0.75 * a[1] + 0.25 * b[1]]);
    out.push([0.25 * a[0] + 0.75 * b[0], 0.25 * a[1] + 0.75 * b[1]]);
  }
  out.splice(1, 1);
  out.pop();
  out.push(poly[poly.length - 1]);
  return out;
}

function resample(points: Point[], target: Point[]): Point[] {
  const along = [0];
  for (let i = 1; i < points.length; i++) along.push(along[i - 1] + dist(points[i - 1], points[i]));
  const total = along[along.length - 1];
  const seg = [0];
  for (let i = 1; i < target.length; i++) seg.push(seg[i - 1] + dist(target[i - 1], target[i]));
  const targetLen = seg[seg.length - 1];
  const last = points.length - 1;
  let j = 0;
  return along.map((d, i) => {
    if (i === 0) return points[0];
    if (i === last) return points[last];
    const want = total === 0 ? 0 : (d / total) * targetLen;
    while (j + 2 < seg.length && seg[j + 1] < want) j++;
    const [a, b] = [target[j], target[j + 1]];
    const span = seg[j + 1] - seg[j];
    const k = span === 0 ? 0 : Math.min(1, Math.max(0, (want - seg[j]) / span));
    return [round(a[0] + k * (b[0] - a[0])), round(a[1] + k * (b[1] - a[1]))];
  });
}

/** The path with the wobble taken out. */
export function smooth(points: Point[]): Point[] {
  if (points.length < 3) return points.slice();
  return resample(points, chaikin(simplify(points, SMOOTH_TOLERANCE_PX)));
}

/** The path on the straight line from its first point to its last. */
export function straighten(points: Point[]): Point[] {
  if (points.length < 3) return points.slice();
  return resample(points, [points[0], points[points.length - 1]]);
}
