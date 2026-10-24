<script lang="ts">
  // The inspector's Macro tab: the macro as a whole, in two sections, one open
  // at a time. Play options: repeat, speed, its data file, varied timing,
  // stopping on a key, where clicks land. Runs automatically: the triggers it
  // has, each a card, and + Add for the others.
  import { onMount } from "svelte";
  import Section from "../kit/Section.svelte";
  import Switch from "../kit/Switch.svelte";
  import Choice from "../kit/Choice.svelte";
  import UnitField from "../kit/UnitField.svelte";
  import Key from "../kit/Key.svelte";
  import Menu, { type MenuItem } from "../kit/Menu.svelte";
  import Icon, { type IconName } from "../ui/Icon.svelte";
  import HotkeyCapture from "../ui/HotkeyCapture.svelte";
  import { relay, TRIGGER } from "../../lib/state/relay.svelte";
  import { playSummary } from "../../lib/editor/summary";
  import { CLEARED, DAY_NAMES, hasTrigger, TRIGGER_KINDS, TRIGGER_NAME, triggerNote, triggersSummary, triggerText, type TriggerKind } from "../../lib/editor/triggers";
  import { SPEEDS } from "../../lib/transport";
  import { plural } from "../../lib/format";
  import { toMs } from "../../lib/fields";
  import { areaChoice, pngUrl } from "../../lib/image";
  import type { CoordMode } from "../../lib/types";

  const HEX = /^#[0-9a-fA-F]{6}$/;
  const ICON: Record<TriggerKind, IconName> = { hotkey: "keycap", schedule: "clock", app_launch: "app", pixel: "pixel", image: "image" };

  const pb = $derived(relay.playback);
  const data = $derived(relay.dataFile);
  const rows = $derived(data && !data.error ? data.rows : null);
  const fileName = (path: string) => path.split(/[\\/]/).pop();
  const status = $derived(relay.triggerStatus);
  const t = $derived(status?.triggers);
  /** Triggers added here that aren't set yet, so their card shows until they are. */
  let added = $state<TriggerKind[]>([]);
  $effect(() => {
    void relay.view?.id;
    added = [];
  });
  const shown = $derived(t ? TRIGGER_KINDS.filter((k) => hasTrigger(t, k) || added.includes(k)) : []);
  const missing = $derived(TRIGGER_KINDS.filter((k) => !shown.includes(k)));
  const editable = $derived(relay.editable);

  const monitors = $derived(relay.view?.recording.monitors ?? []);
  const areas = $derived(areaChoice(monitors, t?.image.area ?? null));
  const snipping = $derived(relay.imaging?.index === TRIGGER && relay.imaging.source === "snip");
  const test = $derived(relay.imageTest?.id === relay.view?.id && relay.imageTest?.item === TRIGGER ? relay.imageTest : null);

  // The data file may have changed on disk since the macro was opened; the
  // running programs are the app trigger's suggestions.
  $effect(() => void relay.loadDataFile());
  onMount(relay.loadProcesses);

  let addKey: HTMLElement | undefined = $state();
  let adding = $state(false);
  const ADD = $derived<MenuItem[]>(missing.map((k) => ({ id: k, label: TRIGGER_NAME[k], icon: ICON[k] })));

  function add(kind: string) {
    const k = kind as TriggerKind;
    added = [...added, k];
    // A schedule works as it comes: weekdays at 09:00.
    if (k === "schedule" && t) relay.setTriggers({ schedule: { ...t.schedule, enabled: true } });
  }

  function remove(k: TriggerKind) {
    added = added.filter((a) => a !== k);
    if (k === "image") relay.setTriggerImageOptions({ ...CLEARED.image });
    else relay.setTriggers({ [k]: structuredClone(CLEARED[k]) });
  }

  function toggleDay(i: number) {
    if (!t) return;
    const days = [...t.schedule.schedule.days] as typeof t.schedule.schedule.days;
    days[i] = !days[i];
    relay.setTriggers({ schedule: { ...t.schedule, schedule: { ...t.schedule.schedule, days } } });
  }

  function setPixelColor(e: Event) {
    if (!t) return;
    const input = e.currentTarget as HTMLInputElement;
    const color = input.value.trim().toUpperCase();
    input.value = HEX.test(color) ? color : t.pixel.color;
    if (HEX.test(color) && color !== t.pixel.color) relay.setTriggers({ pixel: { ...t.pixel, color } });
  }

  const open = (s: "play" | "triggers") => (relay.macroSection = relay.macroSection === s ? (s === "play" ? "triggers" : "play") : s);
</script>

