<script lang="ts">
  import Icon from "../../ui/Icon.svelte";
  import StepEditor from "./StepEditor.svelte";
  import { tick } from "svelte";
  import { relay, TRIM_PAUSE_MS } from "../../../lib/state/relay.svelte";
  import { fmtTime, plural } from "../../../lib/format";
  import { stepTitle } from "../../../lib/state/display";
  import type { Step } from "../../../lib/types";

  const TYPE_NAME: Record<Step["kind"], string> = {
    click: "CLICK",
    drag: "DRAG",
    scroll: "SCROLL",
    keys: "KEYS",
    type: "TYPE",
    text: "TEXT",
    wait: "WAIT",
    pixel_wait: "IF",
    find_image: "FIND",
    move: "MOVE",
  };

  const steps = $derived(relay.steps);
  const curIdx = $derived(relay.curStepIdx);
  const anchor = $derived(relay.view?.recording.anchor_window?.rect);

  const inWindow = $derived(relay.playback.coord_mode === "window" && !!anchor);
  const signed = (n: number) => (n < 0 ? `${n}` : `+${n}`);
  /** A position as played: on the screen, or relative to the anchor window. */
  const at = (x: number, y: number) => (inWindow && anchor ? `${signed(x - anchor.x)}, ${signed(y - anchor.y)}` : `${x}, ${y}`);
  const unit = $derived(inWindow ? " in window" : " px");
  const where = (x: number, y: number) => at(x, y) + unit;

  /** "1 notch", "2 notches", "0.5 notch": a wheel notch is 120; precise wheels send less. */
  function notches(delta: number): string {
    const n = Math.abs(delta) / 120;
    const shown = Number.isInteger(n) ? String(n) : n < 0.05 ? n.toPrecision(1) : n.toFixed(1);
    return `${shown} ${Number(shown) > 1 ? "notches" : "notch"}`;
  }

  function describe(s: Step): [string, string] {
    switch (s.kind) {
      case "click":
        return [stepTitle(s), where(s.x, s.y)];
      case "drag":
        return [
          stepTitle(s),
          inWindow ? `${at(s.x, s.y)} → ${at(s.to_x, s.to_y)}${unit}` : `${where(s.x, s.y)} → ${s.to_x}, ${s.to_y}`,
        ];
      case "scroll":
        return [stepTitle(s), `${notches(s.delta)} at ${where(s.x, s.y)}`];
      case "keys":
        return [stepTitle(s), "Key combination"];
      case "type":
        return [stepTitle(s), plural(s.text.length, "character")];
      case "text":
        return [stepTitle(s), "Filled in when it plays"];
      case "pixel_wait":
        return [stepTitle(s), (s.label ? s.label + " · " : "") + `timeout ${s.timeout_ms / 1000} s, else stop`];
      case "find_image":
        return [stepTitle(s), `${s.btn} click when found · timeout ${s.timeout_ms / 1000} s, else stop`];
      case "wait":
        return [stepTitle(s), s.label];
      case "move": {
        const path = inWindow ? `${at(s.x, s.y)} → ${at(s.to_x, s.to_y)}${unit}` : `${s.x}, ${s.y} → ${where(s.to_x, s.to_y)}`;
        return [stepTitle(s), `${path} · ${((s.end - s.t) / 1000).toFixed(2)} s`];
      }
    }
  }

  /** Pauses longer than this get a marker above their step (the length Trim pauses leaves). */
  const SHOW_PAUSE_MS = TRIM_PAUSE_MS;

  /** The row whose editor is open (the store keeps it on its step across edits); none for live steps. */
  const open = $derived(relay.mode === "recording" ? -1 : relay.selected);

  async function choose(i: number) {
    relay.selectStep(i);
    // Bring a newly opened editor into view.
    await tick();
    if (open === i) list?.children[i]?.scrollIntoView({ block: "nearest" });
  }

  let list: HTMLDivElement | undefined = $state();
  // Keep the active row in view while playing.
  $effect(() => {
    if (relay.mode !== "playing" || curIdx < 0 || !list) return;
    list.children[curIdx]?.scrollIntoView({ block: "nearest" });
  });
</script>

<div class="bar">
  <span class="count">{plural(steps.length, "step")}</span>
  <button class="btn btn-ghost" disabled={!relay.canEdit} onclick={relay.insertWait}>+ Wait</button>
  <button
    class="btn btn-ghost"
    disabled={!relay.canEdit}
    title="Type a text that can change each run: the date, the time, the clipboard"
    onclick={relay.insertText}>+ Type text</button
  >
  <button
    class="btn btn-ghost"
    disabled={!relay.canEdit}
    title="Wait until the pixel under the cursor matches"
    onclick={relay.insertPixelCheck}>+ Pixel check</button
  >
  <button
    class="btn btn-ghost"
    disabled={!relay.canEdit || !!relay.imaging}
    title="Snip an image: when it's on screen, click it"
    onclick={() => relay.insertFindImage()}>+ Find image</button
  >
  <button
    class="btn btn-ghost"
    disabled={!relay.canEdit || relay.longPauses === 0}
    title="Shorten every pause longer than 1 s to 1 s"
    onclick={relay.trimPauses}>Trim pauses</button
  >
