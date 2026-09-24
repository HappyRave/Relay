<script lang="ts">
  import Widget from "./components/Widget.svelte";
  import ExportDialog from "./components/ExportDialog.svelte";
  import DevDesktop from "./components/dev/DevDesktop.svelte";
  import { relay } from "./lib/state/relay.svelte";
  import { isTauri } from "./lib/platform/window";

  const tauri = isTauri();

  $effect(() => {
    relay.start();
    return () => relay.dispose();
  });
</script>

{#if tauri}
  <Widget />
{:else}
  <DevDesktop />
{/if}
{#if relay.exportOpen}
  <ExportDialog />
{/if}
