<script lang="ts">
  // The view: where and when, the preview of the screen and the timeline,
  // under the list. It closes to its bar.
  import Icon from "../ui/Icon.svelte";
  import Preview from "../expanded/Preview.svelte";
  import Timeline from "../expanded/Timeline.svelte";
  import { relay } from "../../lib/state/relay.svelte";
  import { fmtTime } from "../../lib/format";
</script>

<section class="plate view" class:open={relay.viewOpen} aria-label="View">
  <div class="bar">
    <button type="button" class="toggle caps" aria-expanded={relay.viewOpen} aria-controls="view-body" onclick={() => (relay.viewOpen = !relay.viewOpen)}>
      <Icon name="down" size={14} />View
    </button>
    <span class="time num" aria-label="Playhead {fmtTime(relay.cur)} of {fmtTime(relay.duration)}">
      {fmtTime(Math.min(relay.cur, relay.duration))} <span class="of">/ {fmtTime(relay.duration)}</span>
    </span>
  </div>
  {#if relay.viewOpen}
    <div class="body legacy" id="view-body">
      <div class="preview"><Preview /></div>
      <div class="timeline"><Timeline /></div>
    </div>
  {/if}
</section>

<style>
  .view {
    display: flex;
    flex-direction: column;
    min-height: 0;
    overflow: hidden;
  }
  .bar {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 6px 10px;
  }
  .toggle {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 24px;
    padding: 0 6px;
    border: 0;
    border-radius: var(--r-sm);
    background: none;
    cursor: pointer;
  }
  .toggle:hover {
    background: var(--hover);
  }
  .toggle :global(svg) {
    transition: transform var(--t-release) var(--spring);
  }
  .toggle[aria-expanded="false"] :global(svg) {
    transform: rotate(-90deg);
  }
  .time {
    font-weight: 650;
  }
  .of {
    color: var(--text-2);
  }
  .body {
    display: grid;
    grid-template-columns: 280px minmax(0, 1fr);
    height: 210px;
    border-top: 1px solid var(--line);
  }
  .legacy {
    color: var(--color-text);
    background: var(--color-bg);
  }
  .preview,
  .timeline {
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
</style>
