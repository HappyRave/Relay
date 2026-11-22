import { describe, expect, test } from "vitest";
import { alsoText, CLEARED, daysText, hasTrigger, triggerNote, triggersSummary, triggerText } from "./triggers";
import type { MacroTriggers } from "../types";

const with_ = (patch: Partial<MacroTriggers>): MacroTriggers => ({ ...structuredClone(CLEARED), ...patch });

describe("triggers in words", () => {
  test("a new macro has none", () => {
    for (const k of ["hotkey", "schedule", "interval", "app_launch", "pixel", "image"] as const) expect(hasTrigger(CLEARED, k)).toBe(false);
  });

  test("a trigger is the macro's once it's on or set, even when off", () => {
    expect(hasTrigger(with_({ hotkey: { enabled: false, combo: "Ctrl + Alt + 1" } }), "hotkey")).toBe(true);
    expect(hasTrigger(with_({ schedule: { enabled: true, schedule: CLEARED.schedule.schedule } }), "schedule")).toBe(true);
    expect(hasTrigger(with_({ schedule: { enabled: false, schedule: { days: CLEARED.schedule.schedule.days, time: "17:00" } } }), "schedule")).toBe(true);
    expect(hasTrigger(with_({ interval: { enabled: false, every: 10, unit: "s" } }), "interval")).toBe(true);
    expect(hasTrigger(with_({ interval: { enabled: false, every: 3, unit: "min" } }), "interval")).toBe(true);
    expect(hasTrigger(with_({ app_launch: { enabled: false, exe: "EXCEL.EXE", delay_ms: 2000 } }), "app_launch")).toBe(true);
    expect(hasTrigger(with_({ pixel: { ...CLEARED.pixel, x: 5 } }), "pixel")).toBe(true);
    expect(hasTrigger(with_({ image: { ...CLEARED.image, image: "iVBOR" } }), "image")).toBe(true);
  });

  test("days", () => {
    expect(daysText([true, true, true, true, true, false, false])).toBe("Mon–Fri");
    expect(daysText([true, true, true, true, true, true, true])).toBe("Every day");
    expect(daysText([true, false, true, false, true, false, false])).toBe("Mon, Wed, Fri");
    expect(daysText([false, false, false, false, false, true, true])).toBe("Sat, Sun");
  });

  test("what each does", () => {
    const t = with_({
      hotkey: { enabled: true, combo: "Ctrl + Alt + 1" },
      app_launch: { enabled: true, exe: "EXCEL.EXE", delay_ms: 1500 },
      pixel: { enabled: true, x: 12, y: 34, color: "#00FF00", tolerance: 8 },
    });
    expect(triggerText("hotkey", t)).toBe("Ctrl + Alt + 1");
    expect(triggerText("schedule", t)).toBe("Mon–Fri at 09:00");
    expect(triggerText("app_launch", t)).toBe("1.5 s after EXCEL.EXE starts");
    expect(triggerText("pixel", t)).toBe("When 12, 34 turns #00FF00");
    expect(triggerText("image", t)).toBe("No image yet");
    expect(triggerText("interval", t)).toBe("Every 10 min");
    expect(triggersSummary(t)).toBe("Ctrl + Alt + 1 · When an app starts · When a pixel changes");
    expect(triggersSummary({ ...t, interval: { enabled: true, every: 90, unit: "s" } })).toBe("Ctrl + Alt + 1 · Every 90 s · When an app starts · When a pixel changes");
    expect(triggersSummary(CLEARED)).toBe("Only when you play it");
  });

  test("what else runs it, as the Library says", () => {
    expect(alsoText([{ kind: "interval", every: 2, unit: "h" }, { kind: "pixel" }])).toBe("Every 2 h · When a pixel changes");
  });

  test("the line under a trigger's name", () => {
    const status = { triggers: with_({ schedule: { enabled: true, schedule: { days: [false, false, false, false, false, false, false], time: "09:00" } } }), next_run: null, hotkey_error: null, paused: false };
    expect(triggerNote("hotkey", status)).toBe("Runs from any app");
    expect(triggerNote("hotkey", { ...status, hotkey_error: "Ctrl + C is used by Windows" })).toBe("Ctrl + C is used by Windows");
    expect(triggerNote("schedule", status)).toBe("Pick a day");
    expect(triggerNote("pixel", status)).toBe("Off");
    expect(triggerNote("interval", status)).toBe("Off");
    expect(triggerNote("interval", { ...status, triggers: with_({ interval: { enabled: true, every: 5, unit: "s" } }) })).toBe("On");
  });
});
