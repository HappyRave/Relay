<script lang="ts">
  import { relay } from "../../lib/state/relay.svelte";
  import { BADGE, stepTitle, tail } from "../../lib/state/display";
  import {
    PREVIEW_ASPECT,
    cumulativeLengths,
    lastIndexAtOrBefore,
    pan,
    pathD,
    zoomAround,
    zoomedView,
    type Zoom,
  } from "../../lib/preview/geometry";
  import { startedCount } from "../../lib/timeline/lanes";
  import { pad4 } from "../../lib/format";
  import type { StepOf } from "../../lib/types";
  import type { PreviewBackground } from "../../lib/ipc/bindings/PreviewBackground";
  import Segmented from "../ui/Segmented.svelte";

  // The drawing's size, as the user's layout makes it (not measured yet: the default's shape).
  let stageW = $state(0);
  let stageH = $state(0);
  const aspect = $derived(stageW > 0 && stageH > 0 ? stageW / stageH : PREVIEW_ASPECT);
  // The whole desktop, fitted to the drawing, so resizing never hides any of
  // it; the wheel zooms in around the pointer, a drag pans, a double-click
  // (or Fit) fits it again. Another macro, or a recording, starts fitted.
  let zoom = $state<Zoom | null>(null);
  $effect(() => {
    void relay.view?.id;
    void relay.recording;
    zoom = null;
  });
  /** The visible part of the desktop. */
  const d = $derived(zoomedView(relay.desktop, aspect, zoom));
  /**
   * The design was drawn on a 1600-wide viewBox in a 600 px preview: sizes
   * scaled by `k` stay the same on screen at any zoom and pane size.
   */
  const k = $derived(((d.w / (stageW || 600)) * 600) / 1600);

  let stage: HTMLDivElement | undefined = $state();
  /** The desktop point under the pointer. */
  function pointAt(e: MouseEvent) {
    const r = stage!.getBoundingClientRect();
    return { x: d.x + ((e.clientX - r.left) / (r.width || 1)) * d.w, y: d.y + ((e.clientY - r.top) / (r.height || 1)) * d.h };
  }
  function onWheel(e: WheelEvent) {
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
    const per = d.w / (r.width || 1); // desktop px per screen px
    zoom = pan(relay.desktop, aspect, zoom, (dragging.x - e.clientX) * per, (dragging.y - e.clientY) * per);
    dragging = { x: e.clientX, y: e.clientY };
  }
  const fit = () => (zoom = null);
  const cur = $derived(Math.min(relay.cur, relay.duration));

  const path = $derived(pathD(relay.moves));
  const lengths = $derived(cumulativeLengths(relay.moves));
  const total = $derived(lengths.length ? lengths[lengths.length - 1] : 0);
  const moveIdx = $derived(lastIndexAtOrBefore(relay.moves, cur));
  const doneLen = $derived(moveIdx > 0 ? lengths[moveIdx] : 0);
  const showFull = $derived(relay.settings.path_mode === "full" && relay.mode !== "recording");

  const cm = $derived(relay.cursorAt(cur));
  const jitter = $derived.by(() => {
    const pb = relay.playback;
    if (relay.mode !== "playing" || !pb.humanize) return { x: 0, y: 0 };
    return { x: ((Math.sin(cur / 53) * pb.jitter_ms) / 20) * k, y: ((Math.cos(cur / 41) * pb.jitter_ms) / 20) * k };
  });

  const clicks = $derived(relay.steps.filter((s): s is StepOf<"click"> => s.kind === "click"));
  /** Click markers; nothing here depends on the playhead, so it's built once per macro. */
  const marks = $derived(
    clicks.map((c, i) => {
      const label = relay.settings.show_click_labels ? c.label : "";
      const right = c.x > d.x + d.w - 260 * k;
      return { id: c.items[0], n: i + 1, x: c.x, y: c.y, label, lx: right ? c.x - 30 * k - label.length * 12 * k : c.x + 28 * k };
    }),
  );
  /** Markers reached by the playhead (changes only when it passes a click). */
  const pastMarks = $derived(startedCount(clicks, cur));
  /** The ring that grows for half a second around the last click reached. */
  const ring = $derived.by(() => {
    const c = clicks[pastMarks - 1];
    const age = c ? cur - c.t : Infinity;
    if (!c || age < 0 || age >= 500) return null;
    return { x: c.x, y: c.y, size: (36 + (age / 500) * 70) * k, opacity: 1 - age / 500 };
  });

  const lastStep = $derived(relay.steps[relay.curStepIdx]);
  /** Typed characters shown in the bar, the latest ones. */
  const TYPED_SHOWN = 16;
  const keyOverlay = $derived.by(() => {
    const s = lastStep;
    if (!s || (s.kind !== "keys" && s.kind !== "type") || cur - s.end >= 900) return null;
    if (s.kind === "type") {
      const typed = s.chars.filter((c) => c.t <= cur).map((c) => c.ch).join("");
      // The end of long text: the bar has room for about this much.
      return { kind: "Typing", parts: [tail(typed, TYPED_SHOWN) + "_"] };
    }
    return { kind: "Keys", parts: s.combo };
  });
  /** What the engine is playing: saved options changed mid-playback don't apply until the next run. */
  const loopLabel = $derived.by(() => {
    const info = relay.playInfo;
    const loops = info ? info.loops : relay.loops === Infinity ? null : relay.loops;
    return `Loop ${relay.loopIdx + 1} / ${loops ?? "∞"} · ${info?.speed ?? relay.playback.speed}×`;
  });
  const activeCond = $derived(lastStep && lastStep.kind === "pixel_wait" && cur < lastStep.end ? lastStep : null);
  /** The bar's middle: the pixel check being waited for, else the step under the playhead. */
  const info = $derived(
    activeCond
      ? `Waiting for pixel ${activeCond.x}, ${activeCond.y}`
      : lastStep
        ? `Step ${relay.curStepIdx + 1} · ${stepTitle(lastStep)}`
        : "",
  );
  /** The screenshot is drawn over the desktop it shows, when the macro has one and it's chosen. Not while recording: that's live. */
  const hasScreen = $derived(!!relay.screenUrl && relay.mode !== "recording");
  const shot = $derived(
    hasScreen && relay.settings.preview_background === "screen" ? relay.view?.recording.virtual_desktop : undefined,
  );
  const blink = $derived(relay.mode === "recording" && Math.floor(cur / 500) % 2 ? 0.35 : 1);
