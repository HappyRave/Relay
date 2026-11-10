// The editor's one divider, between the steps list and the inspector. What's
// saved is what the user chose (null: the default); what's shown is that,
// kept inside the window as it is now, so a window made smaller and then
// bigger again gets the user's width back.
import type { Panes } from "./ipc/bindings/Panes";

/** The inspector's width by default, and its narrowest. */
export const INSPECTOR_W = 340;
export const MIN_INSPECTOR_W = 280;
/** The steps list keeps at least this much. */
export const MIN_LIST_W = 320;

export const NO_PANES: Panes = { inspector_w: null };

/** The inspector's width shown in a `main` px wide row (0: not measured yet), and how wide it may get there. */
export function inspectorWidth(panes: Panes, main: number): { width: number; max: number } {
  const max = main ? Math.max(MIN_INSPECTOR_W, main - MIN_LIST_W) : Infinity;
  return { width: Math.min(max, Math.max(MIN_INSPECTOR_W, panes.inspector_w ?? INSPECTOR_W)), max };
}
