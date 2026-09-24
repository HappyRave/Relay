<script lang="ts">
  import Toggle from "../../ui/Toggle.svelte";
  import Kbd from "../../ui/Kbd.svelte";
  import { relay } from "../../../lib/state/relay.svelte";
  import { nextRunLabel } from "../../../lib/format";

  const DAYS = ["M", "T", "W", "T", "F", "S", "S"];
  const tr = $derived(relay.triggers);

  function toggleDay(i: number) {
    const days = [...tr.schedule.days];
    days[i] = !days[i];
    relay.setTriggers({ schedule: { ...tr.schedule, days } });
  }
</script>

<div class="list">
  <div class="row">
    <div class="grow"><div class="title">Hotkey</div><div class="sub">Run from anywhere</div></div>
    <Kbd combo={tr.hotkey.combo} />
    <Toggle
      label="Hotkey trigger"
      on={tr.hotkey.enabled}
      onchange={(v) => relay.setTriggers({ hotkey: { ...tr.hotkey, enabled: v } })}
    />
  </div>
  <div class="row col">
    <div class="line">
      <div class="grow">
        <div class="title">Schedule</div>
        <div class="sub accent">{nextRunLabel(tr.schedule.enabled, tr.schedule.days, tr.schedule.time)}</div>
      </div>
      <Toggle
        label="Schedule trigger"
        on={tr.schedule.enabled}
        onchange={(v) => relay.setTriggers({ schedule: { ...tr.schedule, enabled: v } })}
      />
    </div>
    <div class="line">
      <div class="days" role="group" aria-label="Days">
        {#each DAYS as d, i (i)}
          <button class:on={tr.schedule.days[i]} aria-pressed={tr.schedule.days[i]} onclick={() => toggleDay(i)}>{d}</button>
        {/each}
      </div>
      <input
        type="time"
        class="input time"
        aria-label="Time"
        value={tr.schedule.time}
        onchange={(e) => relay.setTriggers({ schedule: { ...tr.schedule, time: e.currentTarget.value } })}
      />
    </div>
  </div>
  <div class="row">
    <div class="grow"><div class="title">When app launches</div><div class="sub">{tr.appLaunch.exe}</div></div>
    <Toggle
      label="App launch trigger"
      on={tr.appLaunch.enabled}
      onchange={(v) => relay.setTriggers({ appLaunch: { ...tr.appLaunch, enabled: v } })}
    />
  </div>
  <div class="row">
    <div class="grow">
      <div class="title">When pixel changes</div>
      <div class="sub">{tr.pixel.x}, {tr.pixel.y} becomes {tr.pixel.color}</div>
    </div>
    <Toggle
      label="Pixel trigger"
      on={tr.pixel.enabled}
      onchange={(v) => relay.setTriggers({ pixel: { ...tr.pixel, enabled: v } })}
    />
  </div>
</div>

<style>
  .list {
    flex: 1;
    overflow-y: auto;
    font-size: 13px;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 12px;
    border-bottom: 1px solid var(--color-neutral-300);
  }
  .col {
    flex-direction: column;
    align-items: stretch;
    gap: 8px;
  }
  .line {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .grow {
    flex: 1;
  }
  .title {
    font-weight: 600;
  }
  .sub {
    font-size: 11px;
    color: var(--color-neutral-600);
  }
  .sub.accent {
    color: var(--color-accent-700);
  }
  .days {
    display: flex;
    border: 1px solid var(--color-divider);
  }
  .days button {
    width: 24px;
    height: 28px;
    border: 0;
    font: inherit;
    font-size: 11px;
    font-weight: 800;
    cursor: pointer;
    background: transparent;
    color: var(--color-text);
  }
  .days button:not(.on):hover {
    background: var(--color-neutral-200);
  }
  .days button.on {
    background: var(--color-text);
    color: var(--color-bg);
  }
  .time {
    width: auto;
    min-height: 30px;
    padding: 3px 6px;
    font-size: 12px;
  }
</style>
