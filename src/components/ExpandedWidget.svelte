<script lang="ts">
  import Header from "./expanded/Header.svelte";
  import Preview from "./expanded/Preview.svelte";
  import SidePanel from "./expanded/SidePanel.svelte";
  import Transport from "./expanded/Transport.svelte";
  import Timeline from "./expanded/Timeline.svelte";
  import Splitter from "./ui/Splitter.svelte";
  import { relay } from "../lib/state/relay.svelte";
  import { MIN_PREVIEW_W, MIN_TIMELINE_H, MIN_TRANSPORT_H, SPLITTER, paneLayout, transportScale } from "../lib/layout";

  // The editor fills the window; the header keeps its height, and the
  // preview row, the button row and the timeline share the rest.
  let width = $state(0);
  let height = $state(0);
  let headerH = $state(0);
  const flexible = $derived(height && height - headerH - 2 * SPLITTER);
  const L = $derived(paneLayout(relay.panes, width, flexible));

  // The button row's controls scale with its height, as far as its width allows.
  let transportPane: HTMLDivElement | undefined = $state();
  let natural = $state(0);
  $effect(() => {
    void width;
    const t = transportPane?.querySelector<HTMLElement>(".transport");
    if (!t) return;
    const measure = () => {
      const cs = getComputedStyle(t);
      const items = [...t.children] as HTMLElement[];
      const gaps = parseFloat(cs.columnGap) * Math.max(0, items.length - 1) || 0;
      const padding = parseFloat(cs.paddingLeft) + parseFloat(cs.paddingRight) || 0;
      natural = Math.ceil(items.reduce((sum, el) => sum + el.offsetWidth, 0) + gaps + padding);
    };
    measure();
    document.fonts?.ready.then(measure);
  });
  const scale = $derived(transportScale(L.transportH, width, natural));
</script>

<div class="expanded" bind:clientWidth={width} bind:clientHeight={height}>
  <div bind:clientHeight={headerH}><Header /></div>
  <div class="main" style:grid-template-columns="{L.previewW}px {SPLITTER}px minmax(0, 1fr)">
    <Preview />
    <Splitter
      orientation="vertical"
      label="Resize preview"
      value={L.previewW}
      min={MIN_PREVIEW_W}
      max={L.previewMax}
      onchange={(v) => relay.movePanes({ preview_w: v })}
      oncommit={relay.savePanes}
      onreset={() => relay.resetPane("preview_w")}
    />
    <SidePanel />
  </div>
  <Splitter
    orientation="horizontal"
    label="Resize buttons"
    invert
    value={L.transportH}
    min={MIN_TRANSPORT_H}
    max={L.transportMax}
    onchange={(v) => relay.movePanes({ transport_h: v })}
    oncommit={relay.savePanes}
    onreset={() => relay.resetPane("transport_h")}
  />
  <div class="transport-pane" bind:this={transportPane} style:height="{L.transportH}px">
    <div
      class="transport-scale"
      style:width={width && scale !== 1 ? `${width / scale}px` : "100%"}
      style:transform={scale !== 1 ? `scale(${scale})` : undefined}
    >
      <Transport />
    </div>
  </div>
  <Splitter
    orientation="horizontal"
    label="Resize timeline"
    invert
    value={L.timelineH}
    min={MIN_TIMELINE_H}
    max={L.timelineMax}
    onchange={(v) => relay.movePanes({ timeline_h: v })}
    oncommit={relay.savePanes}
    onreset={() => relay.resetPane("timeline_h")}
  />
  <div class="timeline-pane" style:height="{L.timelineH}px"><Timeline /></div>
</div>

<style>
  .expanded {
    width: 100%;
    height: 100%;
    display: flex;
    flex-direction: column;
  }
  .main {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-rows: minmax(0, 1fr);
  }
  .transport-pane {
    flex: none;
    display: flex;
    align-items: center;
    overflow: hidden;
  }
  /* Laid out at 1/scale of the row's width, then scaled up to fill it. */
  .transport-scale {
    flex: none;
    transform-origin: left center;
  }
  .timeline-pane {
    flex: none;
    min-height: 0;
  }
</style>