</script>

<div class="preview">
  <!-- Everything written about the preview sits in this bar, so nothing covers the drawing. -->
  <div class="bar">
    <span class="badge" class:rec={relay.recording} style:opacity={blink}>{BADGE[relay.mode]}</span>
    {#if relay.playing}
      <span class="loop">{loopLabel}</span>
    {/if}
    {#if keyOverlay}
      <span class="keys">
        <span class="kind">{keyOverlay.kind}</span>
        {#each keyOverlay.parts as part, i (i)}
          <span class="key">{part}</span>
          {#if i < keyOverlay.parts.length - 1}<span class="plus">+</span>{/if}
        {/each}
      </span>
    {/if}
    <span class="info" class:cond={activeCond} title={info}>{info}</span>
    {#if zoom}
      <button class="fit" title="Show the whole screen" onclick={fit}>{Math.round(zoom.scale * 100)}% · Fit</button>
    {/if}
    {#if relay.mode !== "recording"}
      <span class="bg" title={hasScreen ? "" : "No screenshot: this macro was recorded without one"}>
        <Segmented
          label="Background"
          options={[["screen", "Screen"], ["sketch", "Sketch"]] as [PreviewBackground, string][]}
          value={hasScreen ? relay.settings.preview_background : "sketch"}
          disabled={!hasScreen}
          onchange={(v) => relay.updateSettings({ preview_background: v })}
        />
      </span>
    {/if}
    <span class="coords"><span class="axis">X</span> {pad4(cm.x)} <span class="axis">Y</span> {pad4(cm.y)}</span>
  </div>
  <!-- svelte-ignore a11y_no_static_element_interactions: the wheel, drag and double-click only move the view; Fit in the bar does it by keyboard -->
  <div
    class="stage"
    class:zoomed={zoom}
    class:dragging
    bind:this={stage}
    bind:clientWidth={stageW}
    bind:clientHeight={stageH}
    title="Scroll to zoom; drag to move; double-click to see it all"
    onwheel={onWheel}
    onpointerdown={onDown}
    onpointermove={onMove}
    onpointerup={() => (dragging = null)}
    onlostpointercapture={() => (dragging = null)}
    ondblclick={fit}
  >
    <svg viewBox="{d.x} {d.y} {d.w} {d.h}" width="100%" height="100%" preserveAspectRatio="xMidYMid meet">
      {#if shot}
        <!-- Dimmed, so the path and the clicks stay readable over it. -->
        <image
          class="shot"
          href={relay.screenUrl}
          x={shot.x}
          y={shot.y}
          width={shot.w}
          height={shot.h}
          preserveAspectRatio="none"
          opacity="0.55"
        />
      {/if}
      <g fill="none" stroke="var(--color-neutral-800)" stroke-width={3 * k}>
        {#each shot ? [] : relay.frames as f, i (i)}
          <rect x={f.x} y={f.y} width={f.w} height={f.h} />
        {/each}
      </g>
      {#if showFull && path}
        <path d={path} fill="none" stroke="var(--color-neutral-600)" stroke-width={3 * k} stroke-dasharray="{8 * k} {10 * k}" />
      {/if}
      {#if path && doneLen > 0}
        <path
          d={path}
          fill="none"
          stroke="var(--color-accent)"
          stroke-width={5 * k}
          stroke-linejoin="round"
          stroke-linecap="square"
          stroke-dasharray="{doneLen} {total + 1}"
        />
      {/if}
      {#if ring}
        <rect
          x={ring.x - ring.size / 2}
          y={ring.y - ring.size / 2}
          width={ring.size}
          height={ring.size}
          fill="none"
          stroke="var(--color-accent)"
          stroke-width={4 * k}
          opacity={ring.opacity}
        />
      {/if}
      {#each marks as m, i (m.id)}
        {@const past = i < pastMarks}
        <g>
          <rect
            x={m.x - 18 * k}
            y={m.y - 18 * k}
            width={36 * k}
            height={36 * k}
            fill={past ? "var(--color-accent)" : "var(--color-text)"}
            stroke={past ? "var(--color-accent)" : "var(--color-neutral-500)"}
            stroke-width={3 * k}
          />
          <text
            x={m.x}
            y={m.y + 7 * k}
            text-anchor="middle"
            font-weight="800"
            font-size={20 * k}
            fill={past ? "var(--color-bg)" : "var(--color-neutral-400)"}>{m.n}</text
          >
          {#if m.label}
            <text
              x={m.lx}
              y={m.y + 7 * k}
              font-weight="600"
              font-size={22 * k}
              fill={past ? "var(--color-neutral-200)" : "var(--color-neutral-600)"}>{m.label}</text
            >
          {/if}
        </g>
      {/each}
      {#if activeCond}
        <rect
          x={activeCond.x - 60 * k}
          y={activeCond.y - 60 * k}
          width={120 * k}
          height={120 * k}
          fill="none"
          stroke="var(--color-accent)"
          stroke-width={4 * k}
          stroke-dasharray="{14 * k} {8 * k}"
        />
      {/if}
      <g transform="translate({cm.x + jitter.x} {cm.y + jitter.y}) scale({k})">
        <path
          d="M0 0 L0 44 L11 33 L19 52 L27 48 L19 30 L34 30 Z"
          fill="var(--color-bg)"
          stroke="var(--color-text)"
          stroke-width="3"
          stroke-linejoin="round"
        />
      </g>
    </svg>
    {#if relay.mode === "countdown"}
      <div class="countdown"><div>{Math.ceil(relay.countLeft / 1000)}</div></div>
    {/if}
  </div>
</div>

<style>
  .preview {
    min-width: 0;
    min-height: 0;
    height: 100%;
    display: flex;
    flex-direction: column;
  }
  .bar {
    height: 36px;
    flex: none;
    overflow: hidden;
    display: flex;
    align-items: stretch;
    border-bottom: 2px solid var(--color-divider);
    font-size: 12px;
    white-space: nowrap;
  }
  .bar > span {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 0 12px;
  }
  .badge,
  .loop,
  .kind {
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }
  .badge {
    font-weight: 800;
    border-right: 2px solid var(--color-divider);
  }
  .badge.rec {
    color: var(--color-accent);
  }
  .loop {
    border-right: 2px solid var(--color-divider);
    color: var(--color-neutral-700);
  }
  .keys {
    border-right: 2px solid var(--color-divider);
  }
  .kind {
    color: var(--color-neutral-700);
  }
  .key {
    padding: 2px 8px;
    background: var(--color-text);
    color: var(--color-bg);
    font-weight: 800;
    border-bottom: 3px solid var(--color-accent);
    font-variant-numeric: tabular-nums;
    white-space: pre;
  }
  .plus {
    color: var(--color-neutral-600);
    font-weight: 800;
  }
  .bar > .info {
    flex: 1;
    min-width: 0;
    display: block;
    align-self: center;
    overflow: hidden;
    text-overflow: ellipsis;
    color: var(--color-neutral-700);
  }
  .info.cond {
    color: var(--color-accent);
    font-weight: 800;
  }
  .bar > .bg {
    padding: 0 8px;
  }
  .fit {
    flex: none;
    border: 0;
    border-left: 2px solid var(--color-divider);
    background: transparent;
    font: inherit;
    font-size: 12px;
    font-weight: 600;
    padding: 0 10px;
    cursor: pointer;
    color: var(--color-text);
    white-space: nowrap;
  }
  .fit:hover {
    background: var(--color-neutral-200);
  }
  .stage.zoomed {
    cursor: grab;
  }
  .stage.dragging {
    cursor: grabbing;
  }
  .coords {
    border-left: 2px solid var(--color-divider);
    font-weight: 800;
    font-variant-numeric: tabular-nums;
  }
  .axis {
    color: var(--color-neutral-600);
    font-weight: 600;
  }
  .coords .axis:last-of-type {
    margin-left: 10px;
  }
  .stage {
    position: relative;
    flex: 1;
    min-height: 0;
    background: var(--color-text);
    overflow: hidden;
  }
  svg {
    position: absolute;
    inset: 0;
    display: block;
    font-family: var(--font-heading);
  }
  .countdown {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    background: color-mix(in srgb, var(--color-text) 70%, transparent);
  }
  .countdown div {
    font-family: var(--font-heading);
    font-weight: 800;
    font-size: 140px;
    line-height: 1;
    color: var(--color-accent);
  }
</style>