</div>
{#if relay.imaging?.index === -1}
  <div class="imaging" role="status">
    {#if relay.imaging.source === "snip"}
      <span>Snip the image to find, or</span>
      <button class="btn btn-ghost" onclick={() => relay.insertFindImage("paste")}>Paste</button>
      <button class="btn btn-ghost" onclick={() => relay.insertFindImage("file")}>File…</button>
      <button class="btn btn-ghost" onclick={relay.cancelImage}>Cancel</button>
    {:else}
      <span>{relay.imaging.source === "paste" ? "Reading the clipboard…" : "Choose an image file…"}</span>
    {/if}
  </div>
{/if}
<div class="list" bind:this={list}>
  {#each steps as s, i (s.items[0] ?? i)}
    {@const [detail, sub] = describe(s)}
    <div class="item">
      {#if s.pause > SHOW_PAUSE_MS && relay.mode !== "recording"}
        <div class="pause" aria-hidden="true">{(s.pause / 1000).toFixed(1)} s pause</div>
      {/if}
      <div
        class="row"
        class:active={i === curIdx}
        class:selected={i === open}
        class:future={i > curIdx}
        role="button"
        tabindex="0"
        aria-expanded={i === open}
        onclick={() => choose(i)}
        onkeydown={(e) => {
          // Only the row itself; Enter on its delete button deletes.
          if (e.target !== e.currentTarget || (e.key !== "Enter" && e.key !== " ")) return;
          e.preventDefault();
          choose(i);
        }}
      >
        <span class="time">{fmtTime(s.t)}</span>
        <span class="type">{TYPE_NAME[s.kind]}</span>
        <div class="text">
          <div class="detail">
            {#if s.kind === "pixel_wait"}<span class="swatch" style:background={s.color}></span>{/if}{detail}
          </div>
          <div class="sub">{sub}</div>
        </div>
        <button
          class="del"
          title="Delete step"
          aria-label="Delete step"
          disabled={!relay.canEdit}
          onclick={(e) => {
            e.stopPropagation();
            relay.deleteStep(i);
          }}><Icon name="x" size={14} /></button
        >
      </div>
      {#if i === open && relay.mode === "idle" && relay.editable}
        <StepEditor step={s} index={i} />
      {/if}
    </div>
  {/each}
</div>

<style>
  .bar {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    border-bottom: 1px solid var(--color-divider);
  }
  .count {
    font-size: 12px;
    color: var(--color-neutral-700);
    margin-right: auto;
  }
  .bar .btn,
  .imaging .btn {
    font-size: 12px;
    padding: 4px 6px;
  }
  .imaging {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 12px;
    font-size: 12px;
    background: var(--color-accent-100);
    border-bottom: 1px solid var(--color-divider);
  }
  .list {
    flex: 1;
    overflow-y: auto;
  }
  .row {
    display: grid;
    grid-template-columns: 58px 44px minmax(0, 1fr) 28px;
    align-items: center;
    gap: 8px;
    padding: 6px 0 6px 12px;
    border-bottom: 1px solid var(--color-neutral-300);
    cursor: pointer;
    color: var(--color-text);
  }
  .row:hover {
    background: var(--color-neutral-200);
  }
  .row.future {
    color: var(--color-neutral-600);
  }
  .row.active,
  .row.selected {
    background: var(--color-accent-100);
  }
  .swatch {
    display: inline-block;
    width: 10px;
    height: 10px;
    margin-right: 6px;
    border: 1px solid var(--color-text);
    vertical-align: -1px;
  }
  .row:focus-visible {
    outline-offset: -2px;
  }
  .time {
    font-size: 11px;
    font-variant-numeric: tabular-nums;
  }
  .type {
    font-size: 9px;
    letter-spacing: 0.08em;
    font-weight: 800;
    color: var(--color-neutral-700);
  }
  .active .type {
    color: var(--color-accent-700);
  }
  .text {
    min-width: 0;
  }
  .detail {
    font-size: 13px;
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .sub {
    font-size: 11px;
    color: var(--color-neutral-600);
  }
  .pause {
    padding: 2px 12px 2px 130px;
    font-size: 10px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--color-neutral-600);
    background: repeating-linear-gradient(-45deg, transparent 0 6px, var(--color-neutral-200) 6px 8px);
    border-bottom: 1px solid var(--color-neutral-300);
  }
  .del {
    width: 24px;
    height: 24px;
    border: 0;
    background: transparent;
    color: var(--color-neutral-600);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0;
  }
  .del:hover {
    color: var(--color-accent);
  }
  .del:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }
</style>
