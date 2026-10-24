<script lang="ts">
  // The preview: the screen the macro was recorded on (its screenshot, or the
  // outlines of the monitors and the window), the mouse path, and a numbered
  // callout beside each click: filled once played, white when selected, with
  // its label. The wheel or + and − zoom, a drag or the arrows pan, and a
  // double-click or 0 fits it again.
  import { relay } from "../../../lib/state/relay.svelte";
  import { cumulativeLengths, lastIndexAtOrBefore, pan, pathD, PREVIEW_ASPECT, zoomAround, zoomedView, type Zoom } from "../../../lib/preview/geometry";
  import { placeCallouts } from "../../../lib/preview/callouts";
  import { rowOf, stepNumber, stepRows } from "../../../lib/editor/rows";
  import { plural } from "../../../lib/format";
  import type { StepOf } from "../../../lib/types";

  let { zoom = $bindable(null) }: { zoom?: Zoom | null } = $props();

  let w = $state(0);
  let h = $state(0);
  const aspect = $derived(w > 0 && h > 0 ? w / h : PREVIEW_ASPECT);
  // Another macro, or a recording, starts fitted.
  $effect(() => {
    void relay.view?.id;
    void relay.recording;
    zoom = null;
  });
  /** The part of the desktop in view. */
  const d = $derived(zoomedView(relay.desktop, aspect, zoom));
  const toStage = (x: number, y: number) => ({ x: ((x - d.x) / d.w) * w, y: ((y - d.y) / d.h) * h });

  const cur = $derived(Math.min(relay.cur, relay.duration));
  const path = $derived(pathD(relay.moves));
  const lengths = $derived(cumulativeLengths(relay.moves));
  const total = $derived(lengths.length ? lengths[lengths.length - 1] : 0);
  const moveIdx = $derived(lastIndexAtOrBefore(relay.moves, cur));
  const doneLen = $derived(moveIdx > 0 ? lengths[moveIdx] : 0);
  const showFull = $derived(relay.settings.path_mode === "full" && relay.mode !== "recording");
  const cursor = $derived(relay.cursorAt(cur));

  const steps = $derived(relay.steps);
  const rows = $derived(stepRows(steps, relay.settings.show_moves));
  /** The selected step's move (folded into it, or the move itself), to show what editing it changes. */
  const openMove = $derived.by(() => {
    if (relay.mode === "recording") return "";
    const row = rows[rowOf(rows, relay.selected)];
    const index = row?.move ?? (steps[relay.selected]?.kind === "move" ? relay.selected : -1);
    const s = steps[index];
    if (s?.kind !== "move") return "";
    return pathD([{ t: s.t, x: s.x, y: s.y }, ...relay.moves.filter((m) => m.t >= s.t && m.t <= s.end)]);
  });
  const pixel = $derived(steps[relay.selected]?.kind === "pixel_wait" ? (steps[relay.selected] as StepOf<"pixel_wait">) : null);

  const clicks = $derived(
    steps.flatMap((s, i) => (s.kind === "click" ? [{ s, n: stepNumber(steps, i), selected: relay.selection.includes(i) }] : [])),
  );
  const callouts = $derived(w && h ? placeCallouts(clicks.map((c) => toStage(c.s.x, c.s.y)), w, h) : []);

  const hasScreen = $derived(!!relay.screenUrl && relay.mode !== "recording");
  const shot = $derived(hasScreen && relay.settings.preview_background === "screen" ? relay.view?.recording.virtual_desktop : undefined);
  const label = $derived(`The mouse path over the screen, with ${plural(clicks.length, "click")} numbered in order`);

  let stage: HTMLDivElement | undefined = $state();
  function pointAt(e: MouseEvent) {
    const r = stage!.getBoundingClientRect();
    return { x: d.x + ((e.clientX - r.left) / (r.width || 1)) * d.w, y: d.y + ((e.clientY - r.top) / (r.height || 1)) * d.h };
  }
  function onWheel(e: WheelEvent) {
    e.preventDefault();
    zoom = zoomAround(relay.desktop, aspect, zoom, pointAt(e), Math.pow(1.25, -e.deltaY / 100));
  }
  let dragging: { x: number; y: number } | null = $state(null);
  function onDown(e: PointerEvent) {
    if (e.button !== 0 || !zoom) return;
    dragging = { x: e.clientX, y: e.clientY };
    try {
      stage!.setPointerCapture(e.pointerId);
    } catch {
      // Already up: the drag follows its events anyway.
    }
  }
  function onMove(e: PointerEvent) {
    if (!dragging) return;
    const r = stage!.getBoundingClientRect();
    const per = d.w / (r.width || 1);
    zoom = pan(relay.desktop, aspect, zoom, (dragging.x - e.clientX) * per, (dragging.y - e.clientY) * per);
    dragging = { x: e.clientX, y: e.clientY };
  }
  function key(e: KeyboardEvent) {
    const middle = { x: d.x + d.w / 2, y: d.y + d.h / 2 };
    const step = d.w / 10;
    const panBy = { ArrowLeft: [-step, 0], ArrowRight: [step, 0], ArrowUp: [0, -step], ArrowDown: [0, step] }[e.key];
    if (e.key === "+" || e.key === "=") zoom = zoomAround(relay.desktop, aspect, zoom, middle, 1.25);
    else if (e.key === "-") zoom = zoomAround(relay.desktop, aspect, zoom, middle, 0.8);
    else if (e.key === "0") zoom = null;
    else if (panBy && zoom) zoom = pan(relay.desktop, aspect, zoom, panBy[0], panBy[1]);
    else return;
    e.preventDefault();
  }
</script>

