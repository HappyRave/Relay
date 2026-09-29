import { describe, expect, test } from "vitest";
import {
  DEFAULT_PREVIEW_W,
  DEFAULT_TIMELINE_H,
  DEFAULT_TRANSPORT_H,
  MIN_PREVIEW_W,
  MIN_TIMELINE_H,
  MIN_TRANSPORT_H,
  NO_PANES,
  paneLayout,
} from "./layout";

// The default editor: 940 px inside its border; below the header, the preview
// row, the button row and the timeline share 560 px (312 + 102 + 146).
const [W, FLEX] = [940, 560];
const panes = (preview_w: number | null, transport_h: number | null, timeline_h: number | null) => ({
  preview_w,
  transport_h,
  timeline_h,
});

describe("paneLayout", () => {
  test("the design's layout by default", () => {
    expect(paneLayout(NO_PANES, W, FLEX)).toEqual({
      previewW: 600,
      previewMax: 618, // leaves the side panel its 320 px
      transportH: 102,
      transportMax: 194, // what leaves the preview row and the timeline their minimums (twice the default is 204)
      timelineH: 146,
      timelineMax: 238, // leaves the preview row its 220 px
    });
  });

  test("the user's dividers, kept so the side panel and the preview row keep their minimum", () => {
    expect(paneLayout(panes(500, 120, 200), W, FLEX)).toMatchObject({ previewW: 500, transportH: 120, timelineH: 200 });
    // The button row goes first; the timeline gets what's left above the preview row's minimum.
    expect(paneLayout(panes(9000, 9000, 9000), W, FLEX)).toMatchObject({
      previewW: 618,
      transportH: 194,
      timelineH: 560 - 220 - 194,
    });
    expect(paneLayout(panes(10, 10, 10), W, FLEX)).toMatchObject({
      previewW: MIN_PREVIEW_W,
      transportH: MIN_TRANSPORT_H,
      timelineH: MIN_TIMELINE_H,
    });
  });

  test("a bigger window gives the dividers more room; a smaller one squeezes them, without forgetting them", () => {
    const p = panes(900, 120, 400);
    expect(paneLayout(p, 1400, 900)).toMatchObject({ previewW: 900, transportH: 120, timelineH: 400, timelineMax: 560 });
    expect(paneLayout(p, W, FLEX)).toMatchObject({ previewW: 618, transportH: 120, timelineH: 220 });
    expect(p).toEqual(panes(900, 120, 400)); // the saved values stay
  });

  test("a window smaller than every minimum keeps the minimums", () => {
    expect(paneLayout(NO_PANES, 500, 300)).toEqual({
      previewW: 360,
      previewMax: 360,
      transportH: 102,
      transportMax: 102,
      timelineH: 146,
      timelineMax: 146,
    });
  });

  test("the smallest editor Rust allows (MIN_EXPANDED, 760 × 520) keeps every minimum", () => {
    // Inside its 2 px border: 756 wide; below the 44 px header and the two dividers, 468 px.
    const [w, flex] = [760 - 4, 520 - 4 - 44 - 4];
    for (const p of [NO_PANES, panes(9000, 9000, 9000)]) {
      const l = paneLayout(p, w, flex);
      expect(l.previewW).toBeGreaterThanOrEqual(MIN_PREVIEW_W);
      expect(w - 2 - l.previewW).toBeGreaterThanOrEqual(320); // the side panel
      expect(flex - l.transportH - l.timelineH).toBeGreaterThanOrEqual(220); // the preview row
    }
  });

  test("before anything is measured, the values are shown as they are", () => {
    expect(paneLayout(NO_PANES, 0, 0)).toMatchObject({
      previewW: DEFAULT_PREVIEW_W,
      transportH: DEFAULT_TRANSPORT_H,
      timelineH: DEFAULT_TIMELINE_H,
    });
    expect(paneLayout(panes(800, 130, 300), 0, 0)).toMatchObject({ previewW: 800, transportH: 130, timelineH: 300 });
  });
});
