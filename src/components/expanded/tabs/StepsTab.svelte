<script lang="ts">
  import Icon from "../../ui/Icon.svelte";
  import { relay } from "../../../lib/state/relay.svelte";
  import { currentStepIndex } from "../../../lib/timeline/lanes";
  import { fmtTime } from "../../../lib/format";
  import type { Step } from "../../../lib/types";

  const TYPE_NAME: Record<Step["kind"], string> = { click: "CLICK", keys: "KEYS", type: "TYPE", wait: "WAIT", pixel: "IF" };

  const steps = $derived(relay.view.steps);
  const cur = $derived(Math.min(relay.cur, relay.duration));
  const curIdx = $derived(currentStepIndex(steps, cur));

  function describe(s: Step): [string, string] {
    switch (s.kind) {
      case "click": {
        const what = s.btn === "Left" ? (s.count === 2 ? "Double click" : "Click") : s.btn + " click";
        const anchor = relay.view.frames[0];
        const sub =
          relay.playback.coordMode === "window" && anchor
            ? `+${Math.round(s.x - anchor.x)}, +${Math.round(s.y - anchor.y)} in window`
            : `${s.x}, ${s.y} px`;
        return [what + (s.label ? " · " + s.label : ""), sub];
      }
      case "keys":
        return [s.combo, "Key combination"];
      case "type":
        return ["“" + s.text + "”", s.text.length + " characters"];
      case "pixel":
        return [
          `Wait for pixel ${s.x}, ${s.y} = ${s.color}`,
          (s.label ? s.label + " · " : "") + `timeout ${s.timeoutMs / 1000} s, else stop`,
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
  <span class="count">{steps.length} steps · {(relay.duration / 1000).toFixed(1)} s</span>
  <button class="btn btn-ghost" disabled={relay.recording} onclick={relay.insertWait}>+ Wait</button>
  <button
    class="btn btn-ghost"
    disabled={relay.recording}
    title="Wait until the pixel under the cursor matches"
    onclick={relay.insertPixelCheck}>+ Pixel check</button
  >
</div>
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
        disabled={relay.recording}
        onclick={(e) => {
          e.stopPropagation();
          relay.deleteStep(s);
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
