// Types shared by the UI. Everything that crosses the IPC boundary is
// generated from relay-core by ts-rs (see ./ipc/bindings); the rest is UI state.

export type { CoordMode } from "./ipc/bindings/CoordMode";
export type { EditOp } from "./ipc/bindings/EditOp";
export type { MacroListItem } from "./ipc/bindings/MacroListItem";
export type { MacroView } from "./ipc/bindings/MacroView";
export type { MouseBtn } from "./ipc/bindings/MouseBtn";
export type { MovePoint } from "./ipc/bindings/MovePoint";
export type { PlaybackOptions } from "./ipc/bindings/PlaybackOptions";
export type { Rect } from "./ipc/bindings/Rect";
export type { Repeat } from "./ipc/bindings/Repeat";
export type { Step } from "./ipc/bindings/Step";

import type { Step } from "./ipc/bindings/Step";
export type StepOf<K extends Step["kind"]> = Extract<Step, { kind: K }>;

/** The UI's session mode (the engine's `Mode` arrives in M2/M3). */
export type Mode = "idle" | "count" | "rec" | "play" | "pause";
export type Tab = "events" | "lib" | "trig" | "options";
export type PathMode = "full" | "trail";
export type ExportFormat = "rly" | "json";

/** Per-macro triggers; UI-only until M7 stores them in library.json. */
export interface Triggers {
  hotkey: { enabled: boolean; combo: string };
  schedule: { enabled: boolean; days: boolean[]; time: string };
  appLaunch: { enabled: boolean; exe: string };
  pixel: { enabled: boolean; x: number; y: number; color: string };
}

/** Global settings; UI-only until M5 persists them. */
export interface Settings {
  captureMoves: boolean;
  captureKeys: boolean;
  countdown: boolean;
  pathMode: PathMode;
  showClickLabels: boolean;
}
