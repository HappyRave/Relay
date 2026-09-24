import { describe, expect, it } from "vitest";
import { fmtLastRun, fmtTime, nextRunLabel, slug } from "./format";

describe("format", () => {
  it("formats playhead times", () => {
    expect(fmtTime(0)).toBe("00:00.00");
    expect(fmtTime(7350)).toBe("00:07.35");
    expect(fmtTime(61_020)).toBe("01:01.02");
    expect(fmtTime(-5)).toBe("00:00.00");
  });

  it("slugs export names", () => {
    expect(slug("Export invoice to PDF")).toBe("export-invoice-to-pdf");
  });

  it("formats the last run relative to today", () => {
    const now = new Date(2026, 8, 24, 12, 0); // Thursday
    expect(fmtLastRun(null, now)).toBe("Never");
    expect(fmtLastRun(new Date(2026, 8, 24, 9, 12).toISOString(), now)).toMatch(/^Today, /);
    expect(fmtLastRun(new Date(2026, 8, 19, 17, 40).toISOString(), now)).toMatch(/, /);
    expect(fmtLastRun(new Date(2026, 8, 12, 8, 0).toISOString(), now)).not.toMatch(/Today|,/);
  });

  it("labels the next scheduled run", () => {
    const now = new Date(2026, 8, 24, 8, 0); // Thursday 08:00
    const weekdays = [true, true, true, true, true, false, false];
    expect(nextRunLabel(true, weekdays, "09:00", now)).toBe("Next run: Today 09:00");
    expect(nextRunLabel(true, weekdays, "07:00", now)).toBe("Next run: Tomorrow 07:00");
    expect(nextRunLabel(true, [true, false, false, false, false, false, false], "07:00", now)).toBe("Next run: Mon 07:00");
    expect(nextRunLabel(false, weekdays, "09:00", now)).toBe("No schedule");
  });
});
