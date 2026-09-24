<script lang="ts">
  import Widget from "./components/Widget.svelte";
  import ExportDialog from "./components/ExportDialog.svelte";
  import DevDesktop from "./components/dev/DevDesktop.svelte";
  import { relay } from "./lib/state/relay.svelte";
  import { isTauri } from "./lib/platform/window";

  const tauri = isTauri();

  $effect(() => {
    relay.start();
    relay.init();
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
