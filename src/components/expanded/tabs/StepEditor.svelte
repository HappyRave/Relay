<script lang="ts">
  // Inline editor under the selected step: the pause before it, labels, wait
  // and move durations, smoothing a move, the pixel check's position, color,
  // tolerance and timeout, the Find image step's image, click point,
  // button, match, timeout and search area, and the Text step's text.
  // Number fields commit on change and then show what's saved: rounded to
  // what Rust stores (whole ms and pixels), clamped, or put back if refused.
  import Segmented from "../../ui/Segmented.svelte";
  import { relay } from "../../../lib/state/relay.svelte";
  import { clamp, commitNumber, toMs } from "../../../lib/fields";
  import { areaChoice, pngSize, pngUrl } from "../../../lib/image";
  import type { MouseBtn, Step } from "../../../lib/types";

  let { step, index }: { step: Step; index: number } = $props();

  const HEX = /^#[0-9a-fA-F]{6}$/;
  /** A pixel check waits at least this long before giving up. */
  const MIN_TIMEOUT_MS = 500;

  type Pixel = { x: number; y: number; color: string; tolerance: number; timeout_ms: number };
  const field = (e: Event) => e.currentTarget as HTMLInputElement;
  const seconds = (ms: number) => String(ms / 1000);
  const positiveMs = (s: number) => (s >= 0 ? toMs(s) : null);

  function setLabel(label: string) {
    relay.edit({ op: "set_label", index, label });
  }

  function updatePixel(patch: Partial<Pixel>) {
    if (step.kind !== "pixel_wait") return;
    const { x, y, color, tolerance, timeout_ms } = step;
    relay.edit({ op: "update_pixel_wait", index, x, y, color, tolerance, timeout_ms, ...patch });
  }

  /** Commits one of the pixel check's number fields. */
  function pixelNumber(e: Event, key: Exclude<keyof Pixel, "color">, accept: (v: number) => number | null, show?: (v: number) => string) {
    if (step.kind !== "pixel_wait") return;
    const v = commitNumber(field(e), step[key], accept, show);
    if (v != null) updatePixel({ [key]: v });
  }

  function setColor(e: Event) {
    if (step.kind !== "pixel_wait") return;
    const input = field(e);
    const color = input.value.trim().toUpperCase();
    input.value = HEX.test(color) ? color : step.color;
    if (HEX.test(color) && color !== step.color) updatePixel({ color });
  }

  function setPause(e: Event) {
    const ms = commitNumber(field(e), step.pause, positiveMs, (ms) => (ms / 1000).toFixed(1));
    if (ms != null) relay.setPause(index, ms);
  }

  function setMoveDuration(e: Event) {
    if (step.kind !== "move") return;
    const ms = commitNumber(field(e), step.end - step.t, positiveMs, (ms) => (ms / 1000).toFixed(2));
    if (ms != null) relay.setMoveDuration(index, ms);
  }

  const BUTTONS: [MouseBtn, string][] = [
    ["Left", "Left"],
    ["Right", "Right"],
    ["Middle", "Middle"],
  ];
  const monitors = $derived(relay.view?.recording.monitors ?? []);
  const areas = $derived(areaChoice(monitors, step.kind === "find_image" ? step.area : null));
  const size = $derived(step.kind === "find_image" ? pngSize(step.image) : [0, 0]);
  const test = $derived(
    step.kind === "find_image" && relay.imageTest?.id === relay.view?.id && relay.imageTest?.item === step.items[0]
      ? relay.imageTest
      : null,
  );
  const snipping = $derived(relay.imaging?.index === index && relay.imaging.source === "snip");

  /** Commits one of the Find image step's number fields. */
  function findNumber(e: Event, key: "threshold" | "timeout_ms", accept: (v: number) => number | null, show?: (v: number) => string) {
    if (step.kind !== "find_image") return;
    const v = commitNumber(field(e), step[key], accept, show);
    if (v != null) relay.updateFindImage(index, { [key]: v });
  }

  /** Clicking the image sets where to click it. */
  function setClickPoint(e: MouseEvent) {
    // A keyboard press has no position.
    if (step.kind !== "find_image" || e.detail === 0) return;
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    if (!r.width || !r.height) return;
    const [w, h] = size;
    const click_x = clamp(Math.floor(((e.clientX - r.left) / r.width) * w), 0, w - 1);
    const click_y = clamp(Math.floor(((e.clientY - r.top) / r.height) * h), 0, h - 1);
    relay.updateFindImage(index, { click_x, click_y });
  }

  function setArea(i: number) {
    if (i === -2) return;
    relay.updateFindImage(index, { area: i < 0 ? null : monitors[i].rect });
  }

  /** Placeholders a Text step can use, with what they type. */
  const PLACEHOLDERS: [string, string][] = [
    ["{date}", "Today's date: 2026-10-01"],
    ["{time}", "The time: 14:05:09"],
    ["{clipboard}", "The text on the clipboard"],
    ["{n}", "The repeat number: 1, 2, 3…"],
  ];
  let textField: HTMLTextAreaElement | undefined = $state();
  /** What the text in the field would type now, or what's wrong with it. */
  let typed: { text: string } | { error: string } | null = $state(null);
  let previews = 0;

  async function preview(text: string) {
    const mine = ++previews;
    const result = await relay.previewText(text);
    if (mine === previews) typed = result;
    return result;
  }

  // The saved text, again after each edit (or undo) of it.
  $effect(() => {
    if (step.kind === "text") void preview(step.text);
  });

  /** Saves the text in the field, unless it has a mistake (shown under it). */
  async function setText() {
    if (step.kind !== "text" || !textField) return;
    const text = textField.value;
    const result = await preview(text);
    if ("text" in result && step.kind === "text" && text !== step.text) await relay.updateText(index, text);
  }

  /** Puts a placeholder where the caret is, and saves. */
  function addPlaceholder(p: string) {
    if (!textField) return;
    const { selectionStart: a, selectionEnd: b, value } = textField;
    textField.value = value.slice(0, a) + p + value.slice(b);
    textField.focus();
    textField.setSelectionRange(a + p.length, a + p.length);
    void setText();
  }

  async function setTextDuration(e: Event) {
    if (step.kind !== "text") return;
    const input = field(e);
    const dur = commitNumber(input, step.dur, positiveMs, seconds);
    if (dur == null) return;
    await relay.edit({ op: "set_wait_duration", index, dur });
    // It can't be shorter than the typing: Rust may have kept more.
    if (step.kind === "text") input.value = seconds(step.dur);
  }

  function setWait(e: Event) {
    if (step.kind !== "wait") return;
    const dur = commitNumber(field(e), step.dur, positiveMs, seconds);
    if (dur != null) relay.edit({ op: "set_wait_duration", index, dur });
  }
