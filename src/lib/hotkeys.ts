// Turning a key press into the hotkey labels relay-core parses ("Ctrl + Alt + 1").

const NAMED: Record<string, string> = {
  Backspace: "Backspace",
  ArrowLeft: "Left",
  ArrowRight: "Right",
  ArrowUp: "Up",
  ArrowDown: "Down",
  PageUp: "PgUp",
  PageDown: "PgDn",
  Delete: "Del",
  Insert: "Ins",
  Enter: "Enter",
  Tab: "Tab",
  Space: "Space",
  Home: "Home",
  End: "End",
};

/**
 * The label relay-core expects for a key. Letters come from the layout
 * (e.key), so the key labeled A on AZERTY is "A", like RegisterHotKey sees
 * it; on a non-Latin layout (Cyrillic, Greek) the physical key is used.
 * Digits, function keys and named keys come from the physical key.
 */
export function keyLabel(e: KeyboardEvent): string | null {
  if (/^[a-z]$/i.test(e.key)) return e.key.toUpperCase();
  if (/^Key[A-Z]$/.test(e.code) && e.key.length === 1 && e.key.charCodeAt(0) > 127) return e.code.slice(3);
  if (/^Digit\d$/.test(e.code)) return e.code.slice(5);
  if (/^F([1-9]|1\d|2[0-4])$/.test(e.code)) return e.code;
  if (/^Numpad\d$/.test(e.code)) return "Num " + e.code.slice(6);
  return NAMED[e.code] ?? null;
}

/** " (Ctrl + Shift + F9)", to name a key in a tooltip; nothing for a key that's off. */
export const keyHint = (combo: string | null | undefined): string => (combo ? ` (${combo})` : "");

const ARIA: Record<string, string> = { Ctrl: "Control", Win: "Meta", Esc: "Escape", Del: "Delete", Ins: "Insert", PgUp: "PageUp", PgDn: "PageDown", Left: "ArrowLeft", Right: "ArrowRight", Up: "ArrowUp", Down: "ArrowDown" };

/** A combo as `aria-keyshortcuts` names it ("Control+Shift+F9"), or undefined for a key that's off. */
export function ariaKeys(combo: string | null | undefined): string | undefined {
  if (!combo) return undefined;
  return combo
    .split(" + ")
    .map((p) => ARIA[p] ?? (p.startsWith("Num ") ? "Numpad" + p.slice(4) : p))
    .join("+");
}

/** "Ctrl + Alt + 1" from a key press, or null for a lone modifier or an unusable key. */
export function comboOf(e: KeyboardEvent): string | null {
  const key = keyLabel(e);
  if (!key) return null;
  const mods = [e.ctrlKey && "Ctrl", e.altKey && "Alt", e.shiftKey && "Shift", e.metaKey && "Win"].filter(Boolean);
  return [...mods, key].join(" + ");
}
