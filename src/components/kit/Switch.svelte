<script lang="ts">
  // A switch with a check mark inside when on, so it doesn't rely on color.
  let {
    on,
    label,
    labelledby,
    disabled = false,
    onchange,
  }: { on: boolean; label?: string; labelledby?: string; disabled?: boolean; onchange: (v: boolean) => void } = $props();
</script>

<button
  class="switch"
  type="button"
  role="switch"
  aria-checked={on}
  aria-label={label}
  aria-labelledby={labelledby}
  {disabled}
  onclick={() => onchange(!on)}
></button>

<style>
  .switch {
    position: relative;
    flex: none;
    width: 44px;
    height: 24px;
    padding: 0;
    border: 0;
    border-radius: 7px;
    background: var(--alu-300);
    box-shadow: inset 0 0 0 1px var(--line-strong);
    cursor: pointer;
    transition: background 160ms;
  }
  :global([data-theme="dark"]) .switch {
    background: var(--graphite-800);
  }
  .switch::after {
    content: "";
    position: absolute;
    top: 3px;
    left: 3px;
    width: 18px;
    height: 18px;
    border-radius: 5px;
    background: var(--paper);
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.3);
    transition: transform var(--t-release) var(--spring);
  }
  .switch[aria-checked="true"] {
    background: var(--ink);
  }
  .switch[aria-checked="true"]::after {
    transform: translateX(20px);
  }
  .switch[aria-checked="true"]::before {
    content: "";
    position: absolute;
    left: 8px;
    top: 8px;
    width: 7px;
    height: 4px;
    border-left: 2px solid var(--paper);
    border-bottom: 2px solid var(--paper);
    transform: rotate(-45deg);
  }
  :global([data-theme="dark"]) .switch[aria-checked="true"] {
    background: var(--paper);
  }
  :global([data-theme="dark"]) .switch[aria-checked="true"]::after {
    background: var(--ink);
  }
  :global([data-theme="dark"]) .switch[aria-checked="true"]::before {
    border-color: var(--ink);
  }
  .switch:focus-visible {
    outline: 2px solid var(--text);
    outline-offset: 2px;
  }
  .switch:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }
  @media (forced-colors: active) {
    .switch {
      border: 1px solid ButtonText;
    }
    .switch[aria-checked="true"] {
      background: Highlight;
    }
  }
</style>
