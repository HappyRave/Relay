<script lang="ts">
  // Relay's window and its three places: the Library, a macro, and Settings.
  import TopBar from "./TopBar.svelte";
  import Key from "../kit/Key.svelte";
  import Icon from "../ui/Icon.svelte";
  import ExpandedWidget from "../ExpandedWidget.svelte";
  import LibraryTab from "../expanded/tabs/LibraryTab.svelte";
  import SettingsPage from "../settings/SettingsPage.svelte";
  import Toast from "../shared/Toast.svelte";
  import { relay } from "../../lib/state/relay.svelte";
</script>

<div class="app-window" data-place={relay.place}>
  {#if relay.place === "library"}
    <div class="library">
      <TopBar>
        {#snippet title()}<span class="brand" data-tauri-drag-region><span class="mark"></span>Relay</span>{/snippet}
        {#snippet actions()}
          <Key size="sm" onclick={() => relay.go("settings")}><Icon name="sliders" />Settings</Key>
        {/snippet}
      </TopBar>
      <main class="plate body legacy">
        <LibraryTab />
        <Toast />
      </main>
    </div>
  {:else if relay.place === "settings"}
    <SettingsPage />
  {:else}
    <div class="plate macro legacy"><ExpandedWidget /></div>
  {/if}
</div>

<style>
  .app-window {
    box-sizing: border-box;
    width: 100%;
    height: 100%;
    padding: var(--gap);
    color: var(--text);
    background: var(--glass);
    font-family: var(--font);
  }
  .library {
    display: grid;
    grid-template-rows: auto minmax(0, 1fr);
    gap: var(--gap);
    height: 100%;
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 16px;
    font-weight: 800;
  }
  .mark {
    width: 14px;
    height: 14px;
    border-radius: var(--r-xs, 4px);
    background: var(--red);
  }
  .body {
    display: flex;
    flex-direction: column;
    min-height: 0;
    overflow: hidden;
  }
  .macro {
    height: 100%;
    overflow: hidden;
  }
  /* Today's Library and editor keep their light look until they're redesigned. */
  .legacy {
    color: var(--color-text);
    background: var(--color-bg);
  }
</style>
