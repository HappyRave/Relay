import { describe, expect, it } from "vitest";
import type { RunEntry } from "./ipc/bindings/RunEntry";
import type { CheckResult } from "./ipc/bindings/CheckResult";
import { checkLabel, failed, fmtDuration, fmtRunTime, outcomeLabel, runSettings, sourceLabel } from "./runs";

const entry = (over: Partial<RunEntry> = {}): RunEntry => ({
  at: "2026-09-24T09:12:00Z",
  macro_id: "m1",
  macro_name: "Export invoice",
  source: "manual",
  outcome: { type: "finished", reason: "completed" },
  duration_ms: 1200,
  from_ms: 0,
  loops: 1,
  speed: 1,
  humanize: false,
  checks: [],
  checks_dropped: 0,
  ...over,
});

const check = (over: Partial<CheckResult> = {}): CheckResult => ({
  step: 4,
  loop_idx: 0,
  image: false,
  after_ms: 1200,
  outcome: { type: "matched" },
  ...over,
});

describe("run history labels", () => {
  it("names every source", () => {
    expect(["manual", "hotkey", "schedule", "app_launch", "pixel", "image"].map((s) => sourceLabel(s as never))).toEqual([
      "Play",
      "Hotkey",
      "Schedule",
      "App launch",
      "Pixel trigger",
      "Image trigger",
    ]);
  });

  it("says how a run ended, and why a trigger was skipped", () => {
    const ended = (reason: string, checks: CheckResult[] = []) =>
      outcomeLabel(entry({ outcome: { type: "finished", reason: reason as never }, checks }));
    expect(ended("completed")).toBe("Completed");
    expect(ended("stopped")).toBe("Stopped");
    expect(ended("key_pressed")).toBe("Stopped by a key");
    expect(ended("killed")).toBe("Kill switch");
    expect(ended("error")).toBe("Failed");
    expect(ended("pixel_timeout", [check({ outcome: { type: "timed_out" } })])).toBe("Pixel check timed out");
    expect(ended("pixel_timeout", [check({ image: true, outcome: { type: "timed_out" } })])).toBe("Image not found");
    const skipped = (reason: string) => outcomeLabel(entry({ outcome: { type: "skipped", reason: reason as never } }));
    expect(skipped("busy")).toBe("Skipped: Relay was busy");
    expect(skipped("locked")).toBe("Skipped: screen locked");
    expect(skipped("missed")).toBe("Skipped: PC was asleep");
    expect(skipped("data_file")).toBe("Skipped: data file");
  });

  it("marks skips and failures, not completed or stopped runs", () => {
    expect(failed(entry())).toBe(false);
    expect(failed(entry({ outcome: { type: "finished", reason: "stopped" } }))).toBe(false);
    expect(failed(entry({ outcome: { type: "finished", reason: "pixel_timeout" } }))).toBe(true);
    expect(failed(entry({ outcome: { type: "skipped", reason: "busy" } }))).toBe(true);
  });

  it("formats durations", () => {
    expect(fmtDuration(0)).toBe("0.0 s");
    expect(fmtDuration(12_340)).toBe("12.3 s");
    expect(fmtDuration(125_000)).toBe("2 min 05 s");
    expect(fmtDuration(4_800_000)).toBe("1 h 20 min");
  });

  it("dates a run with its time, also days ago", () => {
    const now = new Date(2026, 8, 24, 12, 0);
    expect(fmtRunTime(new Date(2026, 8, 24, 9, 12).toISOString(), now)).toBe("Today, 09:12");
    expect(fmtRunTime(new Date(2026, 8, 18, 17, 40).toISOString(), now)).toBe("Fri, 17:40");
    expect(fmtRunTime(new Date(2026, 8, 12, 8, 5).toISOString(), now)).toBe("Sep 12, 08:05");
  });

  it("describes each check, with its loop when there were several", () => {
    expect(checkLabel(check(), 1)).toBe("Step 4 · Pixel check matched after 1.2 s");
    expect(checkLabel(check({ loop_idx: 1 }), 3)).toBe("Loop 2 · Step 4 · Pixel check matched after 1.2 s");
    expect(checkLabel(check({ after_ms: 5000, outcome: { type: "timed_out" } }), 1)).toBe(
      "Step 4 · Pixel check timed out after 5.0 s",
    );
    expect(checkLabel(check({ after_ms: 300, outcome: { type: "interrupted" } }), 1)).toBe(
      "Step 4 · Stopped during the pixel check (0.3 s)",
    );
    const image = (outcome: CheckResult["outcome"]) => checkLabel(check({ image: true, step: 7, after_ms: 300, outcome }), 1);
    expect(image({ type: "found", x: 812, y: 440, score: 94 })).toBe("Step 7 · Image found at 812, 440 (94 %) after 0.3 s");
    expect(image({ type: "timed_out" })).toBe("Step 7 · Image not found in 0.3 s");
    expect(image({ type: "interrupted" })).toBe("Step 7 · Stopped while looking for the image (0.3 s)");
  });

  it("mentions the settings a run played with, when not the defaults", () => {
    expect(runSettings(entry())).toBe("");
    expect(runSettings(entry({ from_ms: 3200, speed: 2, humanize: true }))).toBe("From 00:03.20 · 2× speed · Humanized");
    expect(runSettings(entry({ outcome: { type: "skipped", reason: "busy" }, humanize: true }))).toBe("");
    const note = "customers.csv has no column “Total”.";
    expect(runSettings(entry({ outcome: { type: "skipped", reason: "data_file" }, note }))).toBe(note);
  });
});
