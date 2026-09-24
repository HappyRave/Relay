<script lang="ts">
  // The prototype's demo desktop, used when the UI runs in a plain browser
  // (`npm run dev`). In Tauri the window contains only the widget.
  import Widget from "../Widget.svelte";
  import { devDesktop } from "../../lib/dev/devDesktop.svelte";

  let vw = $state(window.innerWidth);
  let vh = $state(window.innerHeight);
  let size = $state({ w: 944, h: 616 });
  let now = $state(new Date());

  $effect(() => {
    const ro = new ResizeObserver(() => {
      vw = window.innerWidth;
      vh = window.innerHeight;
    });
    ro.observe(document.documentElement);
    const clock = setInterval(() => (now = new Date()), 30000);
    return () => {
      ro.disconnect();
      clearInterval(clock);
    };
  });

  const bottom = $derived(vh < 700 ? 52 : 72);
  const scale = $derived(Math.max(0.3, Math.min(1, (vw - 32) / size.w, (vh - bottom - 16) / size.h)));
</script>

<div class="desktop">
  <div class="hint">
    <div class="kicker">Try it</div>
    <div>
      Press <b>F9</b> or the red button, then move, click and type anywhere on this desktop. <b>F10</b> plays it back,
      <b>Esc</b> stops.
    </div>
  </div>
  <div class="wordmark">Relay.</div>
  <div class="taskbar">
    <div class="start"><span></span><span></span><span></span><span></span></div>
    <div class="app"><span class="mark"></span>Relay</div>
    <div class="clock">{now.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })}</div>
  </div>
  <div
    class="host"
    style:bottom="{bottom}px"
    style:transform="translate(calc(-50% + {devDesktop.dx}px), {devDesktop.dy}px) scale({scale})"
  >
    <Widget onresize={(w, h) => (size = { w, h })} />
  </div>
</div>

<style>
  .desktop {
    position: fixed;
    inset: 0;
    background-color: var(--color-neutral-300);
    background-image: linear-gradient(var(--color-neutral-400) 1px, transparent 1px),
      linear-gradient(90deg, var(--color-neutral-400) 1px, transparent 1px);
    background-size: 96px 96px;
  }
  .hint {
    position: absolute;
    top: 32px;
    left: 32px;
    display: flex;
    flex-direction: column;
    gap: 6px;
    max-width: 360px;
    font-size: 14px;
    line-height: 1.45;
  }
  .kicker {
    font-size: 11px;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: var(--color-accent-700);
    font-weight: 600;
  }
  .wordmark {
    position: absolute;
    left: 28px;
    bottom: 48px;
    font-family: var(--font-heading);
    font-weight: 800;
    font-size: clamp(80px, 16vw, 220px);
    line-height: 0.8;
    letter-spacing: -0.05em;
    color: var(--color-neutral-400);
  }
  .taskbar {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    height: 40px;
    background: var(--color-bg);
    border-top: 2px solid var(--color-divider);
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 0 16px;
  }
  .start {
    display: grid;
    grid-template-columns: 8px 8px;
    gap: 2px;
  }
  .start span {
    width: 8px;
    height: 8px;
    background: var(--color-text);
  }
  .app {
    height: 40px;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 10px;
    border-bottom: 3px solid var(--color-accent);
    font-size: 13px;
    font-weight: 600;
  }
  .mark {
    width: 12px;
    height: 12px;
    background: var(--color-accent);
  }
  .clock {
    margin-left: auto;
    font-size: 12px;
    font-variant-numeric: tabular-nums;
  }
  .host {
    position: absolute;
    left: 50%;
    transform-origin: 50% 100%;
    box-shadow: var(--shadow-lg);
    line-height: 0;
  }
  .host :global(.widget) {
    line-height: 1.55;
  }
</style>
