<script lang="ts">
  // The inspector's Step tab: the selected step's fields, with their labels and
  // units in view; what several selected steps share; the step playing; and
  // what happened to a step that timed out in the last run. Fields commit on
  // change and then show what's saved (rounded to whole ms and pixels, or
  // clamped, or put back when refused).
  import Key from "../kit/Key.svelte";
  import Glyph from "../kit/Glyph.svelte";
  import Choice from "../kit/Choice.svelte";
  import Keycaps from "../kit/Keycaps.svelte";
  import UnitField from "../kit/UnitField.svelte";
  import Icon from "../ui/Icon.svelte";
  import { relay } from "../../lib/state/relay.svelte";
  import { category } from "../../lib/categories";
  import { rowOf, rowSteps, rowText, stepRows, type Row } from "../../lib/editor/rows";
  import { clamp, toMs } from "../../lib/fields";
  import { areaChoice, pngSize, pngUrl } from "../../lib/image";
  import type { MouseBtn, Step } from "../../lib/types";

  const HEX = /^#[0-9a-fA-F]{6}$/;
  /** A pixel check or a Find image step waits at least this long before giving up. */
  const MIN_TIMEOUT_S = 0.5;
  const BUTTONS: [MouseBtn, string][] = [
    ["Left", "Left"],
    ["Right", "Right"],
    ["Middle", "Middle"],
  ];
  /** Placeholders a Text step can use, with what they type. */
  const PLACEHOLDERS: [string, string][] = [
    ["{date}", "Today's date: 2026-10-01"],
    ["{time}", "The time: 14:05:09"],
    ["{clipboard}", "The text on the clipboard"],
    ["{n}", "The repeat number: 1, 2, 3…"],
  ];
  const LABELLED: Step["kind"][] = ["click", "drag", "wait", "pixel_wait", "find_image"];

  const steps = $derived(relay.steps);
  const rows = $derived(stepRows(steps, relay.settings.show_moves));
  const anchor = $derived(relay.playback.coord_mode === "window" ? (relay.view?.recording.anchor_window?.rect ?? null) : null);
  const textOf = (r: Row) => rowText(steps[r.index], r.move == null ? null : steps[r.move], { anchor });
  const chosen = $derived(rows.filter((r) => relay.selection.includes(r.index)));
  const row = $derived(rows[rowOf(rows, relay.selected)] ?? null);
  const step = $derived(row ? steps[row.index] : null);
  const move = $derived(row?.move != null ? steps[row.move] : step?.kind === "move" ? step : null);
  /** Where the pause before the row goes: its folded move, if it has one. */
  const first = $derived(row ? rowSteps(row)[0] : -1);
  const idle = $derived(relay.mode === "idle");
  const editing = $derived(relay.canEdit);
  const playingHere = $derived(relay.playingHere && relay.curStepIdx >= 0);
  const now = $derived(rows[rowOf(rows, relay.curStepIdx)] ?? null);
  const failed = $derived(row && relay.failedStep >= 0 && rowSteps(row).includes(relay.failedStep) ? step : null);
  const where = (x: number, y: number) =>
    anchor ? `${x - anchor.x >= 0 ? "+" : ""}${x - anchor.x}, ${y - anchor.y >= 0 ? "+" : ""}${y - anchor.y} in the window` : `${x}, ${y} px`;

  const s = (ms: number) => ms / 1000;
  const edit = relay.edit;

  function setLabel(label: string) {
    if (chosen.length > 1) relay.setLabels(chosen.map((r) => r.index), label);
    else if (row) edit({ op: "set_label", index: row.index, label });
  }

  function setPause(sec: number) {
    if (chosen.length > 1) relay.setPauses(chosen.map((r) => rowSteps(r)[0]), toMs(sec));
    else if (first >= 0) relay.setPause(first, toMs(sec));
  }

  type Pixel = { x: number; y: number; color: string; tolerance: number; timeout_ms: number };
  function updatePixel(patch: Partial<Pixel>) {
    if (step?.kind !== "pixel_wait" || !row) return;
    const { x, y, color, tolerance, timeout_ms } = step;
    edit({ op: "update_pixel_wait", index: row.index, x, y, color, tolerance, timeout_ms, ...patch });
  }

  function setColor(e: Event) {
    if (step?.kind !== "pixel_wait") return;
    const input = e.currentTarget as HTMLInputElement;
    const color = input.value.trim().toUpperCase();
    input.value = HEX.test(color) ? color : step.color;
    if (HEX.test(color) && color !== step.color) updatePixel({ color });
  }

  // — Find image —
  const monitors = $derived(relay.view?.recording.monitors ?? []);
  const areas = $derived(areaChoice(monitors, step?.kind === "find_image" ? step.area : null));
  const size = $derived(step?.kind === "find_image" ? pngSize(step.image) : [0, 0]);
  const test = $derived(
    step?.kind === "find_image" && relay.imageTest?.id === relay.view?.id && relay.imageTest?.item === step.items[0] ? relay.imageTest : null,
  );
  const snipping = $derived(!!row && relay.imaging?.index === row.index && relay.imaging.source === "snip");

  /** Clicking the image sets where to click it. */
  function setClickPoint(e: MouseEvent) {
    if (step?.kind !== "find_image" || !row || e.detail === 0) return;
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    if (!r.width || !r.height) return;
    const [w, h] = size;
    const click_x = clamp(Math.floor(((e.clientX - r.left) / r.width) * w), 0, w - 1);
    const click_y = clamp(Math.floor(((e.clientY - r.top) / r.height) * h), 0, h - 1);
    relay.updateFindImage(row.index, { click_x, click_y });
  }

  // — Text —
  let textField: HTMLTextAreaElement | undefined = $state();
  let typed: { text: string } | { error: string; saves: boolean } | null = $state(null);
  let previews = 0;
  async function preview(text: string) {
    const mine = ++previews;
    const result = await relay.previewText(text);
    if (mine === previews) typed = result;
    return result;
  }
  // The saved text, again after each edit (or undo) of it, or a new data file.
  $effect(() => {
    void relay.dataFile;
    if (step?.kind === "text") void preview(step.text);
  });
  const columns = $derived((relay.dataFile?.columns ?? []).filter((c) => c && !/[{}]/.test(c)));

  async function setText() {
    if (step?.kind !== "text" || !textField || !row) return;
    const text = textField.value;
    const result = await preview(text);
    if (("text" in result || result.saves) && text !== step.text) await relay.updateText(row.index, text);
  }

  function addPlaceholder(p: string) {
    if (!textField) return;
    const { selectionStart: a, selectionEnd: b, value } = textField;
    textField.value = value.slice(0, a) + p + value.slice(b);
    textField.focus();
    textField.setSelectionRange(a + p.length, a + p.length);
    void setText();
  }

  async function setDuration(sec: number) {
    if (!row) return;
    await edit({ op: "set_wait_duration", index: row.index, dur: toMs(sec) });
  }
