<script lang="ts">
  import Icon from "../ui/Icon.svelte";

  let { checked, label, onchange }: { checked: boolean; label: string; onchange: (v: boolean) => void } = $props();
</script>

<button class="check" type="button" role="checkbox" aria-checked={checked} aria-label={label} onclick={() => onchange(!checked)}>
  {#if checked}<Icon name="check" size={14} />{/if}
</button>

<style>
  .check {
    flex: none;
    width: 24px;
    height: 24px;
    padding: 0;
    border: 0;
    border-radius: var(--r-sm);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    color: var(--text-inv);
    background: var(--plate);
    box-shadow: inset 0 0 0 1.6px var(--line-strong);
    cursor: pointer;
  }
  .check[aria-checked="true"] {
    background: var(--text);
    box-shadow: none;
  }
  .check:focus-visible {
    outline: 2px solid var(--text);
    outline-offset: 2px;
  }
</style>
