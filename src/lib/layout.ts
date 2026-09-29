// The editor's three dividers: the preview's width (against the side
// panel), and the heights of the button row and the timeline (against the
// preview row). What's saved is what the user chose (null: the default);
// what's shown is that, kept inside the window as it is now, so a window
// made smaller and then bigger again gets the user's layout back.
import type { Panes } from "./ipc/bindings/Panes";
import { BAR_H } from "./transport";

/** The design's layout, at the default 944 × 612 editor. */
export const DEFAULT_PREVIEW_W = 600;
export const DEFAULT_TRANSPORT_H = BAR_H;
export const DEFAULT_TIMELINE_H = 146;
/** Narrowest preview and side panel (the four tabs fit), shortest preview row. */
export const MIN_PREVIEW_W = 360;
export const MIN_PANEL_W = 320;
export const MIN_MAIN_H = 220;
/** The button row and the timeline never get smaller than the design's; the controls grow up to twice their size. */
export const MIN_TRANSPORT_H = DEFAULT_TRANSPORT_H;
export const MAX_TRANSPORT_H = 2 * DEFAULT_TRANSPORT_H;
export const MIN_TIMELINE_H = DEFAULT_TIMELINE_H;
/** The divider's own thickness. */
export const SPLITTER = 2;

export const NO_PANES: Panes = { preview_w: null, transport_h: null, timeline_h: null };

export interface PaneLayout {
  previewW: number;
  previewMax: number;
  transportH: number;
  transportMax: number;
  timelineH: number;
  timelineMax: number;
}

const clamp = (v: number, lo: number, hi: number) => Math.min(Math.max(v, lo), Math.max(lo, hi));

/**
 * Where the dividers are in an editor `width` px wide, whose preview row,
 * button row and timeline share `flexible` px of height (the preview row
 * keeps at least its minimum). A size of 0 means not measured yet (or no
 * layout, as in jsdom): the user's values are shown as they are.
 */
export function paneLayout(panes: Panes, width: number, flexible: number): PaneLayout {
  const wantW = panes.preview_w ?? DEFAULT_PREVIEW_W;
  const wantT = panes.transport_h ?? DEFAULT_TRANSPORT_H;
  const wantH = panes.timeline_h ?? DEFAULT_TIMELINE_H;
  const measured = flexible > 0;
  const previewMax = width > 0 ? width - SPLITTER - MIN_PANEL_W : Math.max(wantW, DEFAULT_PREVIEW_W);
  const transportMax = Math.min(MAX_TRANSPORT_H, measured ? flexible - MIN_MAIN_H - MIN_TIMELINE_H : MAX_TRANSPORT_H);
  const transportH = Math.round(clamp(wantT, MIN_TRANSPORT_H, transportMax));
  const timelineMax = measured ? flexible - MIN_MAIN_H - transportH : Math.max(wantH, DEFAULT_TIMELINE_H);
  return {
    previewW: Math.round(clamp(wantW, MIN_PREVIEW_W, previewMax)),
    previewMax: Math.max(MIN_PREVIEW_W, Math.round(previewMax)),
    transportH,
    transportMax: Math.max(MIN_TRANSPORT_H, Math.round(transportMax)),
    timelineH: Math.round(clamp(wantH, MIN_TIMELINE_H, timelineMax)),
    timelineMax: Math.max(MIN_TIMELINE_H, Math.round(timelineMax)),
  };
}
