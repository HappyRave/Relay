<script lang="ts">
  // The player: Relay's small window on top, shown when the main window isn't.
  // Record, play or pause, stop; a screen saying what's happening (the
  // state, the macro as a menu to switch macros, the time or the row, the
  // progress or the last step, an error until it's dismissed); and a key
  // back to the window.
  import Key from "../kit/Key.svelte";
  import Leds from "../kit/Leds.svelte";
  import Menu, { type MenuItem } from "../kit/Menu.svelte";
  import Grip from "../shared/Grip.svelte";
  import Icon from "../ui/Icon.svelte";
  import { relay } from "../../lib/state/relay.svelte";
  import { fmtClock } from "../../lib/format";
  import { rowText } from "../../lib/editor/rows";
  import { ariaKeys, keyHint } from "../../lib/hotkeys";

  const keys = $derived(relay.settings.keys);

  /** How long DONE shows after a run completes. */
  const DONE_MS = 1600;

  const live = $derived(relay.recording);
  const pausing = $derived(relay.mode === "playing");
  const rows = $derived(relay.dataFile && !relay.dataFile.error ? relay.dataFile.rows : null);
  const error = $derived([...relay.toasts].reverse().find((t) => t.kind === "error") ?? null);

  let done = $state(false);
  let shake = $state(false);
  $effect(() => {
    if (!relay.finishes) return;
    const ok = relay.lastFinish === "completed";
    if (ok) done = true;
    else if (relay.lastFinish === "pixel_timeout" || relay.lastFinish === "error") shake = true;
    const timer = setTimeout(() => (ok ? (done = false) : (shake = false)), ok ? DONE_MS : 300);
    return () => clearTimeout(timer);
  });

  const status = $derived.by(() => {
    if (relay.mode === "countdown") return { word: "GET READY", live: true };
    if (relay.mode === "recording") return { word: "REC", live: true };
    if (relay.mode === "playing") return { word: "PLAYING", live: true };
    if (relay.mode === "paused") return { word: "PAUSED", live: false };
    if (done) return { word: "DONE ✓", live: false };
    if (error) return { word: "STOPPED", live: false };
    return { word: "READY", live: false };
  });
  const row = $derived(rows ? `Row ${Math.min(relay.loopIdx + 1, rows)} / ${rows}` : null);
  const time = $derived(`${fmtClock(Math.min(relay.cur, relay.duration))} / ${fmtClock(relay.duration)}`);
  const last = $derived.by(() => {
    const s = relay.steps.at(-1);
    if (!s) return "";
    const t = rowText(s, null, { anchor: null });
    return `${t.kind} · ${t.what}`;
  });
  const clicks = $derived(relay.steps.filter((s) => s.kind === "click").map((s) => s.t / (relay.duration || 1)));
  /** What a screen reader hears when the state changes (not every tick). */
  const spoken = $derived.by(() => {
    if (relay.mode === "countdown") return `Get ready, ${relay.countdown}`;
    if (relay.mode === "recording") return "Recording";
    if (relay.mode === "playing") return `Playing ${relay.name}`;
    if (relay.mode === "paused") return "Paused";
    if (done) return `${relay.name} finished`;
    return error ? `Stopped: ${error.message}` : "";
  });

  let name: HTMLElement | undefined = $state();
  let menuOpen = $state(false);
  const items = $derived<MenuItem[]>(
    relay.library.map((m) => ({ id: m.id, label: m.name, checked: m.id === relay.view?.id, shortcut: m.hotkey ?? undefined })),
  );
</script>

<div
  class="player"
  class:big={relay.settings.player_size === "large"}
  class:shake
  role="group"
  aria-label="Relay player"
