<script lang="ts">
  // Relay's window and its three places: the Library, a macro, and Settings.
  import MacroPage from "../editor/MacroPage.svelte";
  import LibraryPage from "../library/LibraryPage.svelte";
  import SettingsPage from "../settings/SettingsPage.svelte";
  import FirstRun from "./FirstRun.svelte";
  import { relay } from "../../lib/state/relay.svelte";
</script>

<div class="app-window" data-place={relay.showsIntro ? "first-run" : relay.place}>
  {#if relay.showsIntro}
    <FirstRun />
  {:else if relay.place === "library"}
    <LibraryPage />
  {:else if relay.place === "settings"}
    <SettingsPage />
  {:else}
    <MacroPage />
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
</style>
