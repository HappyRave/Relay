<script lang="ts">
  // Minimize, and close: to the tray, or quitting when Close to the tray is off.
  import Key from "../kit/Key.svelte";
  import Icon from "../ui/Icon.svelte";
  import { relay } from "../../lib/state/relay.svelte";
  import { hideToTray, isTauri, minimize } from "../../lib/platform/window";

  const close = $derived(relay.settings.close_to_tray ? "Hide to the tray" : "Quit");
</script>

{#if isTauri()}
  <span class="winctl">
    <Key variant="ghost" size="sm" iconOnly aria-label="Minimize" title="Minimize" onclick={minimize}>
      <Icon name="minimize" />
    </Key>
    <Key variant="ghost" size="sm" iconOnly aria-label={close} title={close} onclick={hideToTray}>
      <Icon name="close" />
    </Key>
  </span>
{/if}

<style>
  .winctl {
    display: flex;
    gap: 2px;
    margin-left: 2px;
  }
</style>
