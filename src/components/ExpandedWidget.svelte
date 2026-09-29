<script lang="ts">
  import Header from "./expanded/Header.svelte";
  import Preview from "./expanded/Preview.svelte";
  import SidePanel from "./expanded/SidePanel.svelte";
  import Transport from "./expanded/Transport.svelte";
  import Timeline from "./expanded/Timeline.svelte";
  import Splitter from "./ui/Splitter.svelte";
  import { relay } from "../lib/state/relay.svelte";
  import { MIN_PREVIEW_W, MIN_TIMELINE_H, MIN_TRANSPORT_H, SPLITTER, paneLayout } from "../lib/layout";
  import { barMode, barScale } from "../lib/transport";

  // The editor fills the window; the header keeps its height, and the
  // preview row, the button row and the timeline share the rest.
  let width = $state(0);
  let height = $state(0);
  let headerH = $state(0);
  const flexible = $derived(height && height - headerH - 2 * SPLITTER);
  const L = $derived(paneLayout(relay.panes, width, flexible));

  // The control bar scales evenly with its row's height (every control is one
  // height, so nothing drifts), and compacts its settings for the width it has
  // at that scale.
  const scale = $derived(barScale(L.transportH, width));
  const mode = $derived(barMode(width / scale));
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
  <div class="transport-pane" style:height="{L.transportH}px">
    <div
      class="transport-scale"
      style:width={width && scale !== 1 ? `${width / scale}px` : "100%"}
      style:transform={scale !== 1 ? `scale(${scale})` : undefined}
    >
      <Transport {mode} />
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
