import { describe, expect, test } from "vitest";
import { DEFAULT_PREVIEW_W, DEFAULT_TIMELINE_H, MIN_PREVIEW_W, MIN_TIMELINE_H, NO_PANES, paneLayout } from "./layout";

// The default editor: 940 px inside its border; the preview row and the timeline share 486 px.
const [W, FLEX] = [940, 486];

describe("paneLayout", () => {
  test("the design's layout by default", () => {
    expect(paneLayout(NO_PANES, W, FLEX)).toEqual({ previewW: 600, previewMax: 618, timelineH: 146, timelineMax: 266 });
  });

  test("the user's dividers, kept so the side panel and the preview row keep their minimum", () => {
    expect(paneLayout({ preview_w: 500, timeline_h: 200 }, W, FLEX)).toMatchObject({ previewW: 500, timelineH: 200 });
    expect(paneLayout({ preview_w: 9000, timeline_h: 9000 }, W, FLEX)).toMatchObject({ previewW: 618, timelineH: 266 });
    expect(paneLayout({ preview_w: 10, timeline_h: 10 }, W, FLEX)).toMatchObject({
      previewW: MIN_PREVIEW_W,
      timelineH: MIN_TIMELINE_H,
    });
  });

  test("a bigger window gives the dividers more room; a smaller one squeezes them, without forgetting them", () => {
    const panes = { preview_w: 900, timeline_h: 400 };
    expect(paneLayout(panes, 1400, 900)).toEqual({ previewW: 900, previewMax: 1078, timelineH: 400, timelineMax: 680 });
    expect(paneLayout(panes, W, FLEX)).toMatchObject({ previewW: 618, timelineH: 266 });
    expect(panes).toEqual({ preview_w: 900, timeline_h: 400 }); // the saved values stay
  });

  test("a window smaller than both minimums keeps the minimums", () => {
    expect(paneLayout(NO_PANES, 500, 200)).toEqual({ previewW: 360, previewMax: 360, timelineH: 146, timelineMax: 146 });
  });

  test("before anything is measured, the values are shown as they are", () => {
    expect(paneLayout(NO_PANES, 0, 0)).toMatchObject({ previewW: DEFAULT_PREVIEW_W, timelineH: DEFAULT_TIMELINE_H });
    expect(paneLayout({ preview_w: 800, timeline_h: 300 }, 0, 0)).toMatchObject({ previewW: 800, timelineH: 300 });
  });
});
