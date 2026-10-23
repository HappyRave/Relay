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
    <button class="btn btn-ghost action" onclick={relay.stop}>Stop</button>
  </div>
{/if}

{#each relay.shownToasts as t (t.id)}
  <div class="toast" class:error={t.kind === "error"} role={t.kind === "error" ? "alert" : "status"}>
    <span class="message">{t.message}</span>
    {#if t.action}
      <button class="btn btn-ghost action" onclick={t.action.run}>{t.action.label}</button>
    {/if}
    <button class="close" aria-label="Dismiss" onclick={() => relay.dismissToast(t.id)}><Icon name="x" size={12} /></button>
  </div>
{/each}

<style>
  .toast {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 6px 6px 12px;
    border-top: 2px solid var(--color-text);
    background: var(--color-text);
    color: var(--color-bg);
    font-size: 12px;
  }
  .toast.run {
    border-top-color: var(--color-accent);
  }
  .rec-dot {
    width: 10px;
    height: 10px;
    flex: none;
    border-radius: 50%;
    background: var(--color-accent);
  }
  .toast.error {
    border-top-color: var(--color-accent);
    background: var(--color-accent-100);
    color: var(--color-accent-800);
  }
  .message {
    flex: 1;
    min-width: 0;
  }
  .action {
    font-size: 12px;
    padding: 2px 6px;
    color: var(--color-accent-400);
  }
  .error .action {
    color: var(--color-accent-700);
  }
  .close {
    width: 22px;
    height: 22px;
    flex: none;
    border: 0;
    background: transparent;
    color: inherit;
    opacity: 0.7;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .close:hover {
    opacity: 1;
  }
</style>
