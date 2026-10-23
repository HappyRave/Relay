// Listening for a key combination: the macro hotkey field and Settings → Keys.
import { comboOf } from "./hotkeys";

export interface Capture {
  /** A combination was pressed, or Backspace alone (null: none). */
  done: (combo: string | null) => void;
  /** Esc, or the window lost the focus. */
  cancel: () => void;
  /** A key that can't be part of a combination. */
  hint: (text: string) => void;
}

/**
 * Listens to the window's keys until one of `capture`'s callbacks says how
 * it ended; returns a function that stops listening.
 */
export function listenForCombo(capture: Capture): () => void {
  const onKey = (e: KeyboardEvent) => {
    e.preventDefault();
    e.stopPropagation();
    const mods = e.ctrlKey || e.altKey || e.shiftKey || e.metaKey;
    if (e.key === "Escape" && !mods) return capture.cancel();
    if (e.key === "Backspace" && !mods) return capture.done(null);
    if (["Control", "Alt", "Shift", "Meta"].includes(e.key)) return;
    const c = comboOf(e);
    if (!c) return capture.hint("Use a letter, digit, F-key or arrow");
    capture.done(c);
  };
  window.addEventListener("keydown", onKey, true);
  window.addEventListener("blur", capture.cancel);
  return () => {
    window.removeEventListener("keydown", onKey, true);
    window.removeEventListener("blur", capture.cancel);
  };
}
