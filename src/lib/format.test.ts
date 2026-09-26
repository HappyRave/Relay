import { describe, expect, it } from "vitest";
import { fmtLastRun, fmtTime, nextRunLabel, pad4, slug } from "./format";

describe("format", () => {
  it("formats playhead times", () => {
    expect(fmtTime(0)).toBe("00:00.00");
    expect(fmtTime(7350)).toBe("00:07.35");
    expect(fmtTime(61_020)).toBe("01:01.02");
    expect(fmtTime(-5)).toBe("00:00.00");
    expect(fmtTime(59_996)).toBe("01:00.00");
  });

  it("slugs export names", () => {
    expect(slug("Export invoice to PDF")).toBe("export-invoice-to-pdf");
    expect(slug("Déplacer fenêtre ")).toBe("déplacer-fenêtre");
    expect(slug("***")).toBe("");
  });

  it("pads coordinates, keeping the sign", () => {
    expect(pad4(42)).toBe("0042");
    expect(pad4(-5)).toBe("-0005");
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
    const at = (d: number, h: number, m: number) => new Date(2026, 8, d, h, m).toISOString();
    expect(nextRunLabel(at(24, 9, 0), now)).toBe("Next run: Today 09:00");
    expect(nextRunLabel(at(25, 7, 0), now)).toBe("Next run: Tomorrow 07:00");
    expect(nextRunLabel(at(28, 7, 5), now)).toBe("Next run: Mon 07:05");
    expect(nextRunLabel(null, now)).toBe("No schedule");
  });
});
