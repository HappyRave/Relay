<script lang="ts">
  let {
    on,
    onchange,
    label,
    disabled = false,
  }: { on: boolean; onchange: (v: boolean) => void; label: string; disabled?: boolean } = $props();
</script>

<button class="toggle" class:on role="switch" aria-checked={on} aria-label={label} {disabled} onclick={() => onchange(!on)}>
  <span class="knob"></span>
</button>

<style>
  .toggle {
    width: 36px;
    height: 20px;
    flex: none;
    border: 2px solid var(--color-text);
    background: transparent;
    position: relative;
    padding: 0;
    cursor: pointer;
  }
  .toggle:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }
  .toggle:hover:not(:disabled) {
    background: var(--color-neutral-200);
  }
  .knob {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 12px;
    height: 12px;
    background: var(--color-text);
  }
  .on,
  .on:hover {
    background: var(--color-accent);
  }
  .on .knob {
    left: 18px;
    background: var(--color-bg);
  }
</style>
