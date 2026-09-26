<script lang="ts">
  import Grip from "./shared/Grip.svelte";
  import RecPlayButtons from "./shared/RecPlayButtons.svelte";
  import Icon from "./ui/Icon.svelte";
  import { relay } from "../lib/state/relay.svelte";
  import { BADGE } from "../lib/state/display";
  import { fmtTime } from "../lib/format";
  import { pct } from "../lib/timeline/lanes";
  import { seekable } from "../lib/actions/seekable";

  const cur = $derived(Math.min(relay.cur, relay.duration));
  const clicks = $derived(relay.steps.filter((s) => s.kind === "click"));
</script>

<div class="compact">
  <Grip />
  <RecPlayButtons variant="bar" />
  <div class="info">
    <div class="row">
      <span class="badge" class:rec={relay.recording} class:play={relay.mode === "playing"}>{BADGE[relay.mode]}</span>
      <span class="name">{relay.name}</span>
      <span class="time">
        {fmtTime(cur)} <span class="of">/ {relay.mode === "recording" ? "recording" : fmtTime(relay.duration)}</span>
      </span>
    </div>
    <div class="seek" use:seekable>
      <div class="fill" style:width="{pct(cur, relay.duration)}%"></div>
      {#each clicks as c (c.items[0])}
        <div class="tick" style:left="{pct(c.t, relay.duration)}%"></div>
      {/each}
    </div>
  </div>
  <button class="expand" title="Expand (Ctrl + Shift + M)" aria-label="Expand" onclick={() => (relay.expanded = true)}>
    <Icon name="expand" size={18} />
  </button>
</div>

<style>
  .compact {
    display: flex;
    align-items: stretch;
    height: 64px;
    width: 600px;
  }
  .info {
    flex: 1;
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: 6px;
    padding: 0 16px;
    min-width: 0;
  }
  .row {
    display: flex;
    align-items: baseline;
    gap: 10px;
  }
  .badge {
    font-size: 10px;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    font-weight: 600;
    padding: 2px 6px;
    background: var(--color-neutral-300);
    color: var(--color-text);
    white-space: nowrap;
  }
  .badge.rec {
    background: var(--color-accent);
    color: var(--color-bg);
  }
  .badge.play {
    background: var(--color-text);
    color: var(--color-bg);
  }
  .name {
    font-weight: 800;
    font-size: 14px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .time {
    margin-left: auto;
    font-size: 12px;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
  .of {
    color: var(--color-neutral-600);
  }
  .seek {
    position: relative;
    height: 6px;
    background: var(--color-neutral-300);
    cursor: pointer;
  }
  .fill {
    position: absolute;
    left: 0;
    top: 0;
    bottom: 0;
    background: var(--color-text);
  }
  .tick {
    position: absolute;
    top: -3px;
    bottom: -3px;
    width: 2px;
    background: var(--color-accent);
  }
  .expand {
    width: 52px;
    flex: none;
    border: 0;
    border-left: 2px solid var(--color-divider);
    background: transparent;
    color: var(--color-text);
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
  }
  .expand:hover {
    background: var(--color-neutral-200);
  }
</style>
