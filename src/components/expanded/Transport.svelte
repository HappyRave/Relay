<script lang="ts">
  // The control bar (Design: "Control Bar Options", 1b): the time on the
  // left, the transport pinned to the middle column between two rules, and
  // speed and repeat on the right. Every control is 48 px tall under a label,
  // all labels on one baseline, so the row never drifts; a narrower bar
  // compacts its right side (see `barMode`).
  import RecPlayButtons from "../shared/RecPlayButtons.svelte";
  import Icon from "../ui/Icon.svelte";
  import { MAX_REPEATS, relay } from "../../lib/state/relay.svelte";
  import { fmtTime } from "../../lib/format";
  import { SPEEDS, nextSpeed, type BarMode } from "../../lib/transport";

  /** Which controls fit (the editor measures the bar). */
  let { mode = "wide" }: { mode?: BarMode } = $props();

  const pb = $derived(relay.playback);
  const infinite = $derived(pb.repeat === "forever");
  const count = $derived(relay.repeatCount);
  const cur = $derived(Math.min(relay.cur, relay.duration));
  const off = $derived(relay.recording);
  /** With a data file, the macro plays once per row instead. */
  const data = $derived(relay.dataFile);
  const speedLabel = (s: number) => `${s}×`;
</script>

<div class="transport {mode}">
  <div class="zone time">
    <div class="label"><span>Time</span><span class="of">/ {relay.mode === "recording" ? "recording" : fmtTime(relay.duration)}</span></div>
    <div class="big">{fmtTime(cur)}</div>
  </div>

  <div class="zone center">
    <div class="label" aria-hidden="true">Playback</div>
    <div class="strip">
      <button class="cell" title="Previous step" aria-label="Previous step" onclick={() => relay.jump(-1)}>
        <Icon name="prev" size={20} />
      </button>
      <RecPlayButtons variant="strip" />
      <button class="cell" title="Stop (Esc)" aria-label="Stop" onclick={relay.stop}>
        <Icon name="stop" size={16} />
      </button>
      <button class="cell" title="Next step" aria-label="Next step" onclick={() => relay.jump(1)}>
        <Icon name="next" size={20} />
      </button>
    </div>
  </div>

  <div class="zone settings">
    <div class="setting">
      <span class="label" aria-hidden="true">Speed</span>
      {#if mode === "wide"}
        <div class="strip" role="radiogroup" aria-label="Speed">
          {#each SPEEDS as s (s)}
            <button
              class="speed"
              class:active={s === pb.speed}
              role="radio"
              aria-checked={s === pb.speed}
              disabled={off}
              onclick={() => relay.setPlayback({ speed: s })}>{speedLabel(s)}</button
            >
          {/each}
        </div>
      {:else}
        <button
          class="solo speed"
          title="Speed: click for {speedLabel(nextSpeed(pb.speed))}"
          aria-label="Speed {speedLabel(pb.speed)}"
          disabled={off}
          onclick={relay.cycleSpeed}>{speedLabel(pb.speed)}</button
        >
      {/if}
    </div>

    <div class="setting">
      <span class="label" aria-hidden="true">Repeat</span>
      <div class="strip">
        {#if data}
          <span
            class="rows"
            title={data.error ?? "Plays once per row of the data file (Settings → Playback)"}
            >Each row{data.error ? "" : ` (${data.rows})`}</span
          >
        {:else if mode === "narrow"}
          <!-- Folded: one button that steps through the usual counts. -->
          <button
            class="loop folded"
            class:on={infinite}
            title="Repeat: click for more (1, 2, 3, 5, 10, forever)"
            aria-label={infinite ? "Repeat forever" : `Repeat ${count} ${count === 1 ? "time" : "times"}`}
            disabled={off}
            onclick={relay.cycleRepeat}><Icon name="loop" size={18} /><span>{infinite ? "∞" : `×${count}`}</span></button
          >
        {:else}
          <!-- While looping forever, − goes back to the count it had. -->
          <button class="step" aria-label="Fewer repeats" disabled={off || (!infinite && count <= 1)} onclick={relay.fewerRepeats}>
            <Icon name="minus" size={16} />
          </button>
          <span class="count">{infinite ? "∞" : count}</span>
          <button class="step" aria-label="More repeats" disabled={off || infinite || count >= MAX_REPEATS} onclick={relay.moreRepeats}>
            <Icon name="plus" size={16} />
          </button>
          <button
            class="loop"
            class:on={infinite}
            title="Loop forever"
            aria-label="Loop forever"
            aria-pressed={infinite}
            disabled={off}
            onclick={relay.toggleForever}><Icon name="loop" size={18} /></button
          >
        {/if}
      </div>
    </div>
  </div>
</div>

<style>
  /* Three ruled zones; in each, a 14 px label band and a 48 px control row. */
  .transport {
    height: 102px;
    box-sizing: border-box;
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto minmax(0, 1fr);
    align-items: stretch;
  }
  .zone {
    padding: 16px 24px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    min-width: 0;
  }
  .label {
    height: 14px;
    display: flex;
    gap: 8px;
    font-size: 11px;
    line-height: 14px;
    letter-spacing: 0.12em;
    font-weight: 700;
    text-transform: uppercase;
    color: var(--color-neutral-700);
    white-space: nowrap;
  }
  .of {
    font-weight: 500;
    letter-spacing: 0.04em;
    text-transform: none;
    font-variant-numeric: tabular-nums;
  }
  .big {
    height: 48px;
    display: flex;
    align-items: center;
    font-family: var(--font-heading);
    font-size: 40px;
    font-weight: 800;
    letter-spacing: -0.01em;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
  .mid .big {
    font-size: 36px;
  }
  .narrow .big {
    font-size: 32px;
  }
  .center {
    border-left: 2px solid var(--color-divider);
    border-right: 2px solid var(--color-divider);
  }
  .settings {
    flex-direction: row;
    align-items: flex-start;
    justify-content: flex-end;
    gap: 20px;
  }
  .setting {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  /* A row of joined 48 px cells: each draws its left rule, the strip the rest. */
  .strip {
    display: flex;
    height: 48px;
    box-sizing: border-box;
    border: 1px solid var(--color-neutral-400);
    border-left: none;
  }
  .strip > :global(button),
  .count,
  .rows {
    flex: none;
    border: 0;
    border-left: 1px solid var(--color-neutral-400);
    background: var(--color-bg);
    color: var(--color-text);
    font: inherit;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
  }
  .strip > :global(button:hover:not(:disabled)) {
    background: var(--color-accent-100);
  }
  .strip > :global(button:disabled) {
    color: var(--color-neutral-500);
    cursor: not-allowed;
  }
  .cell {
    width: 48px;
  }
  .speed {
    width: 56px;
    justify-content: flex-start !important;
    padding-left: 12px;
    font-size: 16px !important;
    font-weight: 600 !important;
  }
  .speed.active,
  .strip > .speed.active:hover:not(:disabled) {
    background: var(--color-accent);
    color: var(--color-bg);
    border-left-color: var(--color-accent);
    font-weight: 700 !important;
  }
  .solo {
    height: 48px;
    width: 72px;
    box-sizing: border-box;
    border: 1px solid var(--color-neutral-400);
    background: var(--color-bg);
    color: var(--color-text);
    font-weight: 700 !important;
    cursor: pointer;
    display: flex;
    align-items: center;
  }
  .solo:hover:not(:disabled) {
    background: var(--color-accent-100);
  }
  .solo:disabled {
    color: var(--color-neutral-500);
    cursor: not-allowed;
  }
  .step {
    width: 44px;
  }
  .count {
    width: 48px;
    cursor: default;
    font-size: 17px;
    font-weight: 700;
    font-variant-numeric: tabular-nums;
  }
  .rows {
    padding: 0 13px;
    cursor: default;
    font-size: 15px;
    font-weight: 700;
    white-space: nowrap;
  }
  .loop {
    gap: 6px;
    padding: 0 13px;
    font-size: 15px !important;
    font-weight: 700 !important;
  }
  .loop.on,
  .strip > .loop.on:hover:not(:disabled) {
    background: var(--color-accent);
    color: var(--color-bg);
    border-left-color: var(--color-accent);
  }
</style>
