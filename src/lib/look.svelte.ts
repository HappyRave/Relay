// The look on the page: theme.css reads these attributes on <html>.
import type { Settings } from "./types";
import type { SystemLook } from "./ipc/bindings/SystemLook";

/** What Windows says through the page: dark app mode, reduced animations. */
export type Media = { dark: boolean; reducedMotion: boolean };

export type LookAttrs = {
  "data-theme": "light" | "dark";
  "data-spacing"?: "compact" | "large";
  "data-glass"?: "off";
  "data-motion"?: "calm";
};

/** Whether the window is frosted (as `SystemLook::glass` in Rust). */
export const glassOn = (s: Settings, sys: SystemLook) => sys.transparency && (s.glass ?? sys.glass_default);

export function lookAttrs(s: Settings, sys: SystemLook, media: Media): LookAttrs {
  const attrs: LookAttrs = { "data-theme": s.theme === "system" ? (media.dark ? "dark" : "light") : s.theme };
  if (s.spacing !== "comfortable") attrs["data-spacing"] = s.spacing;
  if (!glassOn(s, sys)) attrs["data-glass"] = "off";
  if (s.reduce_motion === "always" || media.reducedMotion) attrs["data-motion"] = "calm";
  return attrs;
}

const ATTRS = ["data-theme", "data-spacing", "data-glass", "data-motion"] as const;

export function applyLook(el: HTMLElement, attrs: LookAttrs) {
  for (const name of ATTRS) {
    const v = attrs[name];
    if (v) el.setAttribute(name, v);
    else el.removeAttribute(name);
  }
}

/** Follows Windows' app mode and animation setting while the page runs. */
export class SystemMedia implements Media {
  dark = $state(false);
  reducedMotion = $state(false);

  /** Starts following; returns how to stop. */
  watch() {
    const queries = [
      [matchMedia("(prefers-color-scheme: dark)"), (v: boolean) => (this.dark = v)],
      [matchMedia("(prefers-reduced-motion: reduce)"), (v: boolean) => (this.reducedMotion = v)],
    ] as const;
    const stops = queries.map(([q, set]) => {
      set(q.matches);
      const on = (e: MediaQueryListEvent) => set(e.matches);
      q.addEventListener("change", on);
      return () => q.removeEventListener("change", on);
    });
    return () => stops.forEach((stop) => stop());
  }
}
