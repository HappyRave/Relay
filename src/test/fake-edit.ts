// relay-core's edits (crates/relay-core/src/edit.rs) for the fake core, on
// the view the UI receives: steps, the cursor path and the duration. The
// fake has no raw events, but it keeps what the UI can see of them the way
// relay-core would: `items` are event indices (cursor moves count, and each
// belongs to a step: a MOVE, or the click or drag it happened during), so an
// insertion or a deletion renumbers the steps after it; inserts snap past the
// step under the playhead and push what follows back; deleting a wait closes
// its gap; pauses and moves retime what follows; smoothing a move reshapes its
// samples (fake-path.ts). What regrouping would do (two clicks merging into a
// double click once the step between them is deleted, two moves into one)
// isn't modelled: that's tested in Rust.
import type { EditOp, MacroView, MovePoint, Step, StepOf } from "../lib/types";
import type { IpcError } from "../lib/ipc/backend";
import { smooth, straighten, type Point } from "./fake-path";

/** The longest wait or pause an edit may set (a day). */
const MAX_DUR = 24 * 60 * 60 * 1000;
/** Silence kept after the last event; the length of an empty macro. */
const TAIL_MS = 500;
const MIN_DURATION_MS = 2000;

const rejected = (message: string): IpcError => ({ code: "edit_rejected", message });
const clampMs = (t: number) => Math.max(0, t);

function shiftStep(s: Step, delta: number) {
  s.t = clampMs(s.t + delta);
  s.end = clampMs(s.end + delta);
  if (s.kind === "type") for (const c of s.chars) c.t = clampMs(c.t + delta);
}

/** Sorts the steps and recomputes what relay-core derives: pauses and the duration. */
export function summarize(v: MacroView) {
  v.steps.sort((a, b) => a.t - b.t);
  let busy = 0;
  for (const s of v.steps) {
    s.items.sort((a, b) => a - b);
    s.pause = Math.max(0, s.t - busy);
    busy = Math.max(busy, s.end);
  }
  const ends = [...v.steps.map((s) => s.end), ...v.moves.map((m) => m.t)];
  v.duration = ends.length ? Math.max(...ends) + TAIL_MS : MIN_DURATION_MS;
}

/** Moves `at` just past the step it falls on (from its start to its end). */
function snap(steps: Step[], at: number): number {
  for (let s = steps.find((s) => s.t <= at && at <= s.end); s; s = steps.find((s) => s.t <= at && at <= s.end)) {
    at = s.end + 1;
  }
  return at;
}

/** Adds `delta` to every step and cursor sample at or after `from`. */
function shiftFrom(v: MacroView, from: number, delta: number) {
  for (const s of v.steps) if (s.t >= from) shiftStep(s, delta);
  for (const m of v.moves) if (m.t >= from) m.t = clampMs(m.t + delta);
}

function insertTimed(v: MacroView, at: number, dur: number, make: (t: number, item: number) => Step) {
  at = snap(v.steps, at);
  // Where the new event goes in the event list: after everything earlier. A
  // step is either all before `at` or all after it (that's what snapping is for).
  const pos = v.steps.filter((s) => s.end < at).reduce((n, s) => n + s.items.length, 0);
  shiftFrom(v, at, dur);
  for (const s of v.steps) s.items = s.items.map((i) => (i >= pos ? i + 1 : i));
  v.steps.push(make(at, pos));
}

/** The time `t` becomes once each pause `[from, to]` (sorted, apart) lasts `dur`. */
function retime(t: number, pauses: [number, number, number][]): number {
  let shift = 0;
  for (const [from, to, dur] of pauses) {
    if (t >= to) {
      shift += dur - (to - from);
      continue;
    }
    // Inside a pause only the cursor moves: stretched or squeezed to fit.
    if (t > from) return clampMs(from + shift + Math.floor(((t - from) * dur) / (to - from)));
    break;
  }
  return clampMs(t + shift);
}

function retimePauses(v: MacroView, pauses: [number, number, number][]) {
  if (!pauses.length) return;
  for (const s of v.steps) {
    const t = retime(s.t, pauses);
    s.end = s.kind === "wait" || s.kind === "pixel_wait" ? t + s.dur : retime(s.end, pauses);
    s.t = t;
    if (s.kind === "type") for (const c of s.chars) c.t = retime(c.t, pauses);
  }
  for (const m of v.moves) m.t = retime(m.t, pauses);
}

