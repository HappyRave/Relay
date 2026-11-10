<script lang="ts">
  // A number field with its unit. With a step, it scrubs: drag sideways to change
  // the value, like turning an encoder; the arrow keys step it too (Shift: ×10).
  // A click without dragging focuses it for typing. It commits on change, and puts
  // back what it showed if the text isn't a number.
  let {
    value,
    label,
    unit = "",
    pre = "",
    step,
    min = -Infinity,
    max = Infinity,
    decimals = 2,
    disabled = false,
    oncommit,
    onscrub,
  }: {
    value: number;
    label: string;
    unit?: string;
    /** A visible prefix inside the field: X, Y. */
    pre?: string;
    step?: number;
    min?: number;
    max?: number;
    decimals?: number;
    disabled?: boolean;
    oncommit: (v: number) => void;
    /** While dragging, before the commit. */
    onscrub?: (v: number) => void;
  } = $props();

  let input: HTMLInputElement | undefined = $state();
  let dragging = $state(false);
  let offset = $state(0);
  const show = (v: number) => (Number.isInteger(v) && decimals === 0 ? String(v) : v.toFixed(decimals));
  const clamp = (v: number) => Math.min(max, Math.max(min, v));
  const unitName = $derived(({ s: "seconds", ms: "milliseconds", "%": "percent" } as Record<string, string>)[unit] ?? unit);

  function commitText() {
    if (!input) return;
    const v = parseFloat(input.value.replace(",", "."));
    if (Number.isNaN(v)) {
      input.value = show(value);
      return;
    }
    const c = clamp(v);
    input.value = show(c);
    if (c !== value) oncommit(c);
  }

  let start: { x: number; v: number; id: number } | null = null;
  function down(e: PointerEvent) {
    if (!step || disabled || document.activeElement === input || e.button !== 0) return;
    start = { x: e.clientX, v: value, id: e.pointerId };
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
  }
  function move(e: PointerEvent) {
    if (!start || !step || !input) return;
    const dx = e.clientX - start.x;
    if (!dragging && Math.abs(dx) < 3) return;
    dragging = true;
    offset = dx;
    const v = clamp(start.v + Math.round(dx / 4) * step);
    input.value = show(v);
    onscrub?.(v);
  }
  function up() {
    if (!start) return;
    start = null;
    if (dragging) {
      dragging = false;
      commitText();
    } else {
      input?.focus();
      input?.select();
    }
  }
  function key(e: KeyboardEvent) {
    if (!step || (e.key !== "ArrowUp" && e.key !== "ArrowDown")) return;
    e.preventDefault();
    const v = clamp((parseFloat(input!.value.replace(",", ".")) || value) + (e.key === "ArrowUp" ? 1 : -1) * step * (e.shiftKey ? 10 : 1));
    input!.value = show(v);
    oncommit(v);
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions: the drag is a shortcut; the field takes typing and the arrow keys -->
<span
  class="field"
  class:scrub={!!step && !disabled}
  class:dragging
  onpointerdown={down}
  onpointermove={move}
  onpointerup={up}
  onlostpointercapture={up}
>
  {#if pre}<span class="pre" aria-hidden="true">{pre}</span>{/if}
  <input
    bind:this={input}
    class="num"
    inputmode="decimal"
    aria-label={unit ? `${label} in ${unitName}` : label}
    value={show(value)}
    {disabled}
    onchange={commitText}
    onkeydown={key}
  />
  {#if unit}<span class="unit" aria-hidden="true">{unit}</span>{/if}
  {#if step && !disabled}<span class="ticks" aria-hidden="true" style:background-position="{offset}px 0"></span>{/if}
</span>

<style>
  .field {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: var(--field-h);
    min-width: 0;
    padding: 0 10px;
    border-radius: var(--r-sm);
    background: var(--plate-2);
    box-shadow: inset 0 0 0 1px var(--line);
    cursor: text;
  }
  .field:focus-within {
    box-shadow: inset 0 0 0 1.5px var(--text);
  }
  .scrub,
  .scrub input {
    cursor: ew-resize;
  }
  .scrub:focus-within input {
    cursor: text;
  }
  .dragging {
    box-shadow: inset 0 0 0 1.5px var(--blue);
  }
  input {
    width: 100%;
    min-width: 3ch;
    height: 100%;
    border: 0;
    padding: 0;
    background: none;
    font: inherit;
    font-size: 14px;
    color: var(--text);
    outline: none;
  }
  input:disabled {
    color: var(--text-2);
    cursor: not-allowed;
  }
  .pre,
  .unit {
    flex: none;
    font-size: 13px;
    color: var(--text-2);
  }
  .pre {
    font-weight: 700;
  }
  .ticks {
    flex: none;
    width: 34px;
    height: 12px;
    opacity: 0.55;
    background: repeating-linear-gradient(90deg, var(--text-2) 0 1px, transparent 1px 6px);
  }
</style>
