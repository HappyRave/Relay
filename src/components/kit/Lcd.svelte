<script lang="ts">
  // A small dark screen with device numerals: the time, a count.
  import type { Snippet } from "svelte";

  let { label, children }: { label?: string; children: Snippet } = $props();
</script>

<span class="lcd num" role={label ? "img" : undefined} aria-label={label}>{@render children()}</span>

<style>
  .lcd {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    height: 26px;
    padding: 0 10px;
    border-radius: var(--r-sm);
    font-size: 14px;
    font-weight: 650;
    font-stretch: 125%;
    white-space: nowrap;
    color: var(--paper);
    background: var(--lcd);
    box-shadow: inset 0 1px 3px rgba(0, 0, 0, 0.6);
  }
  :global([data-theme="dark"]) .lcd {
    box-shadow: inset 0 0 0 1px var(--line-strong);
  }
  .lcd :global(.of) {
    font-weight: 500;
    color: var(--alu-400);
  }
</style>