/** A MOVE step's samples in the cursor path: `samples` of them from its start. */
function samplesOf(v: MacroView, s: StepOf<"move">): MovePoint[] {
  const first = v.moves.findIndex((m) => m.t >= s.t);
  return first < 0 ? [] : v.moves.slice(first, first + s.samples);
}

/**
 * Removes a deleted step's part of the cursor path: a MOVE's samples, or a
 * click's or drag's presses, releases and the moves while the button was down
 * (those in its time that no MOVE step has).
 */
function dropMoves(v: MacroView, s: Step) {
  let drop: Set<MovePoint>;
  if (s.kind === "move") {
    drop = new Set(samplesOf(v, s));
  } else if (s.kind === "click" || s.kind === "drag") {
    const others = new Set(v.steps.flatMap((o) => (o.kind === "move" ? samplesOf(v, o) : [])));
    drop = new Set(v.moves.filter((m) => m.t >= s.t && m.t <= s.end && !others.has(m)));
  } else {
    return;
  }
  v.moves = v.moves.filter((m) => !drop.has(m));
}

/** Two MOVE steps with nothing between them (the step between was deleted) are one move. */
function mergeMoves(v: MacroView) {
  const moves = v.steps.filter((s): s is StepOf<"move"> => s.kind === "move").sort((a, b) => a.items[0] - b.items[0]);
  for (let i = moves.length - 1; i > 0; i--) {
    const [a, b] = [moves[i - 1], moves[i]];
    if (a.items[a.items.length - 1] + 1 !== b.items[0]) continue;
    Object.assign(a, { end: b.end, to_x: b.to_x, to_y: b.to_y, samples: a.samples + b.samples, items: [...a.items, ...b.items] });
    v.steps.splice(v.steps.indexOf(b), 1);
  }
}

/** Moves a MOVE step's samples onto the path `f` makes of them, from where the cursor was before. */
function reshape(v: MacroView, s: StepOf<"move">, f: (p: Point[]) => Point[]) {
  const samples = samplesOf(v, s);
  const out = f([[s.x, s.y], ...samples.map((m): Point => [m.x, m.y])]);
  samples.forEach((m, i) => ([m.x, m.y] = out[i + 1]));
}

