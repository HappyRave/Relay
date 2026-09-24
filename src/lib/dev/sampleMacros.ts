// Development data: the four sample macros from the design prototype
// (Design/Macro Recorder.dc.html, `buildSample`), plus the event → step grouping
// the prototype uses. From M1 the grouping lives in relay-core and this file is
// only used by the in-browser mock.

import type { MouseBtn, PlaybackOptions, Rect, Step, Triggers } from "../types";

export type MockEvent =
  | { id: number; t: number; type: "move"; x: number; y: number }
  | { id: number; t: number; type: "click"; x: number; y: number; btn: MouseBtn; count: 1 | 2; label: string }
  | { id: number; t: number; type: "key"; key: string }
  | { id: number; t: number; type: "char"; key: string }
  | { id: number; t: number; type: "wait"; dur: number; label: string }
  | { id: number; t: number; type: "cond"; dur: number; label: string; x: number; y: number; color: string };

export interface MockMacro {
  id: string;
  name: string;
  desktop: Rect;
  frames: Rect[];
  events: MockEvent[];
  runs: number;
  lastRun: string;
  playback: PlaybackOptions;
  triggers: Triggers;
}

let nextId = 1;
export const newEventId = () => nextId++;

type Op =
  | ["m", number, number, number, number]
  | ["c", string, ("Left" | "Right" | "Double")?]
  | ["k", string, number?]
  | ["t", string]
  | ["w", number, string]
  | ["p", number, string];

const ease = (u: number) => (u < 0.5 ? 2 * u * u : 1 - Math.pow(-2 * u + 2, 2) / 2);

/** Port of the prototype's `buildSample`, producing virtual-desktop pixels on `desktop`. */
export function buildSample(script: Op[], desktop: Rect = SAMPLE_DESKTOP): MockEvent[] {
  const W = desktop.w;
  const H = desktop.h;
  const px = (x: number) => Math.round(desktop.x + x * W);
  const py = (y: number) => Math.round(desktop.y + y * H);
  let p = { x: 0.5, y: 0.62 };
  let t = 0;
  const ev: MockEvent[] = [{ id: newEventId(), t: 0, type: "move", x: px(p.x), y: py(p.y) }];

  const move = (x: number, y: number, dur: number, bend: number) => {
    const a = p;
    const b = { x, y };
    const mx = (a.x + b.x) / 2;
    const my = (a.y + b.y) / 2;
    const cx = mx - (b.y - a.y) * bend;
    const cy = my + (b.x - a.x) * bend;
    const n = Math.round(dur / 16);
    for (let i = 1; i <= n; i++) {
      const u = ease(i / n);
      const v = 1 - u;
      ev.push({
        id: newEventId(),
        t: Math.round(t + (i * dur) / n),
        type: "move",
        x: px(v * v * a.x + 2 * v * u * cx + u * u * b.x),
        y: py(v * v * a.y + 2 * v * u * cy + u * u * b.y),
      });
    }
    t += dur;
    p = b;
  };
  const click = (label: string, btn: "Left" | "Right" | "Double" = "Left", gap = 250) => {
    ev.push({
      id: newEventId(),
      t,
      type: "click",
      x: px(p.x),
      y: py(p.y),
      btn: btn === "Double" ? "Left" : btn,
      count: btn === "Double" ? 2 : 1,
      label,
    });
    t += gap;
  };
  const key = (k: string, gap = 300) => {
    ev.push({ id: newEventId(), t, type: "key", key: k });
    t += gap;
  };
  const type = (s: string) => {
    for (const ch of s) {
      ev.push({ id: newEventId(), t, type: "char", key: ch });
      t += 85;
    }
    t += 200;
  };
  const wait = (ms: number, label: string) => {
    ev.push({ id: newEventId(), t, type: "wait", dur: ms, label });
    t += ms;
  };
  const cond = (ms: number, label: string) => {
    ev.push({ id: newEventId(), t, type: "cond", dur: ms, label, x: px(p.x), y: py(p.y), color: "#9B9797" });
    t += ms;
  };

  for (const op of script) {
    switch (op[0]) {
      case "m": move(op[1], op[2], op[3], op[4]); break;
      case "c": click(op[1], op[2]); break;
      case "k": key(op[1], op[2]); break;
      case "t": type(op[1]); break;
      case "w": wait(op[1], op[2]); break;
      case "p": cond(op[1], op[2]); break;
    }
  }
  return ev;
}

/** Groups raw events into editor steps (the prototype's `steps()`). */
export function deriveSteps(events: MockEvent[]): Step[] {
  const out: Step[] = [];
  for (const e of events) {
    if (e.type === "move") continue;
    const last = out[out.length - 1];
    if (e.type === "char" && last && last.kind === "type" && e.t - last.end < 500) {
      last.text += e.key;
      last.chars.push({ t: e.t, ch: e.key });
      last.end = e.t + 85;
      last.items.push(e.id);
      continue;
    }
    switch (e.type) {
      case "char":
        out.push({ kind: "type", t: e.t, end: e.t + 85, text: e.key, chars: [{ t: e.t, ch: e.key }], items: [e.id] });
        break;
      case "click":
        out.push({ kind: "click", t: e.t, end: e.t + 350, x: e.x, y: e.y, btn: e.btn, count: e.count, label: e.label, items: [e.id] });
        break;
      case "key":
        out.push({ kind: "keys", t: e.t, end: e.t + 350, combo: e.key, items: [e.id] });
        break;
      case "wait":
        out.push({ kind: "wait", t: e.t, end: e.t + e.dur, dur: e.dur, label: e.label, items: [e.id] });
        break;
      case "cond":
        out.push({ kind: "pixel", t: e.t, end: e.t + e.dur, dur: e.dur, x: e.x, y: e.y, color: e.color, label: e.label, timeoutMs: 5000, items: [e.id] });
        break;
    }
  }
  return out;
}

