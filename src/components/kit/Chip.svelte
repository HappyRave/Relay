<script lang="ts">
  // A small fact, like a device's printed legend. A button when it does something.
  import type { Snippet } from "svelte";

  let {
    onclick,
    label,
    title,
    children,
  }: { onclick?: () => void; label?: string; title?: string; children: Snippet } = $props();
</script>

{#if onclick}
  <button class="chip" type="button" aria-label={label} title={title ?? label} {onclick}>{@render children()}</button>
{:else}
  <span class="chip" aria-label={label} title={title ?? label}>{@render children()}</span>
{/if}

<style>
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    height: 24px;
    padding: 0 8px;
    border: 0;
    border-radius: var(--r-sm);
    font: inherit;
    font-size: 12px;
    font-weight: 700;
    white-space: nowrap;
    color: var(--text);
    background: var(--plate);
    box-shadow: 0 0 0 1px var(--line-strong);
  }
  button.chip {
    cursor: pointer;
  }
  button.chip:hover {
    background: var(--plate-2);
  }
  button.chip:focus-visible {
    outline: 2px solid var(--text);
    outline-offset: 2px;
  }
</style>
