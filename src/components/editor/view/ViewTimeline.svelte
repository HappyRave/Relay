<script lang="ts">
  // The timeline: a lane per kind of step (the pointer, the keyboard, logic)
  // under a dimension line with the macro's length, and the playhead. It's a
  // slider: a click or a drag moves the playhead, the arrow keys go to the
  // previous or next step (a tenth of a second with Shift), Home and End to
  // the ends, and Space plays or pauses.
  import Glyph from "../../kit/Glyph.svelte";
  import { relay } from "../../../lib/state/relay.svelte";
  import { seekable } from "../../../lib/actions/seekable";
  import { keyChips, moveBars, moveSegments, pauseSpans, pct } from "../../../lib/timeline/lanes";
  import { fmtTime } from "../../../lib/format";
  import { rowOf, rowText, stepCount, stepLabel, stepRows } from "../../../lib/editor/rows";
  import type { Step } from "../../../lib/types";

  const d = $derived(relay.duration);
  const cur = $derived(Math.min(relay.cur, d));
  const steps = $derived(relay.steps);
  const recording = $derived(relay.mode === "recording");
  const moves = $derived(recording ? moveSegments(relay.moves, d) : moveBars(steps, d));
  const pauses = $derived(recording ? [] : pauseSpans(steps, d));
  const POINTS: Step["kind"][] = ["click", "scroll", "drag"];
  const points = $derived(steps.filter((s) => POINTS.includes(s.kind)));
  const chips = $derived(keyChips(steps, d));
  const LOGIC: Partial<Record<Step["kind"], string>> = { wait: "Wait", pixel_wait: "Wait for", find_image: "Find" };
  const logic = $derived(
    steps.filter((s) => s.kind in LOGIC).map((s) => ({ l: pct(s.t, d), w: Math.max(0.8, pct(s.end, d) - pct(s.t, d)), label: LOGIC[s.kind]! })),
  );

  let width = $state(0);
  /** Whether a label fits in a block `w` percent wide. */
  const fits = (w: number, label: string) => (w / 100) * width - 10 >= Math.min(label.length * 7, 70);

  const rows = $derived(stepRows(steps, relay.settings.show_moves));
  const here = $derived(rows[rowOf(rows, relay.curStepIdx)]);
  const valueText = $derived.by(() => {
    const at = `${fmtTime(cur)} of ${fmtTime(d)}`;
    if (!here) return at;
    const t = rowText(steps[here.index], here.move == null ? null : steps[here.move], { anchor: null });
    const of = steps[here.index].kind === "move" ? "" : ` of ${stepCount(steps)}`;
    return `${at}, ${stepLabel(steps, here.index)}${of}, ${t.kind} · ${t.what}`;
  });

  function key(e: KeyboardEvent) {
    const move = { ArrowLeft: -1, ArrowRight: 1, ArrowDown: -1, ArrowUp: 1 }[e.key] as -1 | 1 | undefined;
    if (move) {
      e.preventDefault();
      if (e.shiftKey) relay.seek(relay.cur + move * 100);
      else relay.jump(move);
    } else if (e.key === "Home" || e.key === "End") {
      e.preventDefault();
      relay.seek(e.key === "Home" ? 0 : d);
    } else if (e.key === " ") {
      e.preventDefault();
      relay.togglePlay();
    }
  }
</script>

<div
  class="timeline"
  role="slider"
  tabindex="0"
  aria-label="Playhead"
  aria-valuemin={0}
  aria-valuemax={Math.round(d) / 1000}
  aria-valuenow={Math.round(cur) / 1000}
  aria-valuetext={valueText}
  bind:clientWidth={width}
  onkeydown={key}
  use:seekable