/** The prototype's duration rule: last event end + a 500 ms tail. */
export function eventsDuration(events: MockEvent[]): number {
  const l = events[events.length - 1];
  if (!l) return 2000;
  const dur = l.type === "wait" || l.type === "cond" ? l.dur : 0;
  return l.t + dur + 500;
}

export const SAMPLE_DESKTOP: Rect = { x: 0, y: 0, w: 1920, h: 1080 };

/** The prototype's wireframe "desktop", scaled from its 1600×900 viewBox to 1920×1080. */
export const SAMPLE_FRAMES: Rect[] = [
  [40, 30, 1180, 780],
  [40, 30, 1180, 60],
  [40, 90, 280, 420],
  [560, 300, 620, 360],
  [560, 300, 620, 50],
  [680, 400, 420, 50],
  [960, 540, 180, 60],
  [1280, 80, 280, 340],
  [-10, 860, 1620, 60],
].map(([x, y, w, h]) => ({ x: x * 1.2, y: y * 1.2, w: w * 1.2, h: h * 1.2 }));

export const defaultPlayback = (): PlaybackOptions => ({
  speed: 1,
  loops: 3,
  infinite: false,
  humanize: true,
  jitterMs: 40,
  stopOnKey: true,
  coordMode: "screen",
});

export const defaultTriggers = (combo = "—"): Triggers => ({
  hotkey: { enabled: combo !== "—", combo },
  schedule: { enabled: false, days: [true, true, true, true, true, false, false], time: "09:00" },
  appLaunch: { enabled: false, exe: "EXCEL.EXE" },
  pixel: { enabled: false, x: 1210, y: 612, color: "#EC3013" },
});

export function sampleLibrary(): MockMacro[] {
  const mk = (id: string, name: string, script: Op[], hotkey: string, lastRun: string, runs: number): MockMacro => ({
    id,
    name,
    desktop: SAMPLE_DESKTOP,
    frames: SAMPLE_FRAMES,
    events: buildSample(script),
    runs,
    lastRun,
    playback: defaultPlayback(),
    triggers: defaultTriggers(hotkey),
  });
  const lib = [
    mk("inv", "Export invoice to PDF", [["m", .07, .065, 850, .25], ["c", "File menu"], ["m", .12, .3, 650, -.3], ["c", "Export as PDF"], ["w", 700, "Dialog opens"], ["m", .55, .47, 800, .2], ["c", "Filename field", "Double"], ["k", "Ctrl + A"], ["t", "invoice_0924"], ["m", .65, .63, 750, -.2], ["c", "Save"], ["p", 900, "Save button turns grey"], ["k", "Ctrl + W", 400], ["m", .87, .16, 1100, .25], ["c", "Invoices folder", "Right"], ["m", .9, .3, 420, -.15], ["c", "Upload to portal"], ["k", "Enter", 500]], "Ctrl + Alt + 1", "Today, 09:12", 148),
    mk("ts", "Fill weekly timesheet", [["m", .3, .2, 700, .2], ["c", "Mon cell"], ["t", "8"], ["k", "Tab", 150], ["t", "8"], ["k", "Tab", 150], ["t", "8"], ["k", "Tab", 150], ["t", "7.5"], ["k", "Tab", 150], ["t", "6"], ["m", .62, .74, 900, -.2], ["c", "Submit"], ["w", 800, "Confirmation"], ["m", .52, .55, 500, .1], ["c", "OK"]], "Ctrl + Alt + 2", "Fri, 17:40", 36),
    mk("ren", "Batch rename photos", [["m", .2, .35, 600, .2], ["c", "First photo"], ["k", "Ctrl + A"], ["k", "F2"], ["t", "trip_2026"], ["k", "Enter"], ["p", 1200, "Explorer refresh"], ["m", .8, .1, 900, -.2], ["c", "Sort by date"], ["m", .82, .2, 300, .1], ["c", "Date taken"]], "—", "Sep 12", 4),
    mk("std", "Open standup tools", [["k", "Win + R"], ["t", "teams"], ["k", "Enter"], ["w", 1500, "Teams loads"], ["m", .1, .4, 700, .2], ["c", "Calendar"], ["m", .45, .3, 700, -.2], ["c", "Standup", "Double"], ["k", "Ctrl + Shift + M"]], "Ctrl + Alt + 4", "Today, 08:58", 212),
  ];
  lib[0].triggers.schedule.enabled = true;
  return lib;
}
