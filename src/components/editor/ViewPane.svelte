<script lang="ts">
  // The view, under the list: where and when. The preview of the screen, the
  // timeline, and a title block with the macro's measurements. It closes to
  // its bar, which then says what's under the playhead.
  import ViewPreview from "./view/ViewPreview.svelte";
  import ViewTimeline from "./view/ViewTimeline.svelte";
  import Key from "../kit/Key.svelte";
  import Lcd from "../kit/Lcd.svelte";
  import Choice from "../kit/Choice.svelte";
  import Icon from "../ui/Icon.svelte";
  import { relay } from "../../lib/state/relay.svelte";
  import { fmtTime } from "../../lib/format";
  import { rowOf, rowText, stepLabel, stepRows } from "../../lib/editor/rows";
  import type { Zoom } from "../../lib/preview/geometry";
  import type { PreviewBackground } from "../../lib/ipc/bindings/PreviewBackground";

  let zoom = $state<Zoom | null>(null);
  let stageW = $state(0);

  const cur = $derived(Math.min(relay.cur, relay.duration));
  const rows = $derived(stepRows(relay.steps, relay.settings.show_moves));
  const here = $derived(rows[rowOf(rows, relay.curStepIdx)]);
  const hereText = $derived.by(() => {
    if (!here) return "";
    const t = rowText(relay.steps[here.index], here.move == null ? null : relay.steps[here.move], { anchor: null });
    const label = stepLabel(relay.steps, here.index);
    return `${label[0].toUpperCase()}${label.slice(1)} · ${t.kind} · ${t.what}`;
  });
  const pointer = $derived(relay.steps.some((s) => s.kind !== "keys" && s.kind !== "type" && s.kind !== "text" && s.kind !== "wait"));
  const hasScreen = $derived(!!relay.screenUrl && relay.mode !== "recording");
  const desk = $derived(relay.desktop);
  const scale = $derived(stageW ? Math.round((desk.w / ((zoom?.scale ?? 1) * stageW)) * 10) / 10 : 0);
</script>

<section class="plate view" aria-label="View">
  <div class="head">
    <Key variant="ghost" size="sm" aria-expanded={relay.viewOpen} aria-controls="view-body" onclick={() => (relay.viewOpen = !relay.viewOpen)}>
      <span class="chev" class:open={relay.viewOpen}><Icon name="down" size={14} /></span><span class="caps">View</span>
    </Key>
    <Lcd label="Playhead at {fmtTime(cur)} of {fmtTime(relay.duration)}">
      <span>{fmtTime(cur)}</span><span class="of">/ {(relay.duration / 1000).toFixed(2)}</span>
    </Lcd>
    {#if relay.viewOpen && relay.mode !== "recording"}
      <span title={hasScreen ? undefined : "No screenshot: this macro was recorded without one"}>
        <Choice
          label="Draw over"
          options={[["screen", "Screen"], ["sketch", "Outline"]] as [PreviewBackground, string][]}
          value={hasScreen ? relay.settings.preview_background : "sketch"}
          disabled={!hasScreen}
          onchange={(preview_background) => relay.updateSettings({ preview_background })}
        />
      </span>
    {:else if !relay.viewOpen}
      <span class="here">{pointer ? hereText : "This macro uses only the keyboard, so there's nothing to draw."}</span>
    {/if}
    <span class="spacer"></span>
    <Key variant="ghost" size="sm" iconOnly aria-label="Previous step" title="Previous step (←)" disabled={relay.recording} onclick={() => relay.jump(-1)}><Icon name="prev" size={14} /></Key>
    <Key variant="ghost" size="sm" iconOnly aria-label="Next step" title="Next step (→)" disabled={relay.recording} onclick={() => relay.jump(1)}><Icon name="next" size={14} /></Key>
    {#if relay.viewOpen && zoom}
      <Key variant="ghost" size="sm" title="Show the whole screen (0)" onclick={() => (zoom = null)}>{Math.round(zoom.scale * 100)}% · Fit</Key>
    {/if}
  </div>
  {#if relay.viewOpen}
    <div class="body" id="view-body">
      <div class="preview" bind:clientWidth={stageW}><ViewPreview bind:zoom /></div>
      <ViewTimeline />
      <dl class="title-block" aria-label="Macro details">
        <dt>Steps</dt><dd class="num">{rows.length}</dd>
        <dt>Length</dt><dd class="num">{(relay.duration / 1000).toFixed(2)} s</dd>
        <dt>Screen</dt><dd class="num">{desk.w} × {desk.h}</dd>
        <dt>Scale</dt><dd class="num">{scale ? `1 : ${scale}` : "—"}</dd>
      </dl>
    </div>
  {/if}
</section>

<style>
  .view {
    display: flex;
    flex-direction: column;
    gap: 8px;
    min-height: 0;
    padding: 8px 10px 10px;
  }
  .head {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
  }
  .chev {
    display: inline-flex;
    transform: rotate(-90deg);
    transition: transform var(--t-release) var(--spring);
  }
  .chev.open {
    transform: none;
  }
  .head .caps {
    color: inherit;
  }
  .of {
    color: var(--alu-400);
    font-weight: 500;
  }
  .here {
    min-width: 0;
    overflow: hidden;
    font-size: 12px;
    white-space: nowrap;
    text-overflow: ellipsis;
    color: var(--text-2);
  }
  .spacer {
    flex: 1;
  }
  .body {
    display: grid;
    grid-template-columns: minmax(200px, 30%) minmax(0, 1fr) 150px;
    height: 150px;
    gap: 10px;
  }
  .preview {
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
  }
  .preview > :global(*) {
    flex: 1;
  }
  .title-block {
    display: grid;
    grid-template-columns: auto 1fr;
    align-content: start;
    margin: 0;
    overflow: hidden;
    border-radius: var(--r-md);
    box-shadow: 0 0 0 1px var(--line-strong);
  }
  .title-block dt,
  .title-block dd {
    margin: 0;
    padding: 4px 8px;
    overflow: hidden;
    font-size: 12px;
    white-space: nowrap;
    text-overflow: ellipsis;
    border-bottom: 1px solid var(--line);
  }
  .title-block dt {
    font-weight: 700;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    font-stretch: 88%;
    color: var(--text-2);
    border-right: 1px solid var(--line);
  }
  .title-block :nth-last-child(-n + 2) {
    border-bottom: 0;
  }
</style>
