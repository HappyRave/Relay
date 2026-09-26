<script lang="ts">
  import CompactBar from "./CompactBar.svelte";
  import ExpandedWidget from "./ExpandedWidget.svelte";
  import Toast from "./shared/Toast.svelte";
  import { relay } from "../lib/state/relay.svelte";
  import { fitWindow, isTauri } from "../lib/platform/window";

  let { onresize }: { onresize?: (w: number, h: number) => void } = $props();
  let el: HTMLDivElement | undefined = $state();

  $effect(() => {
    if (!el) return;
    const ro = new ResizeObserver(([entry]) => {
      const box = entry.borderBoxSize[0];
      const w = Math.round(box.inlineSize);
      const h = Math.round(box.blockSize);
      onresize?.(w, h);
      if (isTauri()) fitWindow(w, h, relay.expanded);
    });
    ro.observe(el);
    return () => ro.disconnect();
  });
</script>

<div class="widget" bind:this={el}>
  {#if relay.expanded}<ExpandedWidget />{:else}<CompactBar /><Toast />{/if}
</div>

<style>
  .widget {
    display: inline-block;
    background: var(--color-bg);
    border: 2px solid var(--color-text);
    vertical-align: top;
  }
</style>
