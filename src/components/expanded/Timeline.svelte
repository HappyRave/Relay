<script lang="ts">
  import { relay } from "../../lib/state/relay.svelte";
  import { keyChips, moveSegments, pct, ruler } from "../../lib/timeline/lanes";
  import { seekable } from "../../lib/actions/seekable";

  const d = $derived(relay.duration);
  const cur = $derived(Math.min(relay.cur, d));
  const steps = $derived(relay.view.steps);
  const ticks = $derived(ruler(d));
  const moves = $derived(moveSegments(relay.view.moves, d));
  const clicks = $derived(steps.filter((s) => s.kind === "click"));
  const chips = $derived(keyChips(steps, d, cur));
  const waits = $derived(steps.filter((s) => s.kind === "wait"));
  const conds = $derived(steps.filter((s) => s.kind === "pixel"));
</script>

<div class="timeline">
  <div></div>
  <div class="ruler">
    {#each ticks as r (r.label)}
      <div class="tick" style:left="{r.l}%">{r.label}</div>
    {/each}
  </div>
  <div class="lane-label">Mouse</div>
  <div class="lanes" use:seekable>
    <div class="lane" style:top="0" style:border-top-width="2px">
      {#each moves as s, i (i)}
        <div class="move" style:left="{s.l}%" style:width="{s.w}%"></div>
      {/each}
    </div>
    <div class="lane" style:top="26px">
      {#each clicks as c (c.items[0])}
        <div class="click" class:past={c.t <= cur} style:left="{pct(c.t, d)}%"></div>
      {/each}
    </div>
    <div class="lane" style:top="52px">
      {#each chips as k, i (i)}
        <div class="chip" class:past={k.past} style:left="{k.l}%" style:width="{k.w}%" title={k.label}>{k.label}</div>
      {/each}
    </div>
    <div class="lane last" style:top="78px">
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
  .timeline {
    display: grid;
    grid-template-columns: 72px 1fr;
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
    height: 26px;
  }
  .lanes {
    grid-row: 2 / span 4;
    grid-column: 2;
    position: relative;
    cursor: ew-resize;
  }
  .lane {
    position: absolute;
    left: 0;
    right: 0;
    height: 26px;
    border-top: 1px solid var(--color-divider);
  }
  .lane.last {
    border-bottom: 2px solid var(--color-divider);
  }
  .move {
    position: absolute;
    top: 8px;
    height: 10px;
    background: var(--color-neutral-400);
  }
  .click {
    position: absolute;
    top: 4px;
    height: 18px;
    width: 4px;
    margin-left: -2px;
    background: var(--color-text);
  }
  .click.past {
    background: var(--color-accent);
  }
  .chip {
    position: absolute;
    top: 3px;
    height: 20px;
    padding: 0 4px;
    background: var(--color-bg);
    color: var(--color-text);
    border: 1px solid var(--color-text);
    font-size: 10px;
    font-weight: 800;
    line-height: 18px;
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
    top: 5px;
    height: 16px;
    background: repeating-linear-gradient(135deg, var(--color-neutral-500) 0 2px, transparent 2px 6px);
    border: 1px solid var(--color-neutral-500);
  }
  .cond {
    position: absolute;
    top: 4px;
    height: 18px;
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