>
  <div class="dimension"><span class="num">{(d / 1000).toFixed(2)} s</span></div>
  <div class="lane">
    <span class="tag"><Glyph category="pointer" small /></span>
    {#each pauses as p, i (i)}<div class="blk hatch" style:left="{p.l}%" style:width="{p.w}%"></div>{/each}
    {#each moves as m, i (i)}<div class="blk move" style:left="{m.l}%" style:width="{m.w}%"></div>{/each}
    {#each points as s (s.items[0])}<div class="blk point" class:unplayed={s.t > cur} style:left="{pct(s.t, d)}%"></div>{/each}
  </div>
  <div class="lane">
    <span class="tag"><Glyph category="keys" small /></span>
    {#each chips as c (c.t)}
      <div class="blk keys" title={c.label} style:left="{c.l}%" style:width="{c.w}%">{#if fits(c.w, c.label)}<span>{c.label}</span>{/if}</div>
    {/each}
  </div>
  <div class="lane">
    <span class="tag"><Glyph category="logic" small /></span>
    {#each logic as b, i (i)}
      <div class="blk logic" title={b.label} style:left="{b.l}%" style:width="{b.w}%">{#if fits(b.w, b.label)}<span>{b.label}</span>{/if}</div>
    {/each}
  </div>
  <div class="playhead" style:left="{pct(cur, d)}%"></div>
</div>

<style>
  .timeline {
    position: relative;
    display: grid;
    grid-template-rows: 18px repeat(3, minmax(0, 1fr));
    gap: 6px;
    min-width: 0;
    min-height: 0;
    cursor: pointer;
    touch-action: none;
  }
  .timeline:focus-visible {
    outline: 2px solid var(--text);
    outline-offset: 3px;
    border-radius: var(--r-sm);
  }
  .dimension {
    position: relative;
    align-self: center;
    height: 1px;
    background: var(--text-2);
  }
  .dimension::before,
  .dimension::after {
    content: "";
    position: absolute;
    top: -4px;
    width: 1px;
    height: 9px;
    background: var(--text-2);
  }
  .dimension::before {
    left: 0;
  }
  .dimension::after {
    right: 0;
  }
  .dimension span {
    position: absolute;
    left: 50%;
    top: -9px;
    padding: 0 6px;
    font-size: 12px;
    font-weight: 700;
    white-space: nowrap;
    transform: translateX(-50%);
    background: var(--plate);
  }
  .lane {
    position: relative;
    min-height: 0;
    overflow: hidden;
    border-radius: var(--r-sm);
    background: var(--plate-2);
    box-shadow: inset 0 0 0 1px var(--line);
  }
  .tag {
    position: absolute;
    left: 5px;
    top: 50%;
    z-index: 2;
    display: flex;
    transform: translateY(-50%);
  }
  .blk {
    position: absolute;
    top: 3px;
    bottom: 3px;
    display: flex;
    align-items: center;
    overflow: hidden;
    border-radius: 4px;
    font-size: 12px;
    font-weight: 700;
    white-space: nowrap;
  }
  .blk span {
    min-width: 0;
    padding: 0 5px;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .move {
    background: var(--blue-soft);
  }
  .point {
    width: 4px;
    margin-left: -2px;
    background: var(--blue);
  }
  .point.unplayed {
    width: 6px;
    margin-left: -3px;
    background: none;
    box-shadow: inset 0 0 0 1.5px var(--blue);
  }
  .keys {
    color: var(--ink);
    background: var(--yellow);
  }
  .logic {
    color: var(--ink);
    background: var(--green);
  }
  .hatch {
    background: repeating-linear-gradient(-45deg, transparent 0 4px, var(--line-strong) 4px 5px);
  }
  .playhead {
    position: absolute;
    top: 0;
    bottom: 0;
    z-index: 3;
    width: 2px;
    margin-left: -1px;
    background: var(--red);
    pointer-events: none;
  }
  .playhead::before {
    content: "";
    position: absolute;
    top: 3px;
    left: -5px;
    width: 12px;
    height: 12px;
    border-radius: 3px;
    background: var(--red);
  }
  @media (forced-colors: active) {
    .playhead,
    .playhead::before {
      background: Highlight;
    }
  }
</style>
