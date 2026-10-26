<script lang="ts">
  import AppWindow from "./components/window/AppWindow.svelte";
  import Player from "./components/player/Player.svelte";
  import ExportDialog from "./components/ExportDialog.svelte";
  import DevDesktop from "./components/dev/DevDesktop.svelte";
  import { relay } from "./lib/state/relay.svelte";
  import { onMount } from "svelte";
  import { isTauri, windowLabel } from "./lib/platform/window";
  import { applyLook, lookAttrs, SystemMedia } from "./lib/look.svelte";

  const tauri = isTauri();
  const media = new SystemMedia();
  // Both of Relay's windows load this page: the main window, and the player.
  relay.role = windowLabel();

  $effect(() => media.watch());
  $effect(() => applyLook(document.documentElement, lookAttrs(relay.settings, relay.systemLook, media)));

  onMount(() => {
    relay.start();
    relay.init().catch((e) => console.error("Relay: couldn't start", e));
    return () => relay.dispose();
  });
</script>

{#if !tauri}
  <DevDesktop />
{:else if relay.ready && relay.role === "player"}
  <Player />
{:else if relay.ready}
  <div class="window"><AppWindow /></div>
{/if}
{#if relay.exportOpen}
  <ExportDialog />
{/if}

<style>
  .window {
    width: 100vw;
    height: 100vh;
  }
</style>
