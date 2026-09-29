<script lang="ts">
  import { relay } from "../../lib/state/relay.svelte";
  import { keyChips, moveSegments, pct, ruler, startedCount } from "../../lib/timeline/lanes";
  import { seekable } from "../../lib/actions/seekable";
  import type { StepOf } from "../../lib/types";

  const d = $derived(relay.duration);
  const cur = $derived(Math.min(relay.cur, d));
  const steps = $derived(relay.steps);
  // The ruler's width picks its step, so labels never crowd on a narrow timeline.
  let rulerW = $state(0);
  const ticks = $derived(ruler(d, rulerW));
  const moves = $derived(moveSegments(relay.moves, d));
  const clicks = $derived(steps.filter((s): s is StepOf<"click"> => s.kind === "click"));
  const chips = $derived(keyChips(steps, d));
  // How many of each have been reached: changes only when the playhead passes one.
  const pastClicks = $derived(startedCount(clicks, cur));
  const pastChips = $derived(startedCount(chips, cur));
  const waits = $derived(steps.filter((s): s is StepOf<"wait"> => s.kind === "wait"));
  const conds = $derived(steps.filter((s): s is StepOf<"pixel_wait"> => s.kind === "pixel_wait"));
</script>

<div class="timeline">
  <div></div>
  <div class="ruler" bind:clientWidth={rulerW}>
    {#each ticks as r (r.label)}
      <div class="tick" style:left="{r.l}%">{r.label}</div>
    {/each}
  </div>
  <div class="lane-label">Mouse</div>
  <div class="lanes" use:seekable>
    <div class="lane" style:border-top-width="2px">
      {#each moves as s, i (i)}
        <div class="move" style:left="{s.l}%" style:width="{s.w}%"></div>
      {/each}
    </div>
    <div class="lane">
      {#each clicks as c, i (c.items[0])}
        <div class="click" class:past={i < pastClicks} style:left="{pct(c.t, d)}%"></div>
      {/each}
    </div>
    <div class="lane">
      {#each chips as k, i (i)}
        <div class="chip" class:past={i < pastChips} style:left="{k.l}%" style:width="{k.w}%" title={k.label}><span>{k.label}</span></div>
      {/each}
    </div>
    <div class="lane last">
      {#each waits as w (w.items[0])}
        <div class="wait" style:left="{pct(w.t, d)}%" style:width="{pct(w.t + w.dur, d) - pct(w.t, d)}%"></div>
      {/each}
      {#each conds as c (c.items[0])}
        <div class="cond" class:past={c.t <= cur} style:left="{pct(c.t, d)}%" style:width="{pct(c.t + c.dur, d) - pct(c.t, d)}%">
          IF
        </div>
      {/each}
    </div>
    <div class="playhead" style:left="{pct(cur, d)}%"><div class="head"></div></div>
  </div>
  <div class="lane-label">Clicks</div>
  <div class="lane-label">Keys</div>
  <div class="lane-label">Logic</div>
</div>

<style>
  /* A taller timeline pane gives the four lanes more height each (never under 26 px). */
  .timeline {
    box-sizing: border-box;
    height: 100%;
    display: grid;
    grid-template-columns: 72px minmax(0, 1fr);
    grid-template-rows: 18px repeat(4, minmax(26px, 1fr));
    padding: 10px 16px 14px;
    gap: 0 10px;
  }
  .ruler {
    position: relative;
    height: 18px;
  }
  .tick {
    position: absolute;
    top: 0;
    bottom: 0;
    border-left: 1px solid var(--color-neutral-500);
    padding-left: 3px;
    font-size: 10px;
    color: var(--color-neutral-600);
    font-variant-numeric: tabular-nums;
  }
  .lane-label {
    font-size: 10px;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    font-weight: 600;
    display: flex;
    align-items: center;
  }
  .lanes {
    grid-row: 2 / span 4;
    grid-column: 2;
    position: relative;
    display: grid;
    grid-template-rows: repeat(4, minmax(0, 1fr));
    cursor: ew-resize;
  }
  .lane {
    position: relative;
    border-top: 1px solid var(--color-divider);
  }
  /* Marks sit in the middle of their lane and grow with it. */
  .move,
  .click,
  .chip,
  .wait,
  .cond {
    top: 50%;
    transform: translateY(-50%);
  }
  .lane.last {
    border-bottom: 2px solid var(--color-divider);
  }
  .move {
    position: absolute;
    height: 38%;
    background: var(--color-neutral-400);
  }
  .click {
    position: absolute;
    height: calc(100% - 8px);
    width: 4px;
    margin-left: -2px;
    background: var(--color-text);
  }
  .click.past {
    background: var(--color-accent);
  }
  .chip {
    position: absolute;
    box-sizing: border-box;
    height: calc(100% - 6px);
    display: flex;
    align-items: center;
    padding: 0 4px;
    background: var(--color-bg);
    color: var(--color-text);
    border: 1px solid var(--color-text);
    font-size: 10px;
    font-weight: 800;
    overflow: hidden;
  }
  .chip span {
    min-width: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .chip.past {
    background: var(--color-text);
    color: var(--color-bg);
  }
  .wait {
    position: absolute;
    box-sizing: border-box;
    height: calc(100% - 10px);
    background: repeating-linear-gradient(135deg, var(--color-neutral-500) 0 2px, transparent 2px 6px);
    border: 1px solid var(--color-neutral-500);
  }
  .cond {
    position: absolute;
    box-sizing: border-box;
    height: calc(100% - 8px);
    border: 2px solid var(--color-accent);
    background: transparent;
    color: var(--color-accent-700);
    font-size: 9px;
    font-weight: 800;
    display: flex;
    align-items: center;
    padding-left: 3px;
    overflow: hidden;
  }
  .cond.past {
    background: var(--color-accent);
    color: var(--color-bg);
  }
  .playhead {
    position: absolute;
    top: -6px;
    bottom: 0;
    width: 2px;
    margin-left: -1px;
    background: var(--color-accent);
    pointer-events: none;
  }
  .head {
    position: absolute;
    top: 0;
    left: -5px;
    width: 12px;
    height: 8px;
    background: var(--color-accent);
  }
</style>
