<script lang="ts">
  import Icon from "../../ui/Icon.svelte";
  import { relay } from "../../../lib/state/relay.svelte";
  import { currentStepIndex } from "../../../lib/timeline/lanes";
  import { fmtTime, plural } from "../../../lib/format";
  import type { Step } from "../../../lib/types";

  const TYPE_NAME: Record<Step["kind"], string> = {
    click: "CLICK",
    drag: "DRAG",
    scroll: "SCROLL",
    keys: "KEYS",
    type: "TYPE",
    wait: "WAIT",
    pixel_wait: "IF",
  };
  const COUNT_NAME = ["", "Click", "Double click", "Triple click"];

  const steps = $derived(relay.steps);
  const cur = $derived(Math.min(relay.cur, relay.duration));
  const curIdx = $derived(currentStepIndex(steps, cur));
  const anchor = $derived(relay.view?.recording.anchor_window?.rect);

  function where(x: number, y: number): string {
    return relay.playback.coord_mode === "window" && anchor
      ? `+${x - anchor.x}, +${y - anchor.y} in window`
      : `${x}, ${y} px`;
  }

  function describe(s: Step): [string, string] {
    switch (s.kind) {
      case "click": {
        const what = s.btn === "Left" ? (COUNT_NAME[s.count] ?? `${s.count}× click`) : `${s.btn} click`;
        return [what + (s.label ? " · " + s.label : ""), where(s.x, s.y)];
      }
      case "drag":
        return [`${s.btn === "Left" ? "Drag" : s.btn + " drag"}${s.label ? " · " + s.label : ""}`, `${where(s.x, s.y)} → ${s.to_x}, ${s.to_y}`];
      case "scroll":
        return [`Scroll ${s.horizontal ? (s.delta > 0 ? "right" : "left") : s.delta > 0 ? "up" : "down"}`, `${Math.abs(s.delta) / 120} notches at ${where(s.x, s.y)}`];
      case "keys":
        return [s.combo.join(" + "), "Key combination"];
      case "type":
        return ["“" + s.text + "”", plural(s.text.length, "character")];
      case "pixel_wait":
        return [
          `Wait for pixel ${s.x}, ${s.y} = ${s.color}`,
          (s.label ? s.label + " · " : "") + `timeout ${s.timeout_ms / 1000} s, else stop`,
        ];
      case "wait":
        return ["Wait " + (s.dur / 1000).toFixed(1) + " s", s.label];
    }
  }

  let list: HTMLDivElement | undefined = $state();
  // Keep the active row in view while playing.
  $effect(() => {
    if (relay.mode !== "play" || curIdx < 0 || !list) return;
    list.children[curIdx]?.scrollIntoView({ block: "nearest" });
  });
</script>

<div class="bar">
  <span class="count">{plural(steps.length, "step")} · {(relay.duration / 1000).toFixed(1)} s</span>
  <button class="btn btn-ghost" disabled={relay.recording || !relay.editable} onclick={relay.insertWait}>+ Wait</button>
  <button
    class="btn btn-ghost"
    disabled={relay.recording || !relay.editable}
    title="Wait until the pixel under the cursor matches"
    onclick={relay.insertPixelCheck}>+ Pixel check</button
  >
</div>
{#if relay.error}<div class="error" role="alert">{relay.error}</div>{/if}
<div class="list" bind:this={list}>
  {#each steps as s, i (s.items[0] ?? i)}
    {@const [detail, sub] = describe(s)}
    <div
      class="row"
      class:active={i === curIdx}
      class:future={s.t > cur}
      role="button"
      tabindex="0"
      onclick={() => relay.seek(s.t)}
      onkeydown={(e) => e.key === "Enter" && relay.seek(s.t)}
    >
      <span class="time">{fmtTime(s.t)}</span>
      <span class="type">{TYPE_NAME[s.kind]}</span>
      <div class="text">
        <div class="detail">{detail}</div>
        <div class="sub">{sub}</div>
      </div>
      <button
        class="del"
        title="Delete step"
        aria-label="Delete step"
        disabled={relay.recording || !relay.editable}
        onclick={(e) => {
          e.stopPropagation();
          relay.deleteStep(i);
        }}><Icon name="x" size={14} /></button
      >
    </div>
  {/each}
</div>

<style>
  .bar {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    border-bottom: 1px solid var(--color-divider);
  }
  .count {
    font-size: 12px;
    color: var(--color-neutral-700);
    margin-right: auto;
  }
  .bar .btn {
    font-size: 12px;
    padding: 4px 6px;
  }
  .error {
    padding: 6px 12px;
    font-size: 12px;
    background: var(--color-accent-100);
    color: var(--color-accent-800);
    border-bottom: 1px solid var(--color-divider);
  }
  .list {
    flex: 1;
    overflow-y: auto;
  }
  .row {
    display: grid;
    grid-template-columns: 58px 44px minmax(0, 1fr) 28px;
    align-items: center;
    gap: 8px;
    padding: 6px 0 6px 12px;
    border-bottom: 1px solid var(--color-neutral-300);
    cursor: pointer;
    color: var(--color-text);
  }
  .row:hover {
    background: var(--color-neutral-200);
  }
  .row.future {
    color: var(--color-neutral-600);
  }
  .row.active {
    background: var(--color-accent-100);
  }
  .row:focus-visible {
    outline-offset: -2px;
  }
  .time {
    font-size: 11px;
    font-variant-numeric: tabular-nums;
  }
  .type {
    font-size: 9px;
    letter-spacing: 0.08em;
    font-weight: 800;
    color: var(--color-neutral-700);
  }
  .active .type {
    color: var(--color-accent-700);
  }
  .text {
    min-width: 0;
  }
  .detail {
    font-size: 13px;
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .sub {
    font-size: 11px;
    color: var(--color-neutral-600);
  }
  .del {
    width: 24px;
    height: 24px;
    border: 0;
    background: transparent;
    color: var(--color-neutral-600);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0;
  }
  .del:hover {
    color: var(--color-accent);
  }
  .del:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }
</style>
