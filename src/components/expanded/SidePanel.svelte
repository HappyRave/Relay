<script lang="ts">
  import StepsTab from "./tabs/StepsTab.svelte";
  import LibraryTab from "./tabs/LibraryTab.svelte";
  import TriggersTab from "./tabs/TriggersTab.svelte";
  import SettingsTab from "./tabs/SettingsTab.svelte";
  import Toast from "../shared/Toast.svelte";
  import { relay } from "../../lib/state/relay.svelte";
  import type { Tab } from "../../lib/types";

  const TABS: [Tab, string][] = [
    ["steps", "Steps"],
    ["library", "Library"],
    ["triggers", "Triggers"],
    ["settings", "Settings"],
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
    {#if relay.tab === "steps"}<StepsTab />
    {:else if relay.tab === "library"}<LibraryTab />
    {:else if relay.tab === "triggers"}<TriggersTab />
    {:else}<SettingsTab />{/if}
  </div>
  <Toast />
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
</style>
