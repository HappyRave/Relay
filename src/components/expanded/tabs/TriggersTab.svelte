<script lang="ts">
  import Toggle from "../../ui/Toggle.svelte";
  import HotkeyCapture from "../../ui/HotkeyCapture.svelte";
  import Segmented from "../../ui/Segmented.svelte";
  import { relay, TRIGGER } from "../../../lib/state/relay.svelte";
  import { areaChoice, pngUrl } from "../../../lib/image";
  import { onMount } from "svelte";
  import { nextRunLabel } from "../../../lib/format";
  import { clamp, commitNumber, toMs } from "../../../lib/fields";

  const DAYS = ["M", "T", "W", "T", "F", "S", "S"];
  const HEX = /^#[0-9a-fA-F]{6}$/;
  const status = $derived(relay.triggerStatus);
  const t = $derived(status?.triggers);
  const scheduleLabel = $derived.by(() => {
    if (!t?.schedule.enabled) return "Off";
    if (!t.schedule.schedule.days.some(Boolean)) return "Pick a day";
    return nextRunLabel(status?.next_run ?? null);
  });

  const monitors = $derived(relay.view?.recording.monitors ?? []);
  const areas = $derived(areaChoice(monitors, t?.image.area ?? null));
  const snipping = $derived(relay.imaging?.index === TRIGGER && relay.imaging.source === "snip");
  const test = $derived(relay.imageTest?.id === relay.view?.id && relay.imageTest?.item === TRIGGER ? relay.imageTest.text : "");

  // Suggestions for "When app launches".
  onMount(relay.loadProcesses);

  function toggleDay(i: number) {
    if (!t) return;
    const days = [...t.schedule.schedule.days] as typeof t.schedule.schedule.days;
    days[i] = !days[i];
    relay.setTriggers({ schedule: { ...t.schedule, schedule: { ...t.schedule.schedule, days } } });
  }

  /**
   * Commits a number field; it then shows what's saved (rounded to whole
   * pixels or ms, clamped), or the current value again if it was refused.
   */
  function withNumber(e: Event, current: number, accept: (v: number) => number | null, apply: (v: number) => void, show?: (v: number) => string) {
    const v = commitNumber(e.currentTarget as HTMLInputElement, current, accept, show);
    if (v != null) apply(v);
  }
</script>

