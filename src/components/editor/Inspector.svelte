<script lang="ts">
  // The inspector: the selected steps (Step), or the macro as a whole (Macro).
  import Tabs from "../kit/Tabs.svelte";
  import StepPane from "./StepPane.svelte";
  import MacroPane from "./MacroPane.svelte";
  import { relay } from "../../lib/state/relay.svelte";

  let panel: HTMLElement | undefined = $state();

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
      <MacroPane />
    {:else}
      <StepPane />
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
</style>
