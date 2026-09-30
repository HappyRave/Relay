<script lang="ts">
  // Inline editor under the selected step: the pause before it, labels, wait
  // and move durations, smoothing a move, and the pixel check's position,
  // color, tolerance and timeout.
  // Number fields commit on change and then show what's saved: rounded to
  // what Rust stores (whole ms and pixels), clamped, or put back if refused.
  import { relay } from "../../../lib/state/relay.svelte";
  import { clamp, commitNumber, toMs } from "../../../lib/fields";
  import type { Step } from "../../../lib/types";

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
  {:else if step.kind === "wait"}
    <div class="grid">
      <label>
        Duration s
        <input class="input" type="number" min="0" step="0.1" value={step.dur / 1000} onchange={setWait} />
      </label>
    </div>
  {/if}
  {#if step.kind === "click" || step.kind === "drag" || step.kind === "wait" || step.kind === "pixel_wait"}
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
</style>