<div class="pane">
  <Section label="Play options" summary={playSummary(pb, rows).chips.join(" · ")} open={relay.macroSection === "play"} controls="play-options" onclick={() => open("play")} />
  {#if relay.macroSection === "play"}
    <div class="spec" id="play-options">
      <span class="k">Repeat</span>
      {#if rows != null}
        <span>Once per row of the data file ({plural(rows, "row")})</span>
      {:else}
        <span class="line">
          <UnitField
            label="Repeat"
            unit="times"
            value={pb.repeat === "forever" ? relay.repeatCount : pb.repeat.count}
            step={1}
            min={1}
            max={9999}
            decimals={0}
            disabled={pb.repeat === "forever"}
            oncommit={(count) => relay.setPlayback({ repeat: { count: Math.round(count) } })}
          />
          <Key size="sm" aria-pressed={pb.repeat === "forever"} title="Loop until stopped" onclick={relay.toggleForever}><Icon name="loop" size={14} />Forever</Key>
        </span>
      {/if}
      <span class="k" id="m-speed">Speed</span>
      <Choice labelledby="m-speed" options={SPEEDS.map((s): [number, string] => [s, `${s}×`])} value={pb.speed} onchange={(speed) => relay.setPlayback({ speed })} />

      <span class="k">Data file</span>
      <span class="data">
        {#if data}
          <span title={data.path}>{fileName(data.path)} · {plural(data.rows, "row")}</span>
          {#if data.error}<span class="wrong" role="alert">{data.error}</span>{/if}
        {:else}
          <span class="hint">A CSV file: the macro plays once per row, and Text steps type its columns.</span>
        {/if}
        <span class="line">
          <Key size="sm" disabled={!editable} onclick={relay.chooseDataFile}>{data ? "Change…" : "Choose…"}</Key>
          {#if data}<Key size="sm" variant="ghost" disabled={!editable} onclick={relay.removeDataFile}>Remove</Key>{/if}
        </span>
      </span>

      <span class="k" id="m-vary">Vary timing</span>
      <span class="line">
        <Switch labelledby="m-vary" on={pb.humanize} onchange={(humanize) => relay.setPlayback({ humanize })} />
        <UnitField
          label="By up to"
          unit="ms"
          pre="±"
          value={pb.jitter_ms}
          step={5}
          min={0}
          max={200}
          decimals={0}
          disabled={!pb.humanize}
          onscrub={(jitter_ms) => relay.previewPlayback({ jitter_ms })}
          oncommit={(jitter_ms) => relay.setPlayback({ jitter_ms: Math.round(jitter_ms) })}
        />
      </span>
      <span class="k" id="m-stop">Stop on key</span>
      <span class="line">
        <Switch labelledby="m-stop" on={pb.stop_on_key} onchange={(stop_on_key) => relay.setPlayback({ stop_on_key })} />
        <span class="hint">{pb.stop_on_key ? "Any key stops playback" : "Keys don't stop it: Esc and Stop still do"}</span>
      </span>
      <span class="k" id="m-coords">Clicks land</span>
      <Choice
        labelledby="m-coords"
        options={[["screen", "Screen"], ["window", "Window"]] as [CoordMode, string][]}
        value={pb.coord_mode}
        onchange={(coord_mode) => relay.setPlayback({ coord_mode })}
      />
      <span class="full subtle">Saved with this macro. Speed and the others can change while it plays.</span>
    </div>
  {/if}

  <Section
    label="Runs automatically"
    summary={t ? triggersSummary(t) : ""}
    open={relay.macroSection === "triggers"}
    controls="runs-automatically"
    onclick={() => open("triggers")}
  />
  {#if relay.macroSection === "triggers"}
    <div class="triggers" id="runs-automatically">
      {#if relay.triggersPaused}
        <div class="notice" role="status">
          <Icon name="warning" size={14} /><span>Triggers are paused (the kill switch).</span>
          <Key size="sm" onclick={() => relay.setTriggersPaused(false)}>Resume</Key>
        </div>
      {/if}
      {#if relay.triggersFailed}
        <div class="notice" role="alert">
          <span>Couldn't load the triggers.</span>
          <Key size="sm" onclick={relay.loadTriggers}>Retry</Key>
        </div>
      {:else if t && status}
        {#each shown as k (k)}
          <div class="card" role="group" aria-label={TRIGGER_NAME[k]}>
            <div class="top">
              <Icon name={ICON[k]} size={16} />
              <span class="what"><b>{TRIGGER_NAME[k]}</b><span class:wrong={k === "hotkey" && status.hotkey_error}>{triggerNote(k, status)}</span></span>
              <Switch
                label="{TRIGGER_NAME[k]} on"
                on={t[k].enabled}
                disabled={!editable ||
                  (k === "hotkey" && t.hotkey.combo === "") ||
                  (k === "app_launch" && t.app_launch.exe.trim() === "") ||
                  (k === "image" && !t.image.image)}
                onchange={(enabled) => (k === "image" ? relay.setTriggerImageOptions({ enabled }) : relay.setTriggers({ [k]: { ...t[k], enabled } }))}
              />
            </div>
            <span class="sentence">{triggerText(k, t)}</span>
            {#if k === "hotkey"}
              <HotkeyCapture combo={t.hotkey.combo} disabled={!editable} onchange={(combo) => relay.setTriggers({ hotkey: { enabled: combo !== "", combo } })} />
            {:else if k === "schedule"}
              <span class="line">
                <span class="days" role="group" aria-label="Days">
                  {#each DAY_NAMES as day, i (day)}
                    <button type="button" aria-label={day} aria-pressed={t.schedule.schedule.days[i]} disabled={!editable} onclick={() => toggleDay(i)}>{day[0]}</button>
                  {/each}
                </span>
                <span class="k">at</span>
                <input
                  type="time"
                  class="field time"
                  aria-label="Time"
                  value={t.schedule.schedule.time}
                  disabled={!editable}
                  onchange={(e) => {
                    const time = e.currentTarget.value;
                    // A cleared time isn't a time: show the saved one again.
                    if (!time) e.currentTarget.value = t.schedule.schedule.time;
                    else if (time !== t.schedule.schedule.time) relay.setTriggers({ schedule: { ...t.schedule, schedule: { ...t.schedule.schedule, time } } });
                  }}
                />
              </span>
            {:else if k === "app_launch"}
              <span class="fields">
                <label class="k" for="t-exe">Program</label>
                <input
                  id="t-exe"
                  class="field"
                  list="relay-processes"
                  placeholder="e.g. EXCEL.EXE"
                  value={t.app_launch.exe}
                  disabled={!editable}
                  onchange={(e) => {
                    const exe = e.currentTarget.value.trim();
                    relay.setTriggers({ app_launch: { ...t.app_launch, exe, enabled: (t.app_launch.enabled || !hasTrigger(t, "app_launch")) && exe !== "" } });
                  }}
                />
                <datalist id="relay-processes">
                  {#each relay.processes as p (p)}<option value={p}></option>{/each}
                </datalist>
                <span class="k">Delay</span>
                <UnitField label="Delay" unit="s" value={t.app_launch.delay_ms / 1000} step={0.5} min={0} decimals={1} disabled={!editable} oncommit={(s) => relay.setTriggers({ app_launch: { ...t.app_launch, delay_ms: toMs(s) } })} />
              </span>
            {:else if k === "pixel"}
              <span class="fields">
                <span class="k">Position</span>
                <span class="xy">
                  <UnitField label="X" pre="X" value={t.pixel.x} decimals={0} disabled={!editable} oncommit={(x) => relay.setTriggers({ pixel: { ...t.pixel, x: Math.round(x) } })} />
                  <UnitField label="Y" pre="Y" value={t.pixel.y} decimals={0} disabled={!editable} oncommit={(y) => relay.setTriggers({ pixel: { ...t.pixel, y: Math.round(y) } })} />
                </span>
                <label class="k" for="t-color">Color</label>
                <span class="color field">
                  <span class="swatch" style:background={t.pixel.color}></span>
                  <input id="t-color" value={t.pixel.color} maxlength="7" spellcheck="false" disabled={!editable} onchange={setPixelColor} />
                </span>
                <span></span>
                <Key size="sm" disabled={!editable || relay.picking} onclick={relay.pickTriggerPixel}>
                  <Icon name="pick" size={14} />{relay.picking ? "Picking…" : "Pick from screen"}
                </Key>
              </span>
            {:else if k === "image"}
              <span class="line wrap">
                {#if t.image.image}<img class="thumb" src={pngUrl(t.image.image)} alt="What to watch for" />{/if}
                {#if snipping}
                  <span class="hint" role="status">Snip the image…</span>
                  <Key size="sm" onclick={relay.cancelImage}>Cancel</Key>
                {:else}
                  <Key size="sm" title="Snip it from the screen" disabled={!editable || !!relay.imaging} onclick={() => relay.setTriggerImage("snip")}><Icon name="snip" size={14} />Snip</Key>
                  <Key size="sm" title="Use the picture on the clipboard" disabled={!editable || !!relay.imaging} onclick={() => relay.setTriggerImage("paste")}><Icon name="paste" size={14} />Paste</Key>
                  <Key size="sm" disabled={!editable || !!relay.imaging} onclick={() => relay.setTriggerImage("file")}><Icon name="file" size={14} />File…</Key>
                {/if}
              </span>
              <span class="fields">
                <span class="k">Match</span>
                <UnitField label="Match" unit="%" value={t.image.threshold} step={1} min={50} max={100} decimals={0} disabled={!editable} oncommit={(v) => relay.setTriggerImageOptions({ threshold: Math.round(v) })} />
                <span class="k">Test</span>
                <span class="line">
                  <Key size="sm" disabled={!t.image.image || !editable} title="Look for it on the screen now" onclick={relay.testTriggerImage}><Icon name="search" size={14} />Test</Key>
                  {#if test?.found}<Key size="sm" title="Mark it on the screen for 3 s" onclick={relay.showImageTest}><Icon name="eye" size={14} />Show</Key>{/if}
                </span>
                {#if test}<span class="full hint" role="status">{test.text}</span>{/if}
                {#if monitors.length > 1}
                  <span class="k" id="t-area">Looks on</span>
                  <Choice labelledby="t-area" options={areas.options} value={areas.value} onchange={(i) => i !== -2 && relay.setTriggerImageOptions({ area: i < 0 ? null : monitors[i].rect })} />
                {/if}
              </span>
            {/if}
            <span class="remove">
              <Key size="sm" variant="ghost" disabled={!editable} onclick={() => remove(k)}>Remove</Key>
            </span>
          </div>
        {:else}
          <p class="hint">It runs only when you play it. Add a hotkey, a schedule, or something to watch for.</p>
        {/each}
        {#if missing.length}
          <span bind:this={addKey}>
            <Key size="sm" aria-haspopup="menu" disabled={!editable} onclick={() => (adding = true)}><Icon name="plus" size={14} />Add</Key>
          </span>
        {/if}
      {/if}
    </div>
  {/if}
</div>
{#if adding && addKey}
  <Menu items={ADD} opener={addKey.querySelector("button")!} label="Add a trigger" onselect={add} onclose={() => (adding = false)} />
{/if}

<style>
  .pane {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .spec,
  .fields {
    display: grid;
    grid-template-columns: 96px minmax(0, 1fr);
    gap: 8px 10px;
    align-items: center;
  }
  .spec {
    padding: 4px 0 8px;
  }
  .k {
    font-size: 13px;
    color: var(--text-2);
  }
  .full {
    grid-column: 1 / -1;
  }
  .line {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
  }
  .wrap {
    flex-wrap: wrap;
  }
  .data {
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 0;
    overflow-wrap: anywhere;
  }
  .hint {
    font-size: 13px;
    color: var(--text-2);
  }
  .wrong {
    color: var(--red-text);
  }
  .triggers {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 4px 0 8px;
  }
  .card {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 10px 12px;
    border-radius: var(--r-md);
    background: var(--plate-2);
    box-shadow: inset 0 0 0 1px var(--line);
  }
  .top {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .what {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .what span {
    font-size: 12px;
    color: var(--text-2);
  }
  .what span.wrong {
    color: var(--red-text);
  }
  .sentence {
    font-weight: 600;
  }
  .days {
    display: flex;
    gap: 2px;
  }
  .days button {
    width: 26px;
    height: 26px;
    border: 0;
    border-radius: var(--r-sm);
    font: inherit;
    font-size: 12px;
    font-weight: 700;
    color: var(--text);
    background: var(--plate);
    box-shadow: 0 0 0 1px var(--line-strong);
    cursor: pointer;
  }
  .days button[aria-pressed="true"] {
    color: var(--paper);
    background: var(--ink);
    box-shadow: none;
  }
  :global([data-theme="dark"]) .days button[aria-pressed="true"] {
    color: var(--ink);
    background: var(--paper);
  }
  .time {
    width: 96px;
  }
  .xy {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 6px;
  }
  .color {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .color input {
    flex: 1;
    min-width: 0;
    border: 0;
    font: inherit;
    color: inherit;
    background: none;
    outline: none;
  }
  .swatch {
    width: 14px;
    height: 14px;
    border-radius: 3px;
    box-shadow: 0 0 0 1px var(--line-strong);
  }
  .thumb {
    max-width: 96px;
    max-height: 48px;
    border-radius: var(--r-xs);
    box-shadow: 0 0 0 1px var(--line);
  }
  .remove {
    display: flex;
    justify-content: flex-end;
  }
  .notice {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 10px;
    border-radius: var(--r-md);
    font-size: 13px;
    background: var(--plate-2);
  }
  .notice span {
    flex: 1;
  }
</style>
