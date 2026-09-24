<script lang="ts">
  import { relay } from "../../lib/state/relay.svelte";
  import { BADGE } from "../../lib/state/display";
  import { cumulativeLengths, lastIndexAtOrBefore, pathD } from "../../lib/preview/geometry";
  import { currentStepIndex } from "../../lib/timeline/lanes";
  import { pad4 } from "../../lib/format";
  import type { StepOf } from "../../lib/types";

  const d = $derived(relay.desktop);
  /** The design was drawn on a 1600-wide viewBox; scale its sizes to the real desktop. */
  const k = $derived(d.w / 1600);
  const cur = $derived(Math.min(relay.cur, relay.duration));

  const path = $derived(pathD(relay.moves));
  const lengths = $derived(cumulativeLengths(relay.moves));
  const total = $derived(lengths.length ? lengths[lengths.length - 1] : 0);
  const moveIdx = $derived(lastIndexAtOrBefore(relay.moves, cur));
  const doneLen = $derived(moveIdx > 0 ? lengths[moveIdx] : 0);
  const showFull = $derived(relay.settings.pathMode === "full" && relay.mode !== "rec");

  const cm = $derived(relay.cursorAt(cur));
  const jitter = $derived.by(() => {
    const pb = relay.playback;
    if (relay.mode !== "play" || !pb.humanize) return { x: 0, y: 0 };
    return { x: ((Math.sin(cur / 53) * pb.jitter_ms) / 20) * k, y: ((Math.cos(cur / 41) * pb.jitter_ms) / 20) * k };
  });

  const marks = $derived.by(() => {
    let n = 0;
    return relay.steps
      .filter((s): s is StepOf<"click"> => s.kind === "click")
      .map((c) => {
        n++;
        const past = c.t <= cur;
        const age = cur - c.t;
        const ring = age >= 0 && age < 500;
        const rs = (36 + (ring ? (age / 500) * 70 : 0)) * k;
        const label = relay.settings.showClickLabels ? c.label : "";
        const right = c.x > d.x + d.w - 260 * k;
        return {
          id: c.items[0],
          n,
          x: c.x,
          y: c.y,
          past,
          rs,
          rop: ring ? 1 - age / 500 : 0,
          label,
          lx: right ? c.x - 30 * k - label.length * 12 * k : c.x + 28 * k,
        };
      });
  });

  const lastStep = $derived(relay.steps[currentStepIndex(relay.steps, cur)]);
  const keyOverlay = $derived.by(() => {
    const s = lastStep;
    if (!s || (s.kind !== "keys" && s.kind !== "type") || cur - s.end >= 900) return null;
    if (s.kind === "type") {
      const typed = s.chars.filter((c) => c.t <= cur).map((c) => c.ch).join("");
      return { kind: "Typing", parts: [typed + "_"] };
    }
    return { kind: "Keys", parts: s.combo };
  });
  const activeCond = $derived(lastStep && lastStep.kind === "pixel_wait" && cur < lastStep.end ? lastStep : null);
  const blink = $derived(relay.mode === "rec" && Math.floor(cur / 500) % 2 ? 0.35 : 1);
</script>

<div class="preview">
  <svg viewBox="{d.x} {d.y} {d.w} {d.h}" width="600" height="338" preserveAspectRatio="xMidYMid meet">
    <g fill="none" stroke="var(--color-neutral-800)" stroke-width={3 * k}>
      {#each relay.frames as f, i (i)}
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
    {#each marks as m (m.id)}
      <g>
        <rect
          x={m.x - m.rs / 2}
          y={m.y - m.rs / 2}
          width={m.rs}
          height={m.rs}
          fill="none"
          stroke="var(--color-accent)"
          stroke-width={4 * k}
          opacity={m.rop}
        />
        <rect
          x={m.x - 18 * k}
          y={m.y - 18 * k}
          width={36 * k}
          height={36 * k}
          fill={m.past ? "var(--color-accent)" : "var(--color-text)"}
          stroke={m.past ? "var(--color-accent)" : "var(--color-neutral-500)"}
          stroke-width={3 * k}
        />
        <text
          x={m.x}
          y={m.y + 7 * k}
          text-anchor="middle"
          font-weight="800"
          font-size={20 * k}
          fill={m.past ? "var(--color-bg)" : "var(--color-neutral-400)"}>{m.n}</text
        >
        {#if m.label}
          <text
            x={m.lx}
            y={m.y + 7 * k}
            font-weight="600"
            font-size={22 * k}
            fill={m.past ? "var(--color-neutral-200)" : "var(--color-neutral-600)"}>{m.label}</text
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

  <div class="badges">
    <span class="badge" class:rec={relay.recording} style:opacity={blink}>{BADGE[relay.mode]}</span>
    {#if relay.mode === "play" || relay.mode === "pause"}
      <span class="loop">
        Loop {relay.loopIdx + 1} / {relay.loops === Infinity ? "∞" : relay.loops} · {relay.playback.speed}×
      </span>
    {/if}
  </div>
  {#if activeCond}
    <div class="cond">Waiting for pixel {activeCond.x}, {activeCond.y}</div>
  {/if}
  <div class="coords">X {pad4(cm.x)}&nbsp;&nbsp; Y {pad4(cm.y)}</div>
  {#if relay.mode === "count"}
    <div class="countdown"><div>{Math.ceil(relay.countLeft / 1000)}</div></div>
  {/if}
  {#if keyOverlay}
    <div class="keys">
      <span class="kind">{keyOverlay.kind}</span>
      {#each keyOverlay.parts as part, i (i)}
        <span class="key">{part}</span>
        {#if i < keyOverlay.parts.length - 1}<span class="plus">+</span>{/if}
      {/each}
    </div>
  {/if}
</div>

<style>
  .preview {
    position: relative;
    width: 600px;
    height: 338px;
    background: var(--color-text);
    overflow: hidden;
  }
  svg {
    display: block;
    font-family: var(--font-heading);
  }
  .badges {
    position: absolute;
    top: 10px;
    left: 10px;
    display: flex;
    gap: 6px;
    align-items: center;
  }
  .badge,
  .loop {
    font-size: 10px;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    font-weight: 600;
    padding: 3px 7px;
  }
  .badge {
    background: var(--color-neutral-800);
    color: var(--color-bg);
  }
  .badge.rec {
    background: var(--color-accent);
  }
  .loop {
    border: 1px solid var(--color-neutral-600);
    color: var(--color-neutral-300);
  }
  .cond {
    position: absolute;
    left: 10px;
    bottom: 10px;
    padding: 5px 10px;
    background: var(--color-accent);
    color: var(--color-bg);
    font-size: 12px;
    font-weight: 800;
  }
  .coords {
    position: absolute;
    top: 10px;
    right: 10px;
    font-size: 11px;
    font-variant-numeric: tabular-nums;
    color: var(--color-neutral-400);
    letter-spacing: 0.04em;
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
  .keys {
    position: absolute;
    left: 10px;
    bottom: 10px;
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .kind {
    font-size: 10px;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: var(--color-neutral-400);
    font-weight: 600;
  }
  .key {
    min-width: 28px;
    padding: 5px 10px;
    background: var(--color-bg);
    color: var(--color-text);
    font-weight: 800;
    font-size: 14px;
    border-bottom: 3px solid var(--color-accent);
    font-variant-numeric: tabular-nums;
    white-space: pre;
  }
  .plus {
    color: var(--color-neutral-500);
    font-weight: 800;
  }
</style>