<!-- It zooms and pans by keyboard too. -->
<!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
<div
  class="stage"
  class:zoomed={zoom}
  class:dragging
  role="img"
  aria-label={label}
  tabindex="0"
  title="Scroll or + and − to zoom; drag or the arrows to move; double-click or 0 to see it all"
  bind:this={stage}
  bind:clientWidth={w}
  bind:clientHeight={h}
  onwheel={onWheel}
  onpointerdown={onDown}
  onpointermove={onMove}
  onpointerup={() => (dragging = null)}
  onlostpointercapture={() => (dragging = null)}
  ondblclick={() => (zoom = null)}
  onkeydown={key}
>
  <svg viewBox="{d.x} {d.y} {d.w} {d.h}" preserveAspectRatio="xMidYMid meet" aria-hidden="true">
    {#if shot}
      <image href={relay.screenUrl} x={shot.x} y={shot.y} width={shot.w} height={shot.h} preserveAspectRatio="none" opacity="0.55" />
    {:else}
      {#each relay.frames as f, i (i)}
        <rect class="frame" x={f.x} y={f.y} width={f.w} height={f.h} />
      {/each}
    {/if}
    {#if showFull && path}
      <path class="path full" d={path} />
    {/if}
    {#if path && doneLen > 0}
      <path class="path done" d={path} stroke-dasharray="{doneLen} {total + 1}" />
    {/if}
    {#if openMove}
      <path class="path open-move" d={openMove} />
    {/if}
  </svg>
  {#each callouts as c (c.bx + ":" + c.by)}
    <svg class="leader" aria-hidden="true"><line x1={c.x} y1={c.y} x2={c.bx} y2={c.by} /><circle cx={c.x} cy={c.y} r="1.6" /></svg>
  {/each}
  {#each callouts as c, i (i)}
    {@const click = clicks[i]}
    <span
      class="callout"
      class:left={c.left}
      class:unplayed={click.s.t > cur}
      class:selected={click.selected}
      style:left="{c.bx}px"
      style:top="{c.by}px"
      aria-hidden="true"
    >
      <i>{click.n}</i>{#if click.selected && relay.settings.show_click_labels && click.s.label}<span>{click.s.label}</span>{/if}
    </span>
  {/each}
  {#if pixel}
    {@const p = toStage(pixel.x, pixel.y)}
    <span class="pixel" style:left="{p.x}px" style:top="{p.y}px" style:background={pixel.color} aria-hidden="true"></span>
  {/if}
  {#if w}
    {@const m = toStage(cursor.x, cursor.y)}
    <svg class="cursor" style:left="{m.x}px" style:top="{m.y}px" width="12" height="16" viewBox="0 0 12 16" aria-hidden="true">
      <path d="M1 1v12l3-2.8 2.2 4.6 2-1-2.2-4.4H10z" />
    </svg>
  {/if}
</div>

<style>
  .stage {
    position: relative;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
    border-radius: var(--r-md);
    background: var(--stage);
    box-shadow: inset 0 0 0 1px var(--line);
    cursor: default;
    touch-action: none;
  }
  .stage.zoomed {
    cursor: grab;
  }
  .stage.dragging {
    cursor: grabbing;
  }
  .stage:focus-visible {
    outline: 2px solid var(--text);
    outline-offset: 2px;
  }
  svg {
    display: block;
    width: 100%;
    height: 100%;
  }
  .frame {
    fill: none;
    stroke: rgba(244, 243, 239, 0.35);
    stroke-width: 1;
    vector-effect: non-scaling-stroke;
  }
  .path {
    fill: none;
    stroke: #6f8fff;
    stroke-linejoin: round;
    stroke-linecap: round;
    vector-effect: non-scaling-stroke;
  }
  .full {
    stroke-width: 1.4;
    stroke-dasharray: 4 4;
    opacity: 0.75;
  }
  .done {
    stroke-width: 2.4;
  }
  .open-move {
    stroke: #f4f3ef;
    stroke-width: 4;
    opacity: 0.55;
  }
  .leader {
    position: absolute;
    inset: 0;
    overflow: visible;
    pointer-events: none;
  }
  .leader line {
    stroke: rgba(244, 243, 239, 0.55);
    stroke-width: 1;
  }
  .leader circle {
    fill: #f4f3ef;
  }
  .callout {
    position: absolute;
    display: flex;
    align-items: center;
    gap: 5px;
    white-space: nowrap;
    pointer-events: none;
    transform: translate(-10px, -50%);
  }
  .callout.left {
    flex-direction: row-reverse;
    transform: translate(calc(-100% + 10px), -50%);
  }
  .callout i {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex: none;
    width: 20px;
    height: 20px;
    border-radius: 50%;
    font-size: 12px;
    font-style: normal;
    font-weight: 800;
    color: #fff;
    background: var(--blue);
  }
  .callout.unplayed i {
    color: var(--blue-line);
    background: var(--stage);
    box-shadow: inset 0 0 0 1.5px var(--blue-line);
  }
  .callout.selected i {
    color: var(--ink);
    background: var(--paper);
    box-shadow:
      0 0 0 2px var(--ink),
      0 0 0 3.5px var(--paper);
  }
  .callout span {
    font-size: 12px;
    font-weight: 600;
    color: rgba(244, 243, 239, 0.92);
    text-shadow: 0 1px 2px #000;
  }
  .pixel {
    position: absolute;
    width: 10px;
    height: 10px;
    margin: -5px 0 0 -5px;
    border-radius: 2px;
    box-shadow:
      0 0 0 1.5px #fff,
      0 0 0 3px var(--green);
    pointer-events: none;
  }
  .cursor {
    position: absolute;
    width: 12px;
    height: 16px;
    pointer-events: none;
  }
  .cursor path {
    fill: #f4f3ef;
    stroke: #141415;
    stroke-width: 0.8;
  }
</style>
