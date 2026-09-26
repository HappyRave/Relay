<script lang="ts">
  import Widget from "./components/Widget.svelte";
  import ExportDialog from "./components/ExportDialog.svelte";
  import DevDesktop from "./components/dev/DevDesktop.svelte";
  import { relay } from "./lib/state/relay.svelte";
  import { onMount } from "svelte";
  import { isTauri } from "./lib/platform/window";

  const tauri = isTauri();

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