{#if relay.triggersPaused}
  <div class="paused" role="status">
    <span>Triggers are paused (kill switch)</span>
    <button class="btn btn-ghost" onclick={() => relay.setTriggersPaused(false)}>Resume</button>
  </div>
{/if}
{#if !relay.view}
  <div class="empty">Open a macro to set its triggers</div>
{:else if relay.triggersFailed}
  <div class="empty" role="alert">
    Couldn't load the triggers
    <button class="btn btn-secondary" onclick={relay.loadTriggers}>Retry</button>
  </div>
{:else if t}
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
        disabled={t.hotkey.combo === ""}
        onchange={(v) => relay.setTriggers({ hotkey: { ...t.hotkey, enabled: v } })}
      />
    </div>

    <div class="row col">
      <div class="line">
        <div class="grow">
          <div class="title">Schedule</div>
          <div class="sub" class:accent={t.schedule.enabled}>{scheduleLabel}</div>
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
          onchange={(e) => {
            const time = e.currentTarget.value;
            // A cleared time isn't a time: show the saved one again.
            if (!time) e.currentTarget.value = t.schedule.schedule.time;
            else if (time !== t.schedule.schedule.time) relay.setTriggers({ schedule: { ...t.schedule, schedule: { ...t.schedule.schedule, time } } });
          }}
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
          disabled={t.app_launch.exe.trim() === ""}
          onchange={(v) => relay.setTriggers({ app_launch: { ...t.app_launch, enabled: v } })}
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
          {#each relay.processes as p (p)}<option value={p}></option>{/each}
        </datalist>
        <input
          class="input delay"
          type="number"
          min="0"
          step="0.5"
          aria-label="Delay in seconds"
          title="Delay in seconds"
          value={t.app_launch.delay_ms / 1000}
          onchange={(e) =>
            withNumber(
              e,
              t.app_launch.delay_ms,
              (s) => Math.max(0, toMs(s)),
              (delay_ms) => relay.setTriggers({ app_launch: { ...t.app_launch, delay_ms } }),
              (ms) => String(ms / 1000),
            )}
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
          onchange={(e) => withNumber(e, t.pixel.x, Math.round, (x) => relay.setTriggers({ pixel: { ...t.pixel, x } }))}
        />
        <input
          class="input small"
          type="number"
          aria-label="Y"
          value={t.pixel.y}
          onchange={(e) => withNumber(e, t.pixel.y, Math.round, (y) => relay.setTriggers({ pixel: { ...t.pixel, y } }))}
        />
        <input
          class="input small"
          aria-label="Color"
          maxlength="7"
          spellcheck="false"
          value={t.pixel.color}
          onchange={(e) => {
            const color = e.currentTarget.value.trim().toUpperCase();
            e.currentTarget.value = HEX.test(color) ? color : t.pixel.color;
            if (HEX.test(color) && color !== t.pixel.color) relay.setTriggers({ pixel: { ...t.pixel, color } });
          }}
        />
        <button class="btn btn-secondary pick" disabled={relay.picking > 0 || !relay.editable} onclick={relay.pickTriggerPixel}>
          {relay.picking > 0 ? `Point at it… ${relay.picking}` : "Pick"}
        </button>
      </div>
    </div>

    <div class="row col">
      <div class="line">
        {#if t.image.image}<img class="thumb" src={pngUrl(t.image.image)} alt="What to watch for" />{/if}
        <div class="grow">
          <div class="title">When image appears</div>
          <div class="sub">{t.image.image ? `At a ${t.image.threshold}% match or better` : "Snip, paste or choose the image to watch for"}</div>
        </div>
        <Toggle
          label="Image trigger"
          on={t.image.enabled}
          disabled={!t.image.image}
          onchange={(v) => relay.setTriggerImageOptions({ enabled: v })}
        />
      </div>
      <div class="line">
        {#if snipping}
          <span class="sub" role="status">Snip the image…</span>
          <button class="btn btn-secondary pick" onclick={relay.cancelImage}>Cancel</button>
        {:else}
          <button class="btn btn-secondary pick" disabled={!!relay.imaging || !relay.editable} title="Snip it from the screen" onclick={() => relay.setTriggerImage("snip")}>Snip</button>
          <button class="btn btn-secondary pick" disabled={!!relay.imaging || !relay.editable} title="Use the picture on the clipboard" onclick={() => relay.setTriggerImage("paste")}>Paste</button>
          <button class="btn btn-secondary pick" disabled={!!relay.imaging || !relay.editable} onclick={() => relay.setTriggerImage("file")}>File…</button>
        {/if}
        <input
          class="input small"
          type="number"
          min="50"
          max="100"
          aria-label="Match %"
          title="Match %"
          value={t.image.threshold}
          onchange={(e) => withNumber(e, t.image.threshold, (v) => clamp(Math.round(v), 50, 100), (threshold) => relay.setTriggerImageOptions({ threshold }))}
        />
        <button class="btn btn-secondary pick" disabled={!t.image.image || !relay.editable} title="Look for it on the screen now" onclick={relay.testTriggerImage}>Test</button>
      </div>
      {#if test}<div class="sub" role="status">{test}</div>{/if}
      {#if monitors.length > 1}
        <Segmented
          label="Where to look"
          options={areas.options}
          value={areas.value}
          onchange={(i) => i !== -2 && relay.setTriggerImageOptions({ area: i < 0 ? null : monitors[i].rect })}
        />
      {/if}
    </div>
  </div>
{/if}

<style>
  .empty {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 8px;
    padding: 12px;
    font-size: 12px;
    color: var(--color-neutral-700);
  }
  .empty .btn {
    font-size: 12px;
    padding: 2px 8px;
  }
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
  .thumb {
    max-width: 64px;
    max-height: 32px;
    border: 1px solid var(--color-text);
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
