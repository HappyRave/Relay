<script lang="ts">
  import Grip from "../shared/Grip.svelte";
  import Icon from "../ui/Icon.svelte";
  import { relay } from "../../lib/state/relay.svelte";
  import { hideToTray, isTauri } from "../../lib/platform/window";
  import { plural } from "../../lib/format";

  const tauri = isTauri();
</script>

<div class="header">
  <Grip />
  <div class="brand"><span class="mark"></span>Relay</div>
  <input
    class="name"
    aria-label="Macro name"
    value={relay.name}
    disabled={relay.recording || !relay.view}
    oninput={(e) => relay.rename(e.currentTarget.value)}
  />
  <div class="meta">{plural(relay.steps.length, "step")} · {plural(relay.moves.length, "path sample")}</div>
  {#if relay.editable}
    <button class="icon" title="Undo (Ctrl + Z)" aria-label="Undo" disabled={!relay.canUndo} onclick={relay.undo}>
      <Icon name="undo" size={17} />
    </button>
    <button class="icon" title="Redo (Ctrl + Y)" aria-label="Redo" disabled={!relay.canRedo} onclick={relay.redo}>
      <Icon name="redo" size={17} />
    </button>
  {/if}
  <button class="export" onclick={() => (relay.exportOpen = true)}>Export<Icon name="export" size={15} /></button>
  <button class="icon" title="Compact player (Ctrl + Shift + M)" aria-label="Compact player" onclick={() => (relay.expanded = false)}>
    <Icon name="collapse" size={18} />
  </button>
  {#if tauri}
    <!-- The design has no close control: this hides Relay to the tray (Quit is in the tray menu). -->
    <button
      class="icon"
      title={relay.settings.close_to_tray ? "Hide to tray" : "Quit"}
      aria-label={relay.settings.close_to_tray ? "Hide to tray" : "Quit"}
      onclick={hideToTray}><Icon name="x" size={16} /></button
    >
  {/if}
</div>

<style>
  .header {
    height: 44px;
    display: flex;
    align-items: stretch;
    border-bottom: 2px solid var(--color-divider);
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 14px;
    border-right: 2px solid var(--color-divider);
    font-weight: 800;
    font-size: 14px;
  }
  .mark {
    width: 12px;
    height: 12px;
    background: var(--color-accent);
  }
  .name {
    flex: 1;
    min-width: 0;
    border: 0;
    background: transparent;
    font: inherit;
    font-weight: 600;
    font-size: 14px;
    padding: 0 14px;
    color: var(--color-text);
    caret-color: var(--color-accent);
    user-select: text;
  }
  .name:hover:not(:disabled) {
    background: var(--color-neutral-200);
  }
  .name:focus-visible {
    outline-offset: -2px;
  }
  .meta {
    display: flex;
    align-items: center;
    padding: 0 14px;
    font-size: 12px;
    color: var(--color-neutral-700);
    white-space: nowrap;
  }
  button {
    border: 0;
    border-left: 2px solid var(--color-divider);
    background: transparent;
    color: var(--color-text);
    cursor: pointer;
    display: flex;
    align-items: center;
  }
  button:hover:not(:disabled) {
    background: var(--color-neutral-200);
  }
  button:disabled {
    color: var(--color-neutral-500);
  }
  .export {
    font: inherit;
    font-weight: 800;
    font-size: 13px;
    padding: 0 14px;
    gap: 8px;
  }
  .icon {
    width: 48px;
    justify-content: center;
  }
</style>
