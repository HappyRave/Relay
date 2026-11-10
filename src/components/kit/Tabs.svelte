<script lang="ts" generics="T extends string">
  // Two or more tabs in a track, with a thumb that springs to the chosen one.
  // One tab stop; the arrow keys move between tabs and choose them.
  let {
    tabs,
    value,
    label,
    panel,
    onchange,
  }: { tabs: [T, string, string?][]; value: T; label: string; panel: string; onchange: (v: T) => void } = $props();

  let list: HTMLElement | undefined = $state();
  const current = $derived(Math.max(0, tabs.findIndex(([v]) => v === value)));

  function key(e: KeyboardEvent, i: number) {
    const d = { ArrowRight: 1, ArrowLeft: -1 }[e.key];
    if (!d) return;
    e.preventDefault();
    const j = (i + d + tabs.length) % tabs.length;
    onchange(tabs[j][0]);
    (list?.querySelectorAll("[role=tab]")[j] as HTMLButtonElement | undefined)?.focus();
  }
</script>

<div class="tabs" role="tablist" aria-label={label} bind:this={list} style:--n={tabs.length} style:--i={current}>
  <span class="thumb" aria-hidden="true"></span>
  {#each tabs as [v, text, title], i (v)}
    <button
      type="button"
      role="tab"
      aria-selected={v === value}
      aria-controls={panel}
      tabindex={i === current ? 0 : -1}
      {title}
      onclick={() => onchange(v)}
      onkeydown={(e) => key(e, i)}>{text}</button
    >
  {/each}
</div>

<style>
  .tabs {
    position: relative;
    display: grid;
    grid-template-columns: repeat(var(--n), 1fr);
    height: calc(var(--key-sm) + 4px);
    padding: 3px;
    border-radius: var(--r-md);
    background: var(--plate-2);
    box-shadow: inset 0 0 0 1px var(--line);
  }
  .thumb {
    position: absolute;
    top: 3px;
    bottom: 3px;
    left: 3px;
    width: calc((100% - 6px) / var(--n));
    border-radius: var(--r-sm);
    background: var(--plate);
    box-shadow:
      0 0 0 1px var(--line-strong),
      0 1px 2px rgba(0, 0, 0, 0.12);
    transform: translateX(calc(100% * var(--i)));
    transition: transform var(--t-release) var(--spring);
  }
  button {
    position: relative;
    z-index: 1;
    min-height: 24px;
    border: 0;
    border-radius: var(--r-sm);
    font: inherit;
    font-weight: 700;
    color: var(--text-2);
    background: none;
    cursor: pointer;
  }
  button[aria-selected="true"] {
    color: var(--text);
  }
  button:focus-visible {
    outline: 2px solid var(--text);
    outline-offset: 1px;
  }
  @media (forced-colors: active) {
    button[aria-selected="true"] {
      outline: 2px solid Highlight;
    }
  }
</style>
