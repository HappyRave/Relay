<script lang="ts">
  import RecPlayButtons from "../shared/RecPlayButtons.svelte";
  import Segmented from "../ui/Segmented.svelte";
  import Icon from "../ui/Icon.svelte";
  import { relay } from "../../lib/state/relay.svelte";
  import { fmtTime } from "../../lib/format";

  const SPEEDS: [number, string][] = [
    [0.5, "0.5×"],
    [1, "1×"],
    [2, "2×"],
    [4, "4×"],
  ];
  const pb = $derived(relay.playback);
  const infinite = $derived(pb.repeat === "forever");
  /** The count to return to when leaving "forever" (remembered when entering it). */
  let lastCount = $state(1);
  const count = $derived(pb.repeat === "forever" ? lastCount : pb.repeat.count);

  function toggleForever() {
    if (pb.repeat === "forever") relay.setPlayback({ repeat: { count: lastCount } });
    else {
      lastCount = pb.repeat.count;
      relay.setPlayback({ repeat: "forever" });
    }
  }
  const cur = $derived(Math.min(relay.cur, relay.duration));
</script>

<div class="transport">
  <div class="clock">
    <div class="big">{fmtTime(cur)}</div>
    <div class="of">of {relay.mode === "recording" ? "recording" : fmtTime(relay.duration)}</div>
  </div>
  <div class="buttons">
    <button class="btn btn-icon btn-secondary" title="Previous step" aria-label="Previous step" onclick={() => relay.jump(-1)}>
      <Icon name="prev" />
    </button>
    <RecPlayButtons variant="square" />
    <button class="btn btn-icon btn-secondary" title="Stop (Esc)" aria-label="Stop" onclick={relay.stop}>
      <Icon name="stop" size={14} />
    </button>
    <button class="btn btn-icon btn-secondary" title="Next step" aria-label="Next step" onclick={() => relay.jump(1)}>
      <Icon name="next" />
    </button>
  </div>
  <div class="group push">
    <span class="label">Speed</span>
    <Segmented label="Speed" size="md" options={SPEEDS} value={pb.speed} onchange={(v) => relay.setPlayback({ speed: v })} />
  </div>
  <div class="group">
    <span class="label">Repeat</span>
    <div class="repeat">
      <button aria-label="Fewer repeats" onclick={() => relay.setPlayback({ repeat: { count: Math.max(1, count - 1) } })}>−</button>
      <span class="count">{infinite ? "∞" : count}</span>
      <button aria-label="More repeats" onclick={() => relay.setPlayback({ repeat: { count: Math.min(99, count + 1) } })}>+</button>
      <button
        class="inf"
        class:on={infinite}
        title="Loop forever"
        aria-label="Loop forever"
        aria-pressed={infinite}
        onclick={toggleForever}><Icon name="loop" size={15} /></button
      >
    </div>
  </div>
</div>

<style>
  .transport {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 12px 16px;
    border-bottom: 2px solid var(--color-divider);
  }
  .clock {
    width: 190px;
    font-variant-numeric: tabular-nums;
  }
  .big {
    font-family: var(--font-heading);
    font-weight: 800;
    font-size: 28px;
    line-height: 1;
    letter-spacing: -0.02em;
  }
  .of {
    font-size: 11px;
    color: var(--color-neutral-600);
    margin-top: 4px;
  }
  .buttons {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .group {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .push {
    margin-left: auto;
  }
  .label {
    font-size: 10px;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    font-weight: 600;
    color: var(--color-neutral-700);
  }
  .repeat {
    display: flex;
    border: 1px solid var(--color-divider);
    align-items: stretch;
    height: 30px;
  }
  .repeat button {
    border: 0;
    width: 28px;
    background: transparent;
    font: inherit;
    font-weight: 800;
    cursor: pointer;
    color: var(--color-text);
  }
  .repeat button:hover {
    background: var(--color-neutral-200);
  }
  .count {
    min-width: 34px;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 13px;
    font-weight: 800;
    border-left: 1px solid var(--color-divider);
    border-right: 1px solid var(--color-divider);
  }
  .repeat .inf {
    width: auto;
    border-left: 1px solid var(--color-divider);
    padding: 0 9px;
    display: flex;
    align-items: center;
  }
  .repeat .inf.on {
    background: var(--color-accent);
    color: var(--color-bg);
  }
</style>
