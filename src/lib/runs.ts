// Labels for the run history (Library → Runs).
import type { CheckResult } from "./ipc/bindings/CheckResult";
import type { RunEntry } from "./ipc/bindings/RunEntry";
import type { RunSource } from "./ipc/bindings/RunSource";
import { fmtLastRun, fmtTime } from "./format";

const SOURCES: Record<RunSource, string> = {
  manual: "Play",
  hotkey: "Hotkey",
  schedule: "Schedule",
  app_launch: "App launch",
  pixel: "Pixel trigger",
  image: "Image trigger",
};

export const sourceLabel = (s: RunSource) => SOURCES[s];

/** "Completed", "Stopped by a key", "Skipped: Relay was busy"… */
export function outcomeLabel(e: RunEntry): string {
  const o = e.outcome;
  if (o.type === "skipped") {
    return { busy: "Skipped: Relay was busy", locked: "Skipped: screen locked", missed: "Skipped: PC was asleep" }[o.reason];
  }
  switch (o.reason) {
    case "completed":
      return "Completed";
    case "stopped":
      return "Stopped";
    case "key_pressed":
      return "Stopped by a key";
    case "killed":
      return "Kill switch";
    case "pixel_timeout":
      return e.checks.at(-1)?.image ? "Image not found" : "Pixel check timed out";
    case "error":
      return "Failed";
  }
}

/** Whether the entry is a run that didn't finish as planned (shown in the accent color). */
export const failed = (e: RunEntry) =>
  e.outcome.type === "skipped" || (e.outcome.reason !== "completed" && e.outcome.reason !== "stopped");

/** "0.4 s", "12.3 s", "2 min 05 s", "1 h 20 min". */
export function fmtDuration(ms: number): string {
  if (ms < 60_000) return `${(ms / 1000).toFixed(1)} s`;
  const s = Math.round(ms / 1000);
  if (s < 3600) return `${Math.floor(s / 60)} min ${String(s % 60).padStart(2, "0")} s`;
  return `${Math.floor(s / 3600)} h ${String(Math.floor((s % 3600) / 60)).padStart(2, "0")} min`;
}

/** Like the Library's last run, with the time also for older days: "Sep 12, 14:05". */
export function fmtRunTime(iso: string, now = new Date()): string {
  const short = fmtLastRun(iso, now);
  if (short.includes(",")) return short;
  const d = new Date(iso);
  return `${short}, ${String(d.getHours()).padStart(2, "0")}:${String(d.getMinutes()).padStart(2, "0")}`;
}

/** "Step 4 · Pixel check matched after 1.2 s", with its loop when the run had several. */
export function checkLabel(c: CheckResult, loops: number): string {
  const after = fmtDuration(c.after_ms);
  const o = c.outcome;
  let what: string;
  if (c.image) {
    what = {
      matched: `Image found after ${after}`,
      found: o.type === "found" ? `Image found at ${o.x}, ${o.y} (${o.score} %) after ${after}` : "",
      timed_out: `Image not found in ${after}`,
      interrupted: `Stopped while looking for the image (${after})`,
    }[o.type];
  } else {
    what = {
      matched: `Pixel check matched after ${after}`,
      found: `Pixel check matched after ${after}`,
      timed_out: `Pixel check timed out after ${after}`,
      interrupted: `Stopped during the pixel check (${after})`,
    }[o.type];
  }
  return `${loops > 1 ? `Loop ${c.loop_idx + 1} · ` : ""}Step ${c.step} · ${what}`;
}

/** The run's settings worth mentioning: "From 00:03.20 · 2× speed · Humanized". */
export function runSettings(e: RunEntry): string {
  if (e.outcome.type === "skipped") return "";
  const parts: string[] = [];
  if (e.from_ms > 0) parts.push(`From ${fmtTime(e.from_ms)}`);
  if (e.speed !== 1) parts.push(`${e.speed}× speed`);
  if (e.humanize) parts.push("Humanized");
  return parts.join(" · ");
}
