// A macro's triggers in words: which it has, what each does, and the
// one-line summary the Macro tab's Runs automatically shows when closed.
import type { MacroTriggers, TriggerStatus } from "../types";
import { nextRunLabel } from "../format";
import type { TriggerOn } from "../ipc/bindings/TriggerOn";

export type TriggerKind = keyof MacroTriggers;
export const TRIGGER_KINDS: TriggerKind[] = ["hotkey", "schedule", "app_launch", "pixel", "image"];
export const TRIGGER_NAME: Record<TriggerKind, string> = {
  hotkey: "Hotkey",
  schedule: "Schedule",
  app_launch: "When an app starts",
  pixel: "When a pixel changes",
  image: "When an image appears",
};

/** A trigger as a new macro has it: none of them set. */
export const CLEARED: MacroTriggers = {
  hotkey: { enabled: false, combo: "" },
  schedule: { enabled: false, schedule: { days: [true, true, true, true, true, false, false], time: "09:00" } },
  app_launch: { enabled: false, exe: "", delay_ms: 2000 },
  pixel: { enabled: false, x: 0, y: 0, color: "#EC3013", tolerance: 8 },
  image: { enabled: false, image: null, threshold: 85, area: null },
};

/** Whether the macro has this trigger: it's on, or set to something. */
export function hasTrigger(t: MacroTriggers, kind: TriggerKind): boolean {
  switch (kind) {
    case "hotkey":
      return t.hotkey.combo !== "";
    case "schedule":
      return t.schedule.enabled || JSON.stringify(t.schedule.schedule) !== JSON.stringify(CLEARED.schedule.schedule);
    case "app_launch":
      return t.app_launch.exe.trim() !== "";
    case "pixel": {
      const { x, y, color } = t.pixel;
      return t.pixel.enabled || x !== CLEARED.pixel.x || y !== CLEARED.pixel.y || color !== CLEARED.pixel.color;
    }
    case "image":
      return !!t.image.image;
  }
}

const DAY = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
export const DAY_NAMES = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"];

/** "Mon–Fri", "Every day", "Mon, Wed, Fri", "Sat–Sun". */
export function daysText(days: readonly boolean[]): string {
  const on = days.flatMap((d, i) => (d ? [i] : []));
  if (on.length === 7) return "Every day";
  const run = on.length > 2 && on.every((d, i) => i === 0 || d === on[i - 1] + 1);
  return run ? `${DAY[on[0]]}–${DAY[on.at(-1)!]}` : on.map((i) => DAY[i]).join(", ");
}

/** What a trigger does, as its card and the Library say it. */
export function triggerText(kind: TriggerKind, t: MacroTriggers): string {
  switch (kind) {
    case "hotkey":
      return t.hotkey.combo || "No keys yet";
    case "schedule":
      return t.schedule.schedule.days.some(Boolean) ? `${daysText(t.schedule.schedule.days)} at ${t.schedule.schedule.time}` : "No days yet";
    case "app_launch":
      return t.app_launch.exe ? `${t.app_launch.delay_ms / 1000} s after ${t.app_launch.exe} starts` : "No program yet";
    case "pixel":
      return `When ${t.pixel.x}, ${t.pixel.y} turns ${t.pixel.color}`;
    case "image":
      return t.image.image ? `When the image is on screen, at a ${t.image.threshold}% match` : "No image yet";
  }
}

/** The line under a trigger's name: what's wrong with it, or when it runs next. */
export function triggerNote(kind: TriggerKind, status: TriggerStatus): string {
  const t = status.triggers;
  if (kind === "hotkey") return status.hotkey_error ?? "Runs from any app";
  if (kind === "schedule") return t.schedule.enabled ? (t.schedule.schedule.days.some(Boolean) ? nextRunLabel(status.next_run) : "Pick a day") : "Off";
  return t[kind].enabled ? "On" : "Off";
}

/** A Library row's "Also runs": what else runs the macro, in words. */
export function alsoText(also: TriggerOn[]): string {
  return also
    .map((t) =>
      t.kind === "schedule"
        ? `${daysText(t.schedule.days)} at ${t.schedule.time}`
        : t.kind === "app_launch"
          ? `When ${t.exe} starts`
          : t.kind === "pixel"
            ? "When a pixel changes"
            : "When an image appears",
    )
    .join(" · ");
}

/** Runs automatically, in one line: the triggers that are on. */
export function triggersSummary(t: MacroTriggers): string {
  const on = TRIGGER_KINDS.filter((k) => t[k].enabled && hasTrigger(t, k));
  return on.length ? on.map((k) => (k === "hotkey" || k === "schedule" ? triggerText(k, t) : TRIGGER_NAME[k])).join(" · ") : "Only when you play it";
}
