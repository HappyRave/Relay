<script lang="ts">
  // A divider in the gap between two panes: a rule shows when it's hovered,
  // dragged or focused. Drag it, or focus it and use the arrow keys (Shift for
  // bigger steps, Home and End for the limits); double-click puts it back. `value` is the size of the
  // pane it resizes; with `invert`, that pane is after the divider (dragging
  // up or left makes it bigger).
  let {
    orientation,
    label,
    value,
    min,
    max,
    invert = false,
    onchange,
    oncommit,
    onreset,
  }: {
    /** The divider's own direction: "vertical" between left and right panes. */
    orientation: "vertical" | "horizontal";
    label: string;
    value: number;
    min: number;
    max: number;
    invert?: boolean;
    onchange: (value: number) => void;
    /** A drag or a key press ended: time to save. */
    oncommit: () => void;
    onreset: () => void;
  } = $props();

  const STEP = 8;
  const BIG_STEP = 32;
  let drag: { from: number; value: number } | null = $state(null);

  const clamp = (v: number) => Math.round(Math.min(Math.max(v, min), Math.max(min, max)));
  const at = (e: PointerEvent) => (orientation === "vertical" ? e.clientX : e.clientY);
  const moved = (delta: number) => (invert ? -delta : delta);

  function down(e: PointerEvent) {
    if (e.button !== 0) return;
    e.preventDefault();
    drag = { from: at(e), value };
    try {
      // Keeps the drag when the pointer leaves the thin rule.
      (e.currentTarget as Element).setPointerCapture(e.pointerId);
    } catch {
      // The pointer is already up (or was never down): the drag still follows its events.
    }
  }
  function move(e: PointerEvent) {
    if (!drag) return;
    const next = clamp(drag.value + moved(at(e) - drag.from));
    if (next !== value) onchange(next);
  }
  function up() {
    if (!drag) return;
    drag = null;
    oncommit();
  }
  function key(e: KeyboardEvent) {
    const [less, more] = orientation === "vertical" ? ["ArrowLeft", "ArrowRight"] : ["ArrowUp", "ArrowDown"];
    const step = e.shiftKey ? BIG_STEP : STEP;
    let next: number;
    if (e.key === less) next = clamp(value + moved(-step));
    else if (e.key === more) next = clamp(value + moved(step));
    else if (e.key === "Home") next = clamp(min);
    else if (e.key === "End") next = clamp(max);
    else return;
    e.preventDefault();
    if (next !== value) onchange(next);
    oncommit();
  }
</script>

<!-- A focusable separator is the ARIA pattern for a pane splitter (a widget, not static content). -->
<!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
<div
  class="divider {orientation}"
  class:dragging={drag}
  role="separator"
  aria-orientation={orientation}
  aria-label={label}
  aria-valuenow={Math.round(value)}
  aria-valuemin={Math.round(min)}
  aria-valuemax={Math.round(Math.max(min, max))}
  tabindex="0"
  title="Drag to resize; double-click to reset"
  onpointerdown={down}
  onpointermove={move}
  onpointerup={up}
  onlostpointercapture={up}
  onkeydown={key}
  ondblclick={onreset}
></div>

<style>
  .divider {
    position: relative;
    flex: none;
    touch-action: none;
    z-index: 1;
  }
  .vertical {
    width: var(--gap);
    cursor: col-resize;
  }
  .horizontal {
    height: var(--gap);
    cursor: row-resize;
  }
  /* The rule, in the middle of the gap. */
  .divider::after {
    content: "";
    position: absolute;
    border-radius: 1px;
    transition: background 120ms;
  }
  .vertical::after {
    inset: 8px calc(50% - 1px);
  }
  .horizontal::after {
    inset: calc(50% - 1px) 8px;
  }
  .divider:hover::after,
  .divider.dragging::after,
  .divider:focus-visible::after {
    background: var(--text-2);
  }
  .divider:focus-visible {
    outline: none;
  }
  @media (forced-colors: active) {
    .divider:hover::after,
    .divider.dragging::after,
    .divider:focus-visible::after {
      background: Highlight;
    }
  }
</style>