</script>

<div class="editor" role="group" aria-label="Edit step">
  <div class="grid">
    <label class="pause" title="Idle time before this step, when nothing happens">
      Pause before s
      <input class="input" type="number" min="0" step="0.1" value={(step.pause / 1000).toFixed(1)} onchange={setPause} />
    </label>
  </div>
  {#if step.kind === "pixel_wait"}
    <div class="grid">
      <label>X<input class="input" type="number" value={step.x} onchange={(e) => pixelNumber(e, "x", Math.round)} /></label>
      <label>Y<input class="input" type="number" value={step.y} onchange={(e) => pixelNumber(e, "y", Math.round)} /></label>
      <label class="color">
        Color
        <span class="field">
          <span class="swatch" style:background={step.color}></span>
          <input
            class="input"
            value={step.color}
            maxlength="7"
            spellcheck="false"
            onchange={setColor}
          />
        </span>
      </label>
      <label>
        Tolerance
        <input class="input" type="number" min="0" max="255" value={step.tolerance} onchange={(e) => pixelNumber(e, "tolerance", (v) => clamp(Math.round(v), 0, 255))} />
      </label>
      <label>
        Timeout s
        <input class="input" type="number" min="0.5" step="0.5" value={step.timeout_ms / 1000} onchange={(e) => pixelNumber(e, "timeout_ms", (s) => Math.max(MIN_TIMEOUT_MS, toMs(s)), seconds)} />
      </label>
      <button class="btn btn-secondary pick" disabled={relay.picking > 0} onclick={() => relay.pickPixel(index)}>
        {relay.picking > 0 ? `Point at it… ${relay.picking}` : "Pick"}
      </button>
    </div>
  {:else if step.kind === "find_image"}
    <div class="find">
      <button class="shot" title="Click the image where it should be clicked" onclick={setClickPoint}>
        <img src={pngUrl(step.image)} alt="What to find" />
        {#if size[0] > 0}
          <span class="target" style:left="{((step.click_x + 0.5) / size[0]) * 100}%" style:top="{((step.click_y + 0.5) / size[1]) * 100}%"
          ></span>
        {/if}
      </button>
      <div class="sources">
        {#if snipping}
          <span class="hint" role="status">Snip the image…</span>
          <button class="btn btn-secondary tool" onclick={relay.cancelImage}>Cancel</button>
        {:else}
          <button class="btn btn-secondary tool" disabled={!!relay.imaging} title="Snip it from the screen" onclick={() => relay.replaceImage(index, "snip")}>Snip</button>
          <button class="btn btn-secondary tool" disabled={!!relay.imaging} title="Use the picture on the clipboard" onclick={() => relay.replaceImage(index, "paste")}>Paste</button>
          <button class="btn btn-secondary tool" disabled={!!relay.imaging} onclick={() => relay.replaceImage(index, "file")}>File…</button>
        {/if}
      </div>
    </div>
    <div class="grid">
      <label>
        Match %
        <input class="input" type="number" min="50" max="100" value={step.threshold} onchange={(e) => findNumber(e, "threshold", (v) => clamp(Math.round(v), 50, 100))} />
      </label>
      <label>
        Timeout s
        <input class="input" type="number" min="0.5" step="0.5" value={step.timeout_ms / 1000} onchange={(e) => findNumber(e, "timeout_ms", (s) => Math.max(MIN_TIMEOUT_MS, toMs(s)), seconds)} />
      </label>
      <button class="btn btn-secondary pick" title="Look for it on the screen now" onclick={() => relay.testFindImage(index)}>Test</button>
    </div>
    {#if test}
      <div class="test">
        <span role="status">{test.text}</span>
        {#if test.found}
          <button class="btn btn-secondary tool" title="Mark it on the screen for 3 s, with a dot where it would be clicked" onclick={relay.showImageTest}>Show</button>
        {/if}
      </div>
    {/if}
    <div class="choice">
      <span>Click</span>
      <Segmented label="Button to click" options={BUTTONS} value={step.btn} onchange={(btn) => relay.updateFindImage(index, { btn })} />
    </div>
    {#if monitors.length > 1}
      <div class="choice">
        <span>Look on</span>
        <Segmented label="Where to look" options={areas.options} value={areas.value} onchange={setArea} />
      </div>
    {/if}
  {:else if step.kind === "move"}
    <!-- One sample is a jump: no length to set, no path to reshape. -->
    <div class="grid">
      <label title="How long the move takes: shorter is faster">
        Duration s
        <input
          class="input"
          type="number"
          min="0"
          step="0.05"
          value={((step.end - step.t) / 1000).toFixed(2)}
          disabled={step.samples < 2}
          onchange={setMoveDuration}
        />
      </label>
      <button
        class="btn btn-secondary tool"
        title="Take the wobble out of the path"
        disabled={step.samples < 2}
        onclick={() => relay.smoothMove(index)}>Smooth</button
      >
      <button
        class="btn btn-secondary tool"
        title="Make the path a straight line"
        disabled={step.samples < 2}
        onclick={() => relay.straightenMove(index)}>Straighten</button
      >
    </div>
  {:else if step.kind === "text"}
    <label class="label">
      Text
      <textarea
        class="input text"
        rows="2"
        spellcheck="false"
        value={step.text}
        bind:this={textField}
        oninput={() => textField && preview(textField.value)}
        onchange={setText}
      ></textarea>
    </label>
    <div class="sources">
      {#each PLACEHOLDERS as [p, title] (p)}
        <button class="btn btn-secondary tool" {title} onclick={() => addPlaceholder(p)}>{p}</button>
      {/each}
    </div>
    {#if typed}
      <div class="test" class:wrong={"error" in typed} role="status">
        {"error" in typed ? typed.error : typed.text ? `Types now: “${typed.text}”` : "Types nothing yet"}
      </div>
    {/if}
    <div class="grid">
      <label title="How long the step lasts: what follows waits for the typing">
        Duration s
        <input class="input" type="number" min="0" step="0.1" value={step.dur / 1000} onchange={setTextDuration} />
      </label>
    </div>
  {:else if step.kind === "type"}
    <div class="grid">
      <button
        class="btn btn-secondary tool"
        title="Turn it into a Text step you can change, with the date, the time or the clipboard"
        onclick={() => relay.makeEditable(index)}>Make editable</button
      >
    </div>
  {:else if step.kind === "wait"}
    <div class="grid">
      <label>
        Duration s
        <input class="input" type="number" min="0" step="0.1" value={step.dur / 1000} onchange={setWait} />
      </label>
    </div>
  {/if}
  {#if step.kind === "click" || step.kind === "drag" || step.kind === "wait" || step.kind === "pixel_wait" || step.kind === "find_image"}
    <label class="label">
      Label
      <input
        class="input"
        value={step.label}
        placeholder={step.kind === "click" ? "e.g. Save button" : "Optional"}
        onchange={(e) => setLabel(e.currentTarget.value)}
      />
    </label>
  {/if}
</div>

<style>
  /* Two columns for the label, one for the field, like the fields below. */
  .pause {
    grid-column: span 2;
    white-space: nowrap;
  }
  .pause input {
    width: calc(50% - 4px);
  }
  .editor {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 6px 12px 10px 12px;
    background: var(--color-accent-100);
    border-bottom: 1px solid var(--color-neutral-300);
    cursor: default;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 6px 8px;
    align-items: end;
  }
  label {
    display: flex;
    flex-direction: column;
    gap: 2px;
    font-size: 10px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    font-weight: 600;
    color: var(--color-neutral-700);
    min-width: 0;
  }
  .input {
    min-height: 26px;
    padding: 2px 6px;
    font-size: 12px;
    background: var(--color-bg);
    text-transform: none;
    letter-spacing: 0;
  }
  .color {
    grid-column: span 1;
  }
  .field {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .swatch {
    width: 18px;
    height: 18px;
    flex: none;
    border: 1px solid var(--color-text);
  }
  .pick,
  .tool {
    min-height: 26px;
    padding: 2px 8px;
    font-size: 12px;
    justify-content: flex-start;
  }
  .label {
    width: 100%;
  }
  .find {
    display: flex;
    gap: 8px;
    align-items: flex-start;
  }
  /* The image at its size, down to fit; a checkerboard shows where it ends. */
  .shot {
    position: relative;
    display: block;
    flex: none;
    max-width: 60%;
    padding: 0;
    border: 1px solid var(--color-text);
    background: repeating-conic-gradient(var(--color-neutral-300) 0 25%, var(--color-bg) 0 50%) 0 0 / 8px 8px;
    cursor: crosshair;
  }
  .shot img {
    display: block;
    max-width: 100%;
    max-height: 96px;
  }
  .target {
    position: absolute;
    width: 11px;
    height: 11px;
    margin: -6px 0 0 -6px;
    border: 2px solid var(--color-accent);
    border-radius: 50%;
    pointer-events: none;
  }
  .sources {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    align-items: center;
  }
  .hint,
  .test {
    font-size: 12px;
    color: var(--color-neutral-700);
  }
  .test.wrong {
    color: var(--color-accent-700);
  }
  textarea.text {
    resize: vertical;
    font-family: inherit;
    white-space: pre-wrap;
  }
  .test {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .choice {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 10px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    font-weight: 600;
    color: var(--color-neutral-700);
  }
  .choice span {
    width: 52px;
  }
</style>
