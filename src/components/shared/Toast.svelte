<script lang="ts">
  // The one place errors and notices are shown: at the bottom of the side
  // panel, or under the compact player.
  import Icon from "../ui/Icon.svelte";
  import { relay } from "../../lib/state/relay.svelte";
</script>

{#if relay.toast}
  {@const t = relay.toast}
  <div class="toast" class:error={t.kind === "error"} role={t.kind === "error" ? "alert" : "status"}>
    <span class="message">{t.message}</span>
    {#if t.action}
      <button class="btn btn-ghost action" onclick={t.action.run}>{t.action.label}</button>
    {/if}
    <button class="close" aria-label="Dismiss" onclick={relay.dismissToast}><Icon name="x" size={12} /></button>
  </div>
{/if}

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
