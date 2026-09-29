<script lang="ts">
  import Header from "./expanded/Header.svelte";
  import Preview from "./expanded/Preview.svelte";
  import SidePanel from "./expanded/SidePanel.svelte";
  import Transport from "./expanded/Transport.svelte";
  import Timeline from "./expanded/Timeline.svelte";
  import Splitter from "./ui/Splitter.svelte";
  import { relay } from "../lib/state/relay.svelte";
  import { MIN_PREVIEW_W, MIN_TIMELINE_H, SPLITTER, paneLayout } from "../lib/layout";

  // The editor fills the window; the header and the transport keep their
  // height, and the preview row and the timeline share the rest.
  let width = $state(0);
  let height = $state(0);
  let headerH = $state(0);
  let transportH = $state(0);
  const flexible = $derived(height && height - headerH - transportH - SPLITTER);
  const L = $derived(paneLayout(relay.panes, width, flexible));
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
  <div bind:clientHeight={transportH}><Transport /></div>
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
    border-bottom: 2px solid var(--color-divider);
  }
  .timeline-pane {
    flex: none;
    min-height: 0;
  }
</style>
