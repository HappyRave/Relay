<script lang="ts">
  import StepsTab from "./tabs/StepsTab.svelte";
  import LibraryTab from "./tabs/LibraryTab.svelte";
  import TriggersTab from "./tabs/TriggersTab.svelte";
  import SettingsTab from "./tabs/SettingsTab.svelte";
  import Icon from "../ui/Icon.svelte";
  import { relay } from "../../lib/state/relay.svelte";
  import type { Tab } from "../../lib/types";

  const TABS: [Tab, string][] = [
    ["events", "Steps"],
    ["lib", "Library"],
    ["trig", "Triggers"],
    ["options", "Settings"],
  ];
</script>

<div class="panel">
  <div class="tabs" role="tablist">
    {#each TABS as [key, label] (key)}
      <button role="tab" aria-selected={relay.tab === key} class:active={relay.tab === key} onclick={() => (relay.tab = key)}>
        {label}
      </button>
    {/each}
  </div>
  <div class="body" role="tabpanel">
    {#if relay.tab === "events"}<StepsTab />
    {:else if relay.tab === "lib"}<LibraryTab />
    {:else if relay.tab === "trig"}<TriggersTab />
    {:else}<SettingsTab />{/if}
  </div>
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
</div>

<style>
  .panel {
    display: flex;
    flex-direction: column;
    border-left: 2px solid var(--color-divider);
    min-width: 0;
    height: 338px;
  }
  .tabs {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    border-bottom: 2px solid var(--color-divider);
  }
  .tabs button {
    height: 36px;
    border: 0;
    border-bottom: 3px solid transparent;
    margin-bottom: -2px;
    background: transparent;
    font: inherit;
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    text-align: left;
    padding: 0 12px;
    cursor: pointer;
    color: var(--color-neutral-600);
  }
  .tabs button:hover {
    background: var(--color-neutral-200);
  }
  .tabs button.active {
    border-bottom-color: var(--color-accent);
    color: var(--color-text);
  }
  .body {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
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