>
  <Grip />
  <Key
    variant="red"
    live={live}
    aria-label={live ? "Stop recording" : "Record"}
    aria-keyshortcuts={ariaKeys(keys.record)}
    title="{live ? 'Stop recording' : 'Record'}{keyHint(keys.record)}"
    disabled={relay.playing}
    onclick={relay.toggleRec}><span class="rec-dot"></span></Key
  >
  <Key
    variant="ink"
    aria-label={pausing ? "Pause" : "Play"}
    aria-keyshortcuts={ariaKeys(keys.play)}
    title="{pausing ? 'Pause' : 'Play'}{keyHint(keys.play)}"
    disabled={live || !relay.view}
    onclick={relay.togglePlay}><Icon name={pausing ? "pause" : "play"} /></Key
  >
  <Key aria-label="Stop" aria-keyshortcuts="Escape" title="Stop (Esc)" disabled={relay.mode === "idle"} onclick={relay.stop}
    ><Icon name="stop" size={14} /></Key
  >
  <div class="screen" class:flash={done}>
    <div class="top">
      <span class="state" class:live={status.live} class:done>{status.word}</span>
      <span class="nm-wrap" bind:this={name}>
        <button
          type="button"
          class="nm"
          aria-haspopup="menu"
          aria-expanded={menuOpen}
          aria-label="Macro: {live ? 'New recording' : relay.name}. Switch macros"
          disabled={relay.mode !== "idle" || !relay.library.length}
          onclick={() => (menuOpen = !menuOpen)}
          ><span>{live ? "New recording" : relay.name || "No macro"}</span><Icon name="down" size={12} /></button
        >
      </span>
      <span class="rt num">
        {#if relay.mode === "countdown"}<span class="digit">{relay.countdown}</span>
        {:else if relay.mode === "recording"}{relay.steps.length} {relay.steps.length === 1 ? "step" : "steps"}
        {:else if relay.playing && row}{row}
        {:else}{time}{/if}
      </span>
    </div>
    {#if error && relay.mode === "idle"}
      <div class="top">
        <span class="err"><Icon name="warning" size={12} /></span>
        <span class="err msg" title={error.message}>{error.message}</span>
        <button type="button" class="dismiss" aria-label="Dismiss" onclick={() => relay.dismissToast(error.id)}
          ><Icon name="close" size={12} /></button
        >
      </div>
    {:else if relay.mode === "recording"}
      <div class="top last">{last ? `Last: ${last}` : "Recording your clicks and keys"}</div>
    {:else}
      <Leds value={relay.duration ? Math.min(relay.cur, relay.duration) / relay.duration : 0} ticks={clicks} />
    {/if}
  </div>
  <Key aria-label="Open the window" aria-keyshortcuts={ariaKeys(keys.player)} title="Open the window{keyHint(keys.player)}" onclick={relay.openWindow}
    ><Icon name="expand" /></Key
  >
  <span class="sr-only" aria-live="polite">{spoken}</span>
</div>
{#if menuOpen && name}
  <Menu
    {items}
    opener={name.querySelector("button")!}
    label="Switch macros"
    onselect={(id) => relay.loadMacro(id)}
    onclose={() => (menuOpen = false)}
  />
{/if}

<style>
  .player {
    --pk: 48px;
    box-sizing: border-box;
    display: flex;
    align-items: center;
    gap: 6px;
    width: 600px;
    height: 64px;
    padding: 8px;
    color: var(--text);
    background: var(--glass);
    font-family: var(--font);
  }
  .player.big {
    --pk: 64px;
    width: 760px;
    height: 88px;
  }
  .player :global(.key) {
    width: var(--pk);
    height: calc(var(--pk) - 2px);
    padding: 0;
    justify-content: center;
    border-radius: 11px;
  }
  .rec-dot {
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: currentColor;
    flex: none;
  }
  .screen {
    flex: 1;
    align-self: stretch;
    min-width: 0;
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    padding: 6px 10px 7px 12px;
    border-radius: 10px;
    color: var(--paper);
    background: var(--lcd);
    box-shadow:
      inset 0 1px 4px rgba(0, 0, 0, 0.7),
      0 0 0 1px rgba(0, 0, 0, 0.4);
  }
  .big .screen {
    padding: 10px 14px;
  }
  .top {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    font-size: 13px;
    white-space: nowrap;
  }
  .big .top {
    font-size: 16px;
  }
  .state {
    flex: none;
    padding: 1px 6px;
    border-radius: 3px;
    font-size: 12px;
    font-weight: 800;
    letter-spacing: 0.08em;
    color: var(--alu-300);
    background: var(--graphite-800);
  }
  .big .state {
    font-size: 13px;
  }
  .state.live {
    color: #fff;
    background: var(--red);
  }
  .state.done {
    color: var(--ink);
    background: var(--green);
  }
  .nm-wrap {
    display: flex;
    min-width: 0;
  }
  .nm {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    min-width: 0;
    min-height: 24px;
    margin: -4px 0 -4px -4px;
    padding: 0 4px;
    border: 0;
    border-radius: 4px;
    font: inherit;
    font-weight: 700;
    color: inherit;
    background: none;
    cursor: pointer;
  }
  .nm span {
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .nm:hover:not(:disabled) {
    background: var(--graphite-800);
  }
  .nm:disabled {
    cursor: default;
  }
  .nm:focus-visible {
    outline: 2px solid var(--paper);
    outline-offset: 2px;
  }
  .rt {
    flex: none;
    margin-left: auto;
    font-size: 12px;
    font-weight: 600;
    font-stretch: 125%;
    color: var(--alu-300);
  }
  .digit {
    font-size: 16px;
    color: var(--paper);
  }
  .last {
    font-size: 12px;
    color: var(--alu-300);
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .err {
    display: inline-flex;
    font-weight: 600;
    color: #ffb4a6;
  }
  .err.msg {
    display: block;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .dismiss {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex: none;
    width: 24px;
    height: 24px;
    margin: -6px -6px -6px auto;
    padding: 0;
    border: 0;
    border-radius: 4px;
    color: var(--alu-300);
    background: none;
    cursor: pointer;
  }
  .dismiss:hover {
    background: var(--graphite-800);
  }
  .flash {
    animation: flash 600ms var(--ease);
  }
  @keyframes flash {
    0% {
      box-shadow:
        inset 0 1px 4px rgba(0, 0, 0, 0.7),
        0 0 0 1px rgba(0, 0, 0, 0.4),
        0 0 0 0 rgba(29, 180, 106, 0.8);
    }
    100% {
      box-shadow:
        inset 0 1px 4px rgba(0, 0, 0, 0.7),
        0 0 0 1px rgba(0, 0, 0, 0.4),
        0 0 0 12px rgba(29, 180, 106, 0);
    }
  }
  .shake {
    animation: shake 300ms var(--ease);
  }
  @keyframes shake {
    0%,
    100% {
      transform: none;
    }
    25% {
      transform: translateX(-2px);
    }
    75% {
      transform: translateX(2px);
    }
  }
</style>
