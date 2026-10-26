<script lang="ts">
  // A key combination as keycaps: Ctrl, Alt, 1. Read as "Ctrl + Alt + 1".
  let { combo, none = "None" }: { combo: string[]; none?: string } = $props();
</script>

{#if combo.length}
  <span class="keycaps" role="img" aria-label={combo.join(" + ")}>
    {#each combo as key, i (i)}<span class="kc" aria-hidden="true">{key}</span>{/each}
  </span>
{:else}
  <span class="none">{none}</span>
{/if}

<style>
  .keycaps {
    display: inline-flex;
    gap: 4px;
    flex-wrap: wrap;
  }
  .kc {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 26px;
    height: 24px;
    padding: 0 7px;
    border-radius: var(--r-sm);
    font-size: 12px;
    font-weight: 700;
    color: var(--text);
    background: var(--plate);
    box-shadow:
      0 0 0 1px var(--line-strong),
      0 1.5px 0 var(--line-strong);
  }
  .none {
    font-size: 13px;
    color: var(--text-2);
  }
</style>
