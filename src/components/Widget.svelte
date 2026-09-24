<script lang="ts">
  import CompactBar from "./CompactBar.svelte";
  import ExpandedWidget from "./ExpandedWidget.svelte";
  import { relay } from "../lib/state/relay.svelte";
  import { fitWindow, isTauri } from "../lib/platform/window";

  let { onresize }: { onresize?: (w: number, h: number) => void } = $props();
  let el: HTMLDivElement | undefined = $state();

  $effect(() => {
    if (!el) return;
    relay.widgetEl = el;
    const ro = new ResizeObserver(([entry]) => {
      const box = entry.borderBoxSize[0];
      const w = Math.round(box.inlineSize);
      const h = Math.round(box.blockSize);
      onresize?.(w, h);
      if (isTauri()) fitWindow(w, h);
    });
    ro.observe(el);
    return () => {
      ro.disconnect();
      relay.widgetEl = null;
    };
  });
</script>

<div class="widget" bind:this={el}>
  {#if relay.expanded}<ExpandedWidget />{:else}<CompactBar />{/if}
</div>

<style>
  .widget {
    display: inline-block;
    background: var(--color-bg);
    border: 2px solid var(--color-text);
    vertical-align: top;
  }
</style>
