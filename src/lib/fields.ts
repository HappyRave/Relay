// Number fields that commit on `change`. Whatever was typed, the field ends
// up showing what's saved: the accepted value (rounded or clamped), or the
// current one again when the input was refused. Svelte alone wouldn't put it
// back, since the bound value didn't change.

/**
 * Commits a number field. `accept` turns the typed number (in the field's
 * unit) into the value to save (in `current`'s unit), or null to refuse it;
 * `show` formats a saved value for the field. Returns the value to save, or
 * null when there's nothing to change.
 */
export function commitNumber(
  input: HTMLInputElement,
  current: number,
  accept: (typed: number) => number | null,
  show: (value: number) => string = String,
): number | null {
  const typed = input.valueAsNumber;
  const next = Number.isFinite(typed) ? accept(typed) : null;
  input.value = show(next ?? current);
  return next == null || next === current ? null : next;
}

export const clamp = (v: number, min: number, max: number) => Math.min(max, Math.max(min, v));

/** Seconds typed in a field, as whole milliseconds. */
export const toMs = (seconds: number) => Math.round(seconds * 1000);
