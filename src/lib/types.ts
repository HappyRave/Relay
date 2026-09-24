// View types shared by the UI. In M1 these are replaced by types generated
// from relay-core (ts-rs); the shapes are kept close to the planned Rust model.

export type Mode = "idle" | "count" | "rec" | "play" | "pause";
export type Tab = "events" | "lib" | "trig" | "options";
export type MouseBtn = "Left" | "Right" | "Middle";
export type CoordMode = "screen" | "window";
export type PathMode = "full" | "trail";
export type ExportFormat = "rly" | "json";

export interface Rect {
  x: number;
  y: number;
  w: number;
  h: number;
}

/** A sampled cursor position; `t` is ms from recording start, x/y are virtual-desktop px. */
export interface MovePoint {
  t: number;
  x: number;
  y: number;
}

interface StepBase {
  t: number;
  end: number;
  /** Ids of the events this step was built from (used by delete). */
  items: number[];
}

export interface ClickStep extends StepBase {
  kind: "click";
  x: number;
  y: number;
  btn: MouseBtn;
  count: 1 | 2;
  label: string;
}
export interface KeysStep extends StepBase {
  kind: "keys";
  combo: string;
}
export interface TypeStep extends StepBase {
  kind: "type";
  text: string;
  chars: { t: number; ch: string }[];
}
export interface WaitStep extends StepBase {
  kind: "wait";
  dur: number;
  label: string;
}
export interface PixelStep extends StepBase {
  kind: "pixel";
  dur: number;
  x: number;
  y: number;
  color: string;
  label: string;
  timeoutMs: number;
}
export type Step = ClickStep | KeysStep | TypeStep | WaitStep | PixelStep;

export interface MacroView {
  id: string;
  name: string;
  /** The virtual desktop the macro was recorded on. */
  desktop: Rect;
  /** Outlines drawn in the preview (monitors, anchor window, …). */
  frames: Rect[];
  moves: MovePoint[];
  steps: Step[];
  duration: number;
}

export interface PlaybackOptions {
  speed: number;
  loops: number;
  infinite: boolean;
  humanize: boolean;
  jitterMs: number;
  stopOnKey: boolean;
  coordMode: CoordMode;
}

export interface Triggers {
  hotkey: { enabled: boolean; combo: string };
  schedule: { enabled: boolean; days: boolean[]; time: string };
  appLaunch: { enabled: boolean; exe: string };
  pixel: { enabled: boolean; x: number; y: number; color: string };
}

export interface Settings {
  captureMoves: boolean;
  captureKeys: boolean;
  countdown: boolean;
  pathMode: PathMode;
  showClickLabels: boolean;
}
