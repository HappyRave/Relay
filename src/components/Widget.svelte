<script lang="ts">
  import CompactBar from "./CompactBar.svelte";
  import ExpandedWidget from "./ExpandedWidget.svelte";
  import Toast from "./shared/Toast.svelte";
  import { relay } from "../lib/state/relay.svelte";
  import { fitEditor, fitWindow, isTauri } from "../lib/platform/window";

  let { onresize }: { onresize?: (w: number, h: number) => void } = $props();
  let el: HTMLDivElement | undefined = $state();

  // The compact player sizes the window to itself. The editor fills the
  // window, whose size is Rust's (the user resizes it), so it only says so.
  $effect(() => {
    if (!el) return;
    const ro = new ResizeObserver(([entry]) => {
      const box = entry.borderBoxSize[0];
      const w = Math.round(box.inlineSize);
      const h = Math.round(box.blockSize);
      onresize?.(w, h);
      if (isTauri() && !relay.expanded) fitWindow(w, h, false);
    });
    ro.observe(el);
    return () => ro.disconnect();
  });
  $effect(() => {
    if (relay.expanded && isTauri()) fitEditor();
  });
</script>

<div class="widget" class:fill={relay.expanded} bind:this={el}>
  {#if relay.expanded}<ExpandedWidget />{:else}<CompactBar /><Toast />{/if}
</div>

<style>
  .widget {
    display: inline-block;
    background: var(--color-bg);
    border: 2px solid var(--color-text);
    vertical-align: top;
  }
  /* The browser preview's demo desktop sets the size; the app's window is the widget. */
  .widget.fill {
    display: block;
    box-sizing: border-box;
    width: var(--widget-w, 100vw);
    height: var(--widget-h, 100vh);
  }
</style>
