<script lang="ts">
  // A device key: depth, a hover lift, travel when pressed, and a spring back.
  import type { Snippet } from "svelte";
  import type { HTMLButtonAttributes } from "svelte/elements";

  let {
    variant = "plain",
    size = "md",
    round = false,
    iconOnly = false,
    live = false,
    children,
    ...rest
  }: HTMLButtonAttributes & {
    variant?: "plain" | "ink" | "red" | "ghost" | "danger";
    size?: "md" | "sm";
    round?: boolean;
    iconOnly?: boolean;
    /** Breathes: the record key while recording. */
    live?: boolean;
    children?: Snippet;
  } = $props();
</script>

<button type="button" class="key {variant} {size}" class:round class:icon-only={iconOnly} class:live {...rest}>
  {@render children?.()}
</button>

<style>
  .key {
    --key-bg: var(--plate);
    --key-fg: var(--text);
    position: relative;
    display: inline-flex;
    align-items: center;
    justify-content: flex-start;
    gap: 8px;
    flex: none;
    height: var(--key-h);
    min-width: var(--key-h);
    padding: 0 12px;
    font: inherit;
    font-weight: 700;
    font-size: 14px;
    color: var(--key-fg);
    background: var(--key-bg);
    border: 0;
    border-radius: var(--r-md);
    cursor: pointer;
    white-space: nowrap;
    box-shadow: var(--shadow-key);
    transition:
      transform var(--t-release) var(--spring),
      box-shadow var(--t-release) var(--ease),
      background 120ms;
  }
  .key:hover:not(:disabled) {
    transform: translateY(-1px);
  }
  .key:active:not(:disabled) {
    transform: translateY(2px) scale(0.97);
    box-shadow:
      0 0 0 1px var(--line-strong),
      inset 0 2px 3px rgba(0, 0, 0, 0.2);
    transition-duration: var(--t-press);
  }
  .key:focus-visible {
    outline: 2px solid var(--text);
    outline-offset: 2px;
  }
  .key:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }
  .icon-only {
    justify-content: center;
    padding: 0;
  }
  .ink {
    --key-bg: var(--ink);
    --key-fg: var(--paper);
  }
  :global([data-theme="dark"]) .ink {
    --key-bg: var(--paper);
    --key-fg: var(--ink);
  }
  .red {
    --key-bg: var(--red);
    --key-fg: #fff;
  }
  .red:active:not(:disabled) {
    --key-bg: var(--red-press);
  }
  .danger {
    --key-fg: var(--red-text);
  }
  .round {
    border-radius: 50%;
    width: var(--key-h);
    padding: 0;
    justify-content: center;
  }
  .ghost {
    background: transparent;
    box-shadow: none;
  }
  .ghost:hover:not(:disabled) {
    background: var(--hover);
    transform: none;
  }
  .sm {
    height: var(--key-sm);
    min-width: max(24px, var(--key-sm));
    padding: 0 9px;
    font-size: 13px;
    border-radius: var(--r-sm);
    gap: 6px;
  }
  .sm.icon-only {
    padding: 0;
  }
  .live {
    animation: breathe 1.6s ease-in-out infinite;
  }
  @keyframes breathe {
    0%,
    100% {
      box-shadow: var(--shadow-key), 0 0 0 0 rgba(224, 48, 15, 0.55);
    }
    50% {
      box-shadow: var(--shadow-key), 0 0 0 8px rgba(224, 48, 15, 0);
    }
  }
  @media (forced-colors: active) {
    .key {
      border: 1px solid ButtonText;
    }
  }
</style>
