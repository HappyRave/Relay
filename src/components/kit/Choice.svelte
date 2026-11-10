<script lang="ts" generics="T">
  // A few exclusive choices in one track: a radio group, with the arrow keys moving
  // and choosing, and one tab stop.
  let {
    options,
    value,
    label,
    labelledby,
    disabled = false,
    onchange,
  }: {
    options: [T, string][];
    value: T;
    label?: string;
    labelledby?: string;
    disabled?: boolean;
    onchange: (v: T) => void;
  } = $props();

  let group: HTMLElement | undefined = $state();
  const current = $derived(Math.max(0, options.findIndex(([v]) => v === value)));

  function key(e: KeyboardEvent, i: number) {
    const d = { ArrowRight: 1, ArrowDown: 1, ArrowLeft: -1, ArrowUp: -1 }[e.key];
    if (!d) return;
    e.preventDefault();
    const j = (i + d + options.length) % options.length;
    onchange(options[j][0]);
    (group?.querySelectorAll("button")[j] as HTMLButtonElement | undefined)?.focus();
  }
</script>

<div class="choice" role="radiogroup" aria-label={label} aria-labelledby={labelledby} bind:this={group}>
  {#each options as [v, text], i (text)}
    <button
      type="button"
      role="radio"
      aria-checked={v === value}
      tabindex={i === current ? 0 : -1}
      {disabled}
      onclick={() => onchange(v)}
      onkeydown={(e) => key(e, i)}>{text}</button
    >
  {/each}
</div>

<style>
  .choice {
    display: inline-flex;
    flex-wrap: wrap;
    gap: 2px;
    padding: 2px;
    border-radius: var(--r-sm);
    background: var(--plate-2);
    box-shadow: inset 0 0 0 1px var(--line);
  }
  button {
    height: calc(var(--key-sm) - 4px);
    min-height: 24px;
    padding: 0 9px;
    border: 0;
    border-radius: 4px;
    font: inherit;
    font-size: 13px;
    font-weight: 700;
    white-space: nowrap;
    color: var(--text-2);
    background: none;
    cursor: pointer;
  }
  button:hover:not(:disabled) {
    color: var(--text);
  }
  button[aria-checked="true"] {
    color: var(--text);
    background: var(--plate);
    box-shadow: 0 0 0 1px var(--line-strong);
  }
  button:focus-visible {
    outline: 2px solid var(--text);
    outline-offset: 1px;
  }
  button:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }
  @media (forced-colors: active) {
    button[aria-checked="true"] {
      outline: 2px solid Highlight;
    }
  }
</style>
