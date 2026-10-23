<script lang="ts">
  import Widget from "./components/Widget.svelte";
  import ExportDialog from "./components/ExportDialog.svelte";
  import DevDesktop from "./components/dev/DevDesktop.svelte";
  import { relay } from "./lib/state/relay.svelte";
  import { onMount } from "svelte";
  import { isTauri } from "./lib/platform/window";
  import { applyLook, lookAttrs, SystemMedia } from "./lib/look.svelte";

  const tauri = isTauri();
  const media = new SystemMedia();

  $effect(() => media.watch());
  $effect(() => applyLook(document.documentElement, lookAttrs(relay.settings, relay.systemLook, media)));

  onMount(() => {
    relay.start();
    relay.init().catch((e) => console.error("Relay: couldn't start", e));
    return () => relay.dispose();
  });
</script>

{#if tauri}
  {#if relay.ready}<Widget />{/if}
{:else}
  <DevDesktop />
{/if}
{#if relay.exportOpen}
  <ExportDialog />
{/if}
