<script lang="ts">
  // The inspector: the selected steps (Step), or the macro as a whole (Macro).
  import Tabs from "../kit/Tabs.svelte";
  import StepEditor from "../expanded/tabs/StepEditor.svelte";
  import SettingsTab from "../expanded/tabs/SettingsTab.svelte";
  import TriggersTab from "../expanded/tabs/TriggersTab.svelte";
  import { relay } from "../../lib/state/relay.svelte";

  let panel: HTMLElement | undefined = $state();
  const step = $derived(relay.steps[relay.selected]);

  /** Enter in the steps list: the first field of the step. */
  export function focus() {
    relay.inspector = "step";
    panel?.querySelector<HTMLElement>("input, textarea, button, [tabindex='0']")?.focus();
  }
</script>

<section class="plate inspector" aria-label="Inspector">
  <Tabs
    label="Inspector"
    panel="inspector-panel"
    tabs={[
      ["step", "Step"],
      ["macro", "Macro", "Macro (Ctrl + M)"],
    ]}
    value={relay.inspector}
    onchange={(v) => (relay.inspector = v)}
  />
  <div class="panel" id="inspector-panel" role="tabpanel" bind:this={panel}>
    {#if relay.inspector === "macro"}
      <SettingsTab />
      <TriggersTab />
    {:else if step && relay.canEdit}
      <StepEditor {step} index={relay.selected} />
    {:else}
      <div class="empty">
        <b>No step selected</b>
        <span>Select a step to edit it, or open the Macro tab for how the macro plays and when it runs.</span>
      </div>
    {/if}
  </div>
</section>

<style>
  .inspector {
    display: flex;
    flex-direction: column;
    gap: 12px;
    min-width: 0;
    min-height: 0;
    padding: 10px var(--pad) var(--pad);
    overflow: hidden;
  }
  .panel {
    flex: 1;
    min-height: 0;
    overflow: auto;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .empty {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 24px 0;
    text-align: center;
    color: var(--text-2);
  }
  .empty b {
    font-size: 15px;
    color: var(--text);
  }
</style>