</script>

{#snippet title(r: Row, heading: string)}
  {@const t = textOf(r)}
  <div class="title">
    <span class="caps"><Glyph category={category(steps[r.index].kind)} small />{heading}</span>
    <span class="name">{t.kind} · {t.what}</span>
  </div>
{/snippet}

{#snippet moveFields(m: Step, index: number, label: string)}
  <span class="k" id="f-move">{label}</span>
  <UnitField
    label={label}
    unit="s"
    value={s(m.end - m.t)}
    step={0.05}
    min={0}
    disabled={!editing || (m.kind === "move" && m.samples < 2)}
    oncommit={(v) => relay.setMoveDuration(index, toMs(v))}
  />
  <span class="full buttons">
    <Key size="sm" title="Take the wobble out of the path" disabled={!editing || (m.kind === "move" && m.samples < 2)} onclick={() => relay.smoothMove(index)}>
      <Icon name="smooth" size={14} />Smooth the path
    </Key>
    <Key size="sm" title="Make the path a straight line" disabled={!editing || (m.kind === "move" && m.samples < 2)} onclick={() => relay.straightenMove(index)}>
      <Icon name="straighten" size={14} />Straighten
    </Key>
  </span>
{/snippet}

<div class="pane">
  {#if !idle}
    <div class="notice"><Icon name="lock" size={14} /><span>Stop playback to edit steps. Speed and the other play options can still change.</span></div>
  {/if}

  {#if playingHere && now}
    {@render title(now, `Now · step ${rowOf(rows, now.index) + 1} of ${rows.length}`)}
    {#if relay.dataFile && !relay.dataFile.error}
      <div class="spec"><span class="k">Row</span><span class="num">{relay.loopIdx + 1} of {relay.dataFile.rows}</span></div>
    {/if}
  {:else if chosen.length > 1}
    <div class="title">
      <span class="caps">{chosen.length} steps</span>
      <span class="name">{chosen.length} steps selected</span>
    </div>
    <div class="spec">
      <span class="k">Pause before</span>
      <UnitField label="Pause before" unit="s" value={s(steps[rowSteps(chosen[0])[0]].pause)} step={0.05} min={0} disabled={!editing} oncommit={setPause} />
      {#if chosen.some((r) => LABELLED.includes(steps[r.index].kind))}
        <label class="k" for="f-label">Label</label>
        <input id="f-label" class="field" placeholder="For each that takes one" disabled={!editing} onchange={(e) => setLabel(e.currentTarget.value)} />
      {/if}
    </div>
    <span class="buttons">
      {#if chosen.some((r) => steps[r.index].kind === "type")}
        <Key size="sm" disabled={!editing} title="Turn the typed texts into Text steps you can change" onclick={() => relay.makeAllEditable(chosen.map((r) => r.index))}>
          <Icon name="text" size={14} />Make editable
        </Key>
      {/if}
      <Key size="sm" variant="danger" disabled={!editing} onclick={() => relay.deleteRows(chosen)}><Icon name="trash" size={14} />Delete {chosen.length} steps</Key>
    </span>
  {:else if row && step}
    {@render title(row, `${textOf(row).kind} · step ${rowOf(rows, row.index) + 1}`)}
    {#if failed?.kind === "pixel_wait"}
      <div class="notice failed" role="alert">
        <Icon name="error" size={14} /><span>Timed out in the last run: the pixel at {where(failed.x, failed.y)} wasn't {failed.color} within {s(failed.timeout_ms)} s, so the macro stopped.</span>
      </div>
    {/if}
    <div class="spec">
      {#if LABELLED.includes(step.kind) && "label" in step}
        <label class="k" for="f-label">Label</label>
        <input
          id="f-label"
          class="field"
          value={step.label}
          placeholder={step.kind === "click" ? "e.g. Save button" : "Optional"}
          disabled={!editing}
          onchange={(e) => setLabel(e.currentTarget.value)}
        />
      {/if}
      <span class="k">Pause before</span>
      <UnitField label="Pause before" unit="s" value={s(steps[first].pause)} step={0.05} min={0} disabled={!editing} oncommit={setPause} />

      {#if step.kind === "click" || step.kind === "scroll"}
        <span class="k">Position</span><span class="num">{where(step.x, step.y)}</span>
        {#if step.kind === "scroll"}<span class="k">Scrolls</span><span>{textOf(row).what}</span>{/if}
      {:else if step.kind === "drag"}
        <span class="k">From</span><span class="num">{where(step.x, step.y)}</span>
        <span class="k">To</span><span class="num">{where(step.to_x, step.to_y)}</span>
      {:else if step.kind === "keys"}
        <span class="k">Keys</span><Keycaps combo={step.combo} />
      {:else if step.kind === "type"}
        <span class="k">Types</span><span class="typed">“{step.text}”</span>
        <span class="full buttons">
          <Key size="sm" disabled={!editing} title="Turn it into a Text step you can change, with the date, the time or the clipboard" onclick={() => row && relay.makeEditable(row.index)}>
            <Icon name="text" size={14} />Make editable
          </Key>
        </span>
      {:else if step.kind === "wait"}
        <span class="k">Waits</span>
        <UnitField label="Waits" unit="s" value={s(step.dur)} step={0.1} min={0} decimals={1} disabled={!editing} oncommit={setDuration} />
      {:else if step.kind === "text"}
        <label class="k" for="f-text">Text</label>
        <textarea
          id="f-text"
          class="field full"
          rows="3"
          spellcheck="false"
          value={step.text}
          disabled={!editing}
          bind:this={textField}
          oninput={() => textField && preview(textField.value)}
          onchange={setText}
        ></textarea>
        <span class="full buttons wrap">
          {#each PLACEHOLDERS as [p, tip] (p)}
            <Key size="sm" title={tip} disabled={!editing} onclick={() => addPlaceholder(p)}>{p}</Key>
          {/each}
          {#each columns as c (c)}
            <Key size="sm" title="The “{c}” column of the data file, from this repeat's row" disabled={!editing} onclick={() => addPlaceholder(`{col:${c}}`)}>{`{col:${c}}`}</Key>
          {/each}
        </span>
        {#if typed}
          <span class="full hint" class:wrong={"error" in typed} role="status">
            {"error" in typed ? typed.error : typed.text ? `Types now: “${typed.text}”` : "Types nothing yet"}
          </span>
        {/if}
        <span class="k">Lasts</span>
        <UnitField label="Lasts" unit="s" value={s(step.dur)} step={0.1} min={0} decimals={1} disabled={!editing} oncommit={setDuration} />
      {:else if step.kind === "pixel_wait"}
        <span class="k">Position</span>
        <span class="xy">
          <UnitField label="X" pre="X" value={step.x} decimals={0} disabled={!editing} oncommit={(v) => updatePixel({ x: Math.round(v) })} />
          <UnitField label="Y" pre="Y" value={step.y} decimals={0} disabled={!editing} oncommit={(v) => updatePixel({ y: Math.round(v) })} />
        </span>
        <span></span>
        <Key size="sm" disabled={!editing || relay.picking} onclick={() => row && relay.pickPixel(row.index)}>
          <Icon name="pick" size={14} />{relay.picking ? "Picking…" : "Pick from screen"}
        </Key>
        <label class="k" for="f-color">Color</label>
        <span class="color field">
          <span class="swatch" style:background={step.color}></span>
          <input id="f-color" value={step.color} maxlength="7" spellcheck="false" disabled={!editing} onchange={setColor} />
        </span>
        <span class="k">Tolerance</span>
        <UnitField label="Tolerance" value={step.tolerance} step={1} min={0} max={255} decimals={0} disabled={!editing} oncommit={(v) => updatePixel({ tolerance: Math.round(v) })} />
        <span class="k">Gives up after</span>
        <UnitField label="Gives up after" unit="s" value={s(step.timeout_ms)} step={0.5} min={MIN_TIMEOUT_S} decimals={1} disabled={!editing} oncommit={(v) => updatePixel({ timeout_ms: toMs(v) })} />
      {:else if step.kind === "find_image"}
        <span class="full find">
          <button type="button" class="shot" title="Click the image where it should be clicked" disabled={!editing} onclick={setClickPoint}>
            <img src={pngUrl(step.image)} alt="What to find" />
            {#if size[0] > 0}
              <span class="target" style:left="{((step.click_x + 0.5) / size[0]) * 100}%" style:top="{((step.click_y + 0.5) / size[1]) * 100}%"></span>
            {/if}
          </button>
          <span class="buttons wrap">
            {#if snipping}
              <span class="hint" role="status">Snip the image…</span>
              <Key size="sm" onclick={relay.cancelImage}>Cancel</Key>
            {:else}
              <Key size="sm" title="Snip it from the screen" disabled={!editing || !!relay.imaging} onclick={() => row && relay.replaceImage(row.index, "snip")}><Icon name="snip" size={14} />Snip</Key>
              <Key size="sm" title="Use the picture on the clipboard" disabled={!editing || !!relay.imaging} onclick={() => row && relay.replaceImage(row.index, "paste")}><Icon name="paste" size={14} />Paste</Key>
              <Key size="sm" disabled={!editing || !!relay.imaging} onclick={() => row && relay.replaceImage(row.index, "file")}><Icon name="file" size={14} />File…</Key>
            {/if}
          </span>
        </span>
        <span class="k">Match</span>
        <UnitField label="Match" unit="%" value={step.threshold} step={1} min={50} max={100} decimals={0} disabled={!editing} oncommit={(v) => row && relay.updateFindImage(row.index, { threshold: Math.round(v) })} />
        <span class="k">Gives up after</span>
        <UnitField label="Gives up after" unit="s" value={s(step.timeout_ms)} step={0.5} min={MIN_TIMEOUT_S} decimals={1} disabled={!editing} oncommit={(v) => row && relay.updateFindImage(row.index, { timeout_ms: toMs(v) })} />
        <span class="k">Test</span>
        <span class="buttons wrap">
          <Key size="sm" title="Look for it on the screen now" onclick={() => row && relay.testFindImage(row.index)}><Icon name="search" size={14} />Test</Key>
          {#if test?.found}<Key size="sm" title="Mark it on the screen for 3 s, with a dot where it would be clicked" onclick={relay.showImageTest}><Icon name="eye" size={14} />Show</Key>{/if}
        </span>
        {#if test}<span class="full hint" role="status">{test.text}</span>{/if}
        <span class="k" id="f-btn">Clicks</span>
        <Choice labelledby="f-btn" options={BUTTONS} value={step.btn} disabled={!editing} onchange={(btn) => row && relay.updateFindImage(row.index, { btn })} />
        {#if monitors.length > 1}
          <span class="k" id="f-area">Looks on</span>
          <Choice
            labelledby="f-area"
            options={areas.options}
            value={areas.value}
            disabled={!editing}
            onchange={(i) => i !== -2 && row && relay.updateFindImage(row.index, { area: i < 0 ? null : monitors[i].rect })}
          />
        {/if}
      {/if}

      {#if move}
        {@render moveFields(move, row.move ?? row.index, step.kind === "move" ? "Takes" : "Moving there")}
      {/if}
    </div>
    <span class="buttons end">
      <Key size="sm" variant="danger" disabled={!editing} onclick={() => row && relay.deleteRows([row])}><Icon name="trash" size={14} />Delete step</Key>
    </span>
  {:else}
    <div class="empty">
      <b>No step selected</b>
      <span>Select a step to edit it, or open the Macro tab for how the macro plays and when it runs.</span>
    </div>
  {/if}
</div>

<style>
  .pane {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .title {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .title .caps {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .name {
    font-size: 17px;
    font-weight: 800;
    overflow-wrap: anywhere;
  }
  .spec {
    display: grid;
    grid-template-columns: 108px minmax(0, 1fr);
    gap: 8px 10px;
    align-items: center;
  }
  .k {
    font-size: 13px;
    color: var(--text-2);
  }
  .full {
    grid-column: 1 / -1;
  }
  .buttons {
    display: flex;
    gap: 6px;
  }
  .wrap {
    flex-wrap: wrap;
  }
  .end {
    margin-top: auto;
  }
  .xy {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 6px;
  }
  .typed {
    overflow-wrap: anywhere;
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
  .hint {
    font-size: 13px;
    color: var(--text-2);
  }
  .hint.wrong {
    color: var(--red-text);
  }
  .find {
    display: flex;
    gap: 10px;
    align-items: flex-start;
  }
  .shot {
    position: relative;
    flex: none;
    max-width: 50%;
    padding: 0;
    border: 0;
    border-radius: var(--r-sm);
    background: var(--plate-2);
    box-shadow: 0 0 0 1px var(--line);
    cursor: crosshair;
    overflow: hidden;
  }
  .shot img {
    display: block;
    max-width: 100%;
    max-height: 96px;
  }
  .target {
    position: absolute;
    width: 10px;
    height: 10px;
    margin: -5px 0 0 -5px;
    border-radius: 50%;
    box-shadow:
      0 0 0 1.5px #fff,
      0 0 0 3px var(--red);
    pointer-events: none;
  }
  .notice {
    display: flex;
    gap: 8px;
    padding: 10px 12px;
    border-radius: var(--r-md);
    font-size: 13px;
    background: var(--plate-2);
  }
  .notice.failed {
    color: var(--red-text);
    background: color-mix(in srgb, var(--red) 10%, transparent);
  }
  .notice :global(svg) {
    flex: none;
    margin-top: 2px;
  }
  .empty {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 24px 0;
    text-align: center;
    color: var(--text-2);
  }
  .empty b {
    font-size: 15px;
    color: var(--text);
  }
</style>
