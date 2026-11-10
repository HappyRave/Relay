<script lang="ts">
  // The one place errors and notices are shown, at the bottom of the window,
  // the newest at the bottom. A run a trigger started comes first, with Stop.
  import Icon from "../ui/Icon.svelte";
  import { relay } from "../../lib/state/relay.svelte";
  import { runningText } from "../../lib/runs";

  const source = $derived(relay.runSource && relay.runSource !== "manual" ? relay.runSource : null);
  const running = $derived(relay.library.find((m) => m.id === relay.playingId)?.name ?? relay.name);
</script>

{#if source}
  <div class="toast run" role="status">
    <span class="rec-dot" aria-hidden="true"></span>
    <span class="message">{runningText(running, source)}</span>
    <button class="action" onclick={relay.stop}>Stop</button>
  </div>
{/if}

{#each relay.shownToasts as t (t.id)}
  <div class="toast" class:error={t.kind === "error"} role={t.kind === "error" ? "alert" : "status"}>
    <span class="message">{t.message}</span>
    {#if t.action}
      <button class="action" onclick={t.action.run}>{t.action.label}</button>
    {/if}
    <button class="close" aria-label="Dismiss" onclick={() => relay.dismissToast(t.id)}><Icon name="x" size={12} /></button>
  </div>
{/each}

<style>
  .toast {
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: 40px;
    padding: 6px 6px 6px 12px;
    border-radius: var(--r-md);
    background: var(--ink);
    color: var(--paper);
    box-shadow: var(--shadow-pop);
    font-size: 13px;
    animation: rise 200ms var(--spring);
  }
  :global(:root[data-theme="dark"]) .toast:not(.error, .run) {
    background: var(--paper);
    color: var(--ink);
  }
  .toast.error {
    background: var(--red);
    color: #fff;
  }
  .toast.run {
    background: var(--plate);
    color: var(--text);
    box-shadow:
      0 0 0 1.5px var(--red),
      var(--shadow-pop);
  }
  .rec-dot {
    width: 10px;
    height: 10px;
    flex: none;
    border-radius: 50%;
    background: var(--red);
    animation: breathe 1.6s ease-in-out infinite;
  }
  .message {
    flex: 1;
    min-width: 0;
  }
  .action,
  .close {
    flex: none;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    height: 28px;
    border: 0;
    border-radius: var(--r-sm);
    font: inherit;
    font-weight: 700;
    font-size: 13px;
    color: inherit;
    background: transparent;
    cursor: pointer;
  }
  .action {
    padding: 0 10px;
    box-shadow: 0 0 0 1px currentColor;
  }
  .close {
    width: 28px;
    opacity: 0.75;
  }
  .action:hover,
  .close:hover {
    opacity: 1;
    background: color-mix(in srgb, currentColor 14%, transparent);
  }
  @keyframes rise {
    from {
      opacity: 0;
      transform: translateY(8px);
    }
  }
  @keyframes breathe {
    50% {
      opacity: 0.45;
    }
  }
  @media (forced-colors: active) {
    .toast {
      border: 1px solid CanvasText;
    }
  }
</style>
