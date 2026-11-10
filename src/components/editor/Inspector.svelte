<script lang="ts">
  // The inspector: the selected steps (Step), or the macro as a whole (Macro).
  import Tabs from "../kit/Tabs.svelte";
  import StepPane from "./StepPane.svelte";
  import MacroPane from "./MacroPane.svelte";
  import Key from "../kit/Key.svelte";
  import Icon from "../ui/Icon.svelte";
  import { relay } from "../../lib/state/relay.svelte";

  let panel: HTMLElement | undefined = $state();

  /** Enter in the steps list: the first field of the step. */
  export function focus() {
    relay.inspector = "step";
    panel?.querySelector<HTMLElement>("input, textarea, button, [tabindex='0']")?.focus();
  }
</script>

<section class="plate inspector" class:open={relay.inspectorOpen} aria-label="Inspector">
  <div class="tabs">
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
  <span class="close">
    <Key variant="ghost" size="sm" iconOnly aria-label="Close the inspector" title="Close (Esc)" onclick={() => (relay.inspectorOpen = false)}><Icon name="close" size={14} /></Key>
  </span>
  </div>
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
  .tabs {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .tabs > :global(:first-child) {
    flex: 1;
  }
  .close {
    display: none;
  }
  /* In a narrow window, it slides over the list (see MacroPage). */
  :global(.narrow) .inspector {
    position: absolute;
    top: 0;
    right: 0;
    bottom: 0;
    z-index: 12;
    width: min(340px, 86%);
    box-shadow: var(--shadow-pop);
    visibility: hidden;
    transform: translateX(calc(100% + 16px));
    transition:
      transform 240ms var(--ease),
      visibility 240ms;
  }
  :global(.narrow) .inspector.open {
    visibility: visible;
    transform: none;
  }
  :global(.narrow) .close {
    display: inline-flex;
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
