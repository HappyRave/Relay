// The editor's two dividers: the preview's width (against the side panel)
// and the timeline's height (against the preview row). What's saved is what
// the user chose (null: the default); what's shown is that, kept inside the
// window as it is now, so a window made smaller and then bigger again gets
// the user's layout back.
import type { Panes } from "./ipc/bindings/Panes";

/** The design's layout, at the default 944 × 612 editor. */
export const DEFAULT_PREVIEW_W = 600;
export const DEFAULT_TIMELINE_H = 146;
/** Narrowest preview and side panel (the four tabs fit), shortest preview row. */
export const MIN_PREVIEW_W = 360;
export const MIN_PANEL_W = 320;
export const MIN_MAIN_H = 220;
/** The timeline's lanes never get thinner than the design's 26 px. */
export const MIN_TIMELINE_H = DEFAULT_TIMELINE_H;
/** The divider's own thickness. */
export const SPLITTER = 2;

export const NO_PANES: Panes = { preview_w: null, timeline_h: null };

export interface PaneLayout {
  previewW: number;
  previewMax: number;
  timelineH: number;
  timelineMax: number;
}

const clamp = (v: number, lo: number, hi: number) => Math.min(Math.max(v, lo), Math.max(lo, hi));

/**
 * Where the dividers are in an editor `width` px wide, whose preview row and
 * timeline share `flexible` px of height. A size of 0 means not measured yet
 * (or no layout, as in jsdom): the user's values are shown as they are.
 */
export function paneLayout(panes: Panes, width: number, flexible: number): PaneLayout {
  const wantW = panes.preview_w ?? DEFAULT_PREVIEW_W;
  const wantH = panes.timeline_h ?? DEFAULT_TIMELINE_H;
  const previewMax = width > 0 ? width - SPLITTER - MIN_PANEL_W : Math.max(wantW, DEFAULT_PREVIEW_W);
  const timelineMax = flexible > 0 ? flexible - MIN_MAIN_H : Math.max(wantH, DEFAULT_TIMELINE_H);
  return {
    previewW: Math.round(clamp(wantW, MIN_PREVIEW_W, previewMax)),
    previewMax: Math.max(MIN_PREVIEW_W, Math.round(previewMax)),
    timelineH: Math.round(clamp(wantH, MIN_TIMELINE_H, timelineMax)),
    timelineMax: Math.max(MIN_TIMELINE_H, Math.round(timelineMax)),
  };
}