/** Applies `op` to `v` like `relay_core::edit::apply`, or throws its error. */
export function applyEdit(v: MacroView, op: EditOp) {
  const get = (index: number): Step => {
    const s = v.steps[index];
    if (!s) throw rejected(`there is no step ${index}`);
    return s;
  };
  const wrongKind = (index: number) => rejected(`step ${index} can't be edited this way`);

  switch (op.op) {
    case "rename":
      v.name = op.name;
      return;

    case "delete_step": {
      const s = get(op.index);
      v.steps.splice(op.index, 1);
      for (const o of v.steps) o.items = o.items.map((i) => i - s.items.filter((g) => g < i).length);
      dropMoves(v, s);
      mergeMoves(v);
      // Deleting a wait closes the gap it left.
      if (s.kind === "wait" || s.kind === "pixel_wait") shiftFrom(v, s.end, -s.dur);
      break;
    }

    case "insert_wait": {
      const dur = Math.min(op.dur, MAX_DUR);
      insertTimed(v, op.at, dur, (t, item) => ({ kind: "wait", t, end: t + dur, pause: 0, items: [item], dur, label: op.label }));
      break;
    }

    case "insert_pixel_wait": {
      const dur = Math.min(op.dur, MAX_DUR);
      const { x, y, color, tolerance, timeout_ms, label } = op;
      insertTimed(v, op.at, dur, (t, item) => ({
        kind: "pixel_wait",
        t,
        end: t + dur,
        pause: 0,
        items: [item],
        dur,
        x,
        y,
        color: color.toUpperCase(),
        tolerance,
        timeout_ms,
        label,
      }));
      break;
    }

    case "set_wait_duration": {
      const s = get(op.index);
      if (s.kind !== "wait" && s.kind !== "pixel_wait") throw wrongKind(op.index);
      const dur = Math.min(op.dur, MAX_DUR);
      const delta = dur - s.dur;
      // Nothing happens during a wait, so what comes after it in the list is
      // exactly what starts at or after its end.
      for (const o of v.steps) if (o !== s && o.items[0] > s.items[0]) shiftStep(o, delta);
      for (const m of v.moves) if (s.dur > 0 ? m.t >= s.end : m.t > s.t) m.t = clampMs(m.t + delta);
      s.dur = dur;
      s.end = s.t + dur;
      break;
    }

    case "update_pixel_wait": {
      const s = get(op.index);
      if (s.kind !== "pixel_wait") throw wrongKind(op.index);
      Object.assign(s, { x: op.x, y: op.y, color: op.color.toUpperCase(), tolerance: op.tolerance, timeout_ms: op.timeout_ms });
      break;
    }

    case "set_label": {
      const s = get(op.index);
      if (s.kind !== "click" && s.kind !== "drag" && s.kind !== "wait" && s.kind !== "pixel_wait") throw wrongKind(op.index);
      s.label = op.label;
      break;
    }

    case "set_pause": {
      const s = get(op.index);
      const dur = Math.min(op.dur, MAX_DUR);
      if (s.pause > 0) {
        retimePauses(v, [[s.t - s.pause, s.t, dur]]);
        break;
      }
      // No gap to stretch: the step and what follows it in the list move,
      // not what ends at that moment (the last sample of the move before it).
      const staying = v.steps.filter((o) => o.items[0] < s.items[0]);
      const kept = new Set(staying.flatMap((o) => (o.kind === "move" ? samplesOf(v, o) : [])));
      for (const m of v.moves) if (m.t >= s.t && !kept.has(m)) m.t += dur;
      for (const o of v.steps) if (o.items[0] >= s.items[0]) shiftStep(o, dur);
      break;
    }

    case "cap_pauses":
      retimePauses(
        v,
        v.steps.filter((s) => s.pause > op.max).map((s) => [s.t - s.pause, s.t, op.max]),
      );
      break;

    case "set_move_duration": {
      const s = get(op.index);
      if (s.kind !== "move") throw wrongKind(op.index);
      // A single sample is a jump: it has no length to set.
      if (s.end > s.t) retimePauses(v, [[s.t, s.end, Math.min(op.dur, MAX_DUR)]]);
      break;
    }

    case "smooth_move":
    case "straighten_move": {
      const s = get(op.index);
      if (s.kind !== "move") throw wrongKind(op.index);
      reshape(v, s, op.op === "smooth_move" ? smooth : straighten);
      break;
    }
  }
  summarize(v);
}

// — history.rs —

/** Edits kept per macro. */
const LIMIT = 100;
/** Renames closer together than this are one undo step. */
const RENAME_BURST_MS = 2000;

/** What an edit can change: the name, and for all but renames, the events. */
interface Snapshot {
  name: string;
  events: Pick<MacroView, "steps" | "moves" | "duration"> | null;
}

const eventsOf = (v: MacroView) => structuredClone({ steps: v.steps, moves: v.moves, duration: v.duration });

/** One macro's undo and redo, kept for this run (playback options aren't part of it). */
export class History {
  undo: Snapshot[] = [];
  redo: Snapshot[] = [];
  private lastRename: number | null = null;

  /** The part of `v` that `op` is about to change. */
  static before(v: MacroView, op: EditOp): Snapshot {
    return { name: v.name, events: op.op === "rename" ? null : eventsOf(v) };
  }

  /** Records `before` once `op` has been applied to `v`. An edit that changed nothing isn't one. */
  record(before: Snapshot, op: EditOp, v: MacroView) {
    if (before.name === v.name && (!before.events || JSON.stringify(before.events) === JSON.stringify(eventsOf(v)))) return;
    const now = Date.now();
    const renaming = op.op === "rename";
    const sameRename = renaming && this.lastRename != null && now - this.lastRename < RENAME_BURST_MS;
    if (!sameRename) {
      this.undo.push(before);
      if (this.undo.length > LIMIT) this.undo.shift();
    }
    this.lastRename = renaming ? now : null;
    this.redo = [];
  }

  /** Reverts (or with `redo`, re-applies) one edit of `v`. False if there's none. */
  step(v: MacroView, redo: boolean): boolean {
    const [from, to] = redo ? [this.redo, this.undo] : [this.undo, this.redo];
    const s = from.pop();
    if (!s) return false;
    to.push({ name: v.name, events: s.events ? eventsOf(v) : null });
    v.name = s.name;
    if (s.events) Object.assign(v, structuredClone(s.events));
    this.lastRename = null;
    return true;
  }
}
