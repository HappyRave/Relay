<script lang="ts">
  // Inline editor under the selected step: the pause before it, labels, wait
  // durations and the pixel check's position, color, tolerance and timeout.
  import { relay } from "../../../lib/state/relay.svelte";
  import type { Step } from "../../../lib/types";

  let { step, index }: { step: Step; index: number } = $props();

  const HEX = /^#[0-9a-fA-F]{6}$/;

  function setLabel(label: string) {
    relay.edit({ op: "set_label", index, label });
  }

  function setWait(seconds: number) {
    if (Number.isFinite(seconds) && seconds >= 0) relay.edit({ op: "set_wait_duration", index, dur: Math.round(seconds * 1000) });
  }

  function updatePixel(patch: Partial<{ x: number; y: number; color: string; tolerance: number; timeout_ms: number }>) {
    if (step.kind !== "pixel_wait") return;
    const next = { x: step.x, y: step.y, color: step.color, tolerance: step.tolerance, timeout_ms: step.timeout_ms, ...patch };
    if (!HEX.test(next.color) || ![next.x, next.y, next.tolerance, next.timeout_ms].every(Number.isFinite)) return;
    relay.edit({ op: "update_pixel_wait", index, ...next, color: next.color.toUpperCase() });
  }

  const num = (e: Event) => Number((e.currentTarget as HTMLInputElement).value);

  function setPause(seconds: number) {
    if (Number.isFinite(seconds) && seconds >= 0) relay.setPause(index, seconds * 1000);
  }
</script>

<div class="editor" role="group" aria-label="Edit step">
  <div class="grid">
    <label class="pause" title="Idle time before this step, while only the mouse moves">
      Pause before s
      <input class="input" type="number" min="0" step="0.1" value={(step.pause / 1000).toFixed(1)} onchange={(e) => setPause(num(e))} />
    </label>
  </div>
  {#if step.kind === "pixel_wait"}
    <div class="grid">
      <label>X<input class="input" type="number" value={step.x} onchange={(e) => updatePixel({ x: num(e) })} /></label>
      <label>Y<input class="input" type="number" value={step.y} onchange={(e) => updatePixel({ y: num(e) })} /></label>
      <label class="color">
        Color
        <span class="field">
          <span class="swatch" style:background={step.color}></span>
          <input
            class="input"
            value={step.color}
            maxlength="7"
            spellcheck="false"
            onchange={(e) => {
              const color = e.currentTarget.value.trim();
              if (HEX.test(color)) updatePixel({ color });
              else if (step.kind === "pixel_wait") e.currentTarget.value = step.color;
            }}
          />
        </span>
      </label>
      <label>
        Tolerance
        <input class="input" type="number" min="0" max="255" value={step.tolerance} onchange={(e) => updatePixel({ tolerance: Math.min(255, Math.max(0, num(e))) })} />
      </label>
      <label>
        Timeout s
        <input class="input" type="number" min="0.5" step="0.5" value={step.timeout_ms / 1000} onchange={(e) => updatePixel({ timeout_ms: Math.round(num(e) * 1000) })} />
      </label>
      <button class="btn btn-secondary pick" disabled={relay.picking > 0} onclick={() => relay.pickPixel(index)}>
        {relay.picking > 0 ? `Point at it… ${relay.picking}` : "Pick"}
      </button>
    </div>
  {:else if step.kind === "wait"}
    <div class="grid">
      <label>
        Duration s
        <input class="input" type="number" min="0" step="0.1" value={step.dur / 1000} onchange={(e) => setWait(num(e))} />
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
  .pick {
    min-height: 26px;
    padding: 2px 8px;
    font-size: 12px;
    justify-content: flex-start;
  }
  .label {
    width: 100%;
  }
</style>
