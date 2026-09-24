<script lang="ts" generics="T">
  let {
    options,
    value,
    onchange,
    size = "sm",
    label,
  }: {
    options: [T, string][];
    value: T;
    onchange: (v: T) => void;
    size?: "sm" | "md";
    label: string;
  } = $props();
</script>

<div class="seg-group" role="radiogroup" aria-label={label}>
  {#each options as [v, text] (text)}
    <button
      class="opt {size}"
      class:active={v === value}
      role="radio"
      aria-checked={v === value}
      onclick={() => onchange(v)}>{text}</button
    >
  {/each}
</div>

<style>
  .seg-group {
    display: flex;
    border: 1px solid var(--color-divider);
  }
  .opt {
    border: 0;
    font: inherit;
    font-size: 12px;
    cursor: pointer;
    background: transparent;
    color: var(--color-text);
  }
  .opt:not(.active):hover {
    background: var(--color-neutral-200);
  }
  .sm {
    padding: 5px 9px;
  }
  .md {
    padding: 6px 10px;
    font-weight: 600;
  }
  .active {
    background: var(--color-accent);
    color: var(--color-bg);
  }
</style>
