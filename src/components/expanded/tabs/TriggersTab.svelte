<script lang="ts">
  import Toggle from "../../ui/Toggle.svelte";
  import HotkeyCapture from "../../ui/HotkeyCapture.svelte";
  import { relay } from "../../../lib/state/relay.svelte";
  import { backend } from "../../../lib/ipc/backend";
  import { nextRunLabel } from "../../../lib/format";

  const DAYS = ["M", "T", "W", "T", "F", "S", "S"];
  const HEX = /^#[0-9a-fA-F]{6}$/;
  const status = $derived(relay.triggerStatus);
  const t = $derived(status?.triggers);

  // Suggestions for "When app launches".
  let processes = $state<string[]>([]);
  $effect(() => {
    backend.listProcesses().then(
      (p) => (processes = p),
      () => {},
    );
  });

  function toggleDay(i: number) {
    if (!t) return;
    const days = [...t.schedule.schedule.days] as typeof t.schedule.schedule.days;
    days[i] = !days[i];
    relay.setTriggers({ schedule: { ...t.schedule, schedule: { ...t.schedule.schedule, days } } });
  }

  const num = (e: Event) => Number((e.currentTarget as HTMLInputElement).value);
</script>

{#if relay.triggersPaused}
  <div class="paused" role="status">
    <span>Triggers are paused (kill switch)</span>
    <button class="btn btn-ghost" onclick={() => relay.setTriggersPaused(false)}>Resume</button>
  </div>
{/if}
{#if t}
  <div class="list">
    <div class="row">
      <div class="grow">
        <div class="title">Hotkey</div>
        <div class="sub" class:warn={status?.hotkey_error}>{status?.hotkey_error ?? "Run from anywhere"}</div>
      </div>
      <HotkeyCapture
        combo={t.hotkey.combo}
        onchange={(combo) => relay.setTriggers({ hotkey: { enabled: combo !== "", combo } })}
      />
      <Toggle
        label="Hotkey trigger"
        on={t.hotkey.enabled}
        onchange={(v) => relay.setTriggers({ hotkey: { ...t.hotkey, enabled: v && t.hotkey.combo !== "" } })}
      />
    </div>

    <div class="row col">
      <div class="line">
        <div class="grow">
          <div class="title">Schedule</div>
          <div class="sub accent">{nextRunLabel(status?.next_run ?? null)}</div>
        </div>
        <Toggle
          label="Schedule trigger"
          on={t.schedule.enabled}
          onchange={(v) => relay.setTriggers({ schedule: { ...t.schedule, enabled: v } })}
        />
      </div>
      <div class="line">
        <div class="days" role="group" aria-label="Days">
          {#each DAYS as d, i (i)}
            <button class:on={t.schedule.schedule.days[i]} aria-pressed={t.schedule.schedule.days[i]} onclick={() => toggleDay(i)}
              >{d}</button
            >
          {/each}
        </div>
        <input
          type="time"
          class="input time"
          aria-label="Time"
          value={t.schedule.schedule.time}
          onchange={(e) =>
            e.currentTarget.value &&
            relay.setTriggers({ schedule: { ...t.schedule, schedule: { ...t.schedule.schedule, time: e.currentTarget.value } } })}
        />
      </div>
    </div>

    <div class="row col">
      <div class="line">
        <div class="grow">
          <div class="title">When app launches</div>
          <div class="sub">Runs {t.app_launch.delay_ms / 1000} s after it starts</div>
        </div>
        <Toggle
          label="App launch trigger"
          on={t.app_launch.enabled}
          onchange={(v) => relay.setTriggers({ app_launch: { ...t.app_launch, enabled: v && t.app_launch.exe.trim() !== "" } })}
        />
      </div>
      <div class="line">
        <input
          class="input grow"
          list="relay-processes"
          placeholder="e.g. EXCEL.EXE"
          aria-label="Program"
          value={t.app_launch.exe}
          onchange={(e) => {
            const exe = e.currentTarget.value.trim();
            relay.setTriggers({ app_launch: { ...t.app_launch, exe, enabled: t.app_launch.enabled && exe !== "" } });
          }}
        />
        <datalist id="relay-processes">
          {#each processes as p (p)}<option value={p}></option>{/each}
        </datalist>
        <input
          class="input delay"
          type="number"
          min="0"
          step="0.5"
          aria-label="Delay in seconds"
          title="Delay in seconds"
          value={t.app_launch.delay_ms / 1000}
          onchange={(e) => relay.setTriggers({ app_launch: { ...t.app_launch, delay_ms: Math.max(0, Math.round(num(e) * 1000)) } })}
        />
      </div>
    </div>

    <div class="row col">
      <div class="line">
        <div class="grow">
          <div class="title">When pixel changes</div>
          <div class="sub">
            <span class="swatch" style:background={t.pixel.color}></span>{t.pixel.x}, {t.pixel.y} becomes {t.pixel.color}
          </div>
        </div>
        <Toggle label="Pixel trigger" on={t.pixel.enabled} onchange={(v) => relay.setTriggers({ pixel: { ...t.pixel, enabled: v } })} />
      </div>
      <div class="line">
        <input
          class="input small"
          type="number"
          aria-label="X"
          value={t.pixel.x}
          onchange={(e) => relay.setTriggers({ pixel: { ...t.pixel, x: num(e) } })}
        />
        <input
          class="input small"
          type="number"
          aria-label="Y"
          value={t.pixel.y}
          onchange={(e) => relay.setTriggers({ pixel: { ...t.pixel, y: num(e) } })}
        />
        <input
          class="input small"
          aria-label="Color"
          maxlength="7"
          spellcheck="false"
          value={t.pixel.color}
          onchange={(e) => {
            const color = e.currentTarget.value.trim();
            if (HEX.test(color)) relay.setTriggers({ pixel: { ...t.pixel, color: color.toUpperCase() } });
            else e.currentTarget.value = t.pixel.color;
          }}
        />
        <button class="btn btn-secondary pick" disabled={relay.picking > 0 || !relay.editable} onclick={relay.pickTriggerPixel}>
          {relay.picking > 0 ? `Point… ${relay.picking}` : "Pick"}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .paused {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 8px 6px 12px;
    background: var(--color-accent-100);
    color: var(--color-accent-800);
    border-bottom: 1px solid var(--color-divider);
    font-size: 12px;
  }
  .paused span {
    flex: 1;
  }
  .paused .btn {
    font-size: 12px;
    padding: 2px 6px;
  }
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
    min-width: 0;
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
  .sub.warn {
    color: var(--color-accent-700);
    font-weight: 600;
  }
  .swatch {
    display: inline-block;
    width: 9px;
    height: 9px;
    margin-right: 5px;
    border: 1px solid var(--color-text);
    vertical-align: -1px;
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
  .input {
    min-height: 28px;
    padding: 3px 6px;
    font-size: 12px;
  }
  .time {
    width: auto;
  }
  .delay {
    width: 56px;
  }
  .small {
    width: 64px;
  }
  .pick {
    min-height: 28px;
    padding: 2px 8px;
    font-size: 12px;
  }
</style>
