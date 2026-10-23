<script lang="ts">
  // The first run, in the window: what Relay does, its keys, and a first
  // recording. Skippable. After an upgrade, only the keys screen, once, with
  // the keys kept from before.
  import TopBar from "./TopBar.svelte";
  import Key from "../kit/Key.svelte";
  import Keycaps from "../kit/Keycaps.svelte";
  import Icon from "../ui/Icon.svelte";
  import { relay } from "../../lib/state/relay.svelte";

  type Choice = "ctrl_shift" | "plain" | "own";
  const CHOICES: Record<Exclude<Choice, "own">, [string, string]> = {
    ctrl_shift: ["Ctrl + Shift + F9", "Ctrl + Shift + F10"],
    plain: ["F9", "F10"],
  };
  /** The kill switch that works over Remote Desktop. */
  const REMOTE_SAFE = "Ctrl + Alt + Backspace";

  const keysOnly = relay.settings.intro === "keys";
  let step = $state(keysOnly ? 1 : 0);
  const keys = $derived(relay.settings.keys);
  const before = relay.settings.keys;
  let choice = $state<Choice>(before.record === "F9" && before.play === "F10" ? "plain" : "ctrl_shift");
  let refused = $state<string | null>(null);
  const shown = $derived(choice === "own" ? [keys.record, keys.play] : CHOICES[choice]);

  /** Sets the keys chosen; false if Relay refused them (it says why). */
  async function useChoice(): Promise<boolean> {
    refused = null;
    if (choice === "own") return true;
    const [record, play] = CHOICES[choice];
    for (const [key, combo] of [["play", play], ["record", record]] as const) {
      if (keys[key] === combo) continue;
      refused = await relay.setRelayKey(key, combo);
      if (refused) return false;
    }
    return true;
  }

  async function next() {
    if (step === 1 && !(await useChoice())) return;
    if (step === 1 && keysOnly) return done();
    step++;
  }

  async function done() {
    await relay.finishIntro();
    if (choice === "own") relay.openSettings("keys");
  }

  async function recordNow() {
    await relay.finishIntro();
    relay.toggleRec();
  }

  async function remoteSafe() {
    refused = await relay.setRelayKey("kill", REMOTE_SAFE);
  }
</script>

<div class="first" aria-label="Welcome to Relay" role="region">
  <TopBar>
    {#snippet title()}<span class="brand" data-tauri-drag-region><span class="mark"></span>Relay</span>{/snippet}
  </TopBar>
  <div class="body">
    <div class="copy">
      {#if !keysOnly}
        <span class="stepper" role="img" aria-label="Step {step + 1} of 3">
          {#each [0, 1, 2] as i (i)}<i class:on={i <= step}></i>{/each}
        </span>
      {/if}
      {#if step === 0}
        <h1>Do it once.<br />Relay does it again.</h1>
        <p>
          Relay watches what you do with the mouse and keyboard, turns it into steps you can read and change, and plays them
          back whenever you like: on a key, on a schedule, or when something happens on screen.
        </p>
        <p>It never connects to the internet. Your macros stay on this PC.</p>
        <div class="actions">
          <Key variant="ink" onclick={next}>Next</Key>
          <span class="spacer"></span>
          <Key variant="ghost" onclick={done}>Skip</Key>
        </div>
      {:else if step === 1}
        <h1>Your keys</h1>
        <p>
          {keysOnly ? "Relay's keys can be changed now. Keep yours, or choose others." : "Record and play from any app, without opening Relay."}
        </p>
        <div class="choices" role="radiogroup" aria-label="Keys for record and play">
          {#snippet card(value: Choice, title: string, sub: string)}
            <button type="button" class="card" role="radio" aria-checked={choice === value} onclick={() => (choice = value)}>
              <span class="radio" aria-hidden="true"></span>
              <span><b>{title}</b><span class="sub">{sub}</span></span>
            </button>
          {/snippet}
          {@render card("ctrl_shift", "Ctrl + Shift + F9 and Ctrl + Shift + F10", "Other apps keep F9 and F10. Recommended.")}
          {@render card("plain", "F9 and F10", "One key each. Excel, Word and debuggers lose F9 and F10 while Relay runs.")}
          {@render card("own", "I'll choose my own", "In Settings → Keys. For keyboards without F keys.")}
        </div>
        {#if refused}<p class="refused" role="alert">{refused}</p>{/if}
        <div class="notice">
          <Icon name="stop" size={14} />
          <span>
            <b>To stop:</b> Esc, the player's ■, or <Keycaps combo={keys.kill.split(" + ")} /> for everything at once.
            {#if keys.kill === "Ctrl + Alt + End"}
              Remote Desktop takes Ctrl + Alt + End for itself.
              <Key size="sm" onclick={remoteSafe}>Use {REMOTE_SAFE}</Key>
            {:else if keys.kill === REMOTE_SAFE}
              It works over Remote Desktop too.
            {/if}
          </span>
        </div>
        <div class="actions">
          {#if !keysOnly}<Key variant="ghost" onclick={() => (step = 0)}><Icon name="back" />Back</Key>{/if}
          <span class="spacer"></span>
          <Key variant="ink" onclick={next}>{keysOnly ? "Done" : "Next"}</Key>
        </div>
      {:else}
        <h1>Your first macro</h1>
        <ol>
          <li>Open Notepad and click inside it.</li>
          <li>
            Press <b>Record</b>{#if keys.record}&nbsp;or <Keycaps combo={keys.record.split(" + ")} />{/if}. The player counts down 3, 2, 1.
          </li>
          <li>Type a line, and press Enter.</li>
          <li>Press <b>■</b> on the player{#if keys.record}, or <Keycaps combo={keys.record.split(" + ")} /> again{/if}.</li>
        </ol>
        <p>Relay opens your macro so you can name it, read its steps, and play it.</p>
        <div class="actions">
          <Key variant="ghost" onclick={() => (step = 1)}><Icon name="back" />Back</Key>
          <span class="spacer"></span>
          <Key variant="ghost" onclick={done}>Later</Key>
          <Key variant="red" onclick={recordNow}><span class="rec-dot"></span>Record now</Key>
        </div>
      {/if}
    </div>
    <div class="art" aria-hidden="true">
      {#if step === 1}
        <div class="caps">
          {#each shown as combo, i (i)}
            {#if combo}<span class="row">{#each combo.split(" + ") as k (k)}<span class="cap">{k}</span>{/each}</span>{:else}<span class="row none">None</span>{/if}
            <span class="what">{i ? "play" : "record"}</span>
          {/each}
        </div>
      {:else}
        <svg viewBox="0 0 300 200" width="88%">
          <rect x="20" y="20" width="260" height="160" fill="none" stroke="rgba(244,243,239,.35)" />
          <path d="M14 20h4M20 14v4M286 20h-4M280 14v4M14 180h4M20 186v-4M286 180h-4M280 186v-4" stroke="rgba(244,243,239,.6)" />
          <path d="M60 150 C 80 80, 120 60, 150 70 S 210 120, 240 60" fill="none" stroke="#6f8fff" stroke-width="2.2" />
          {#each [[60, 150, 1], [150, 70, 2], [240, 60, 3]] as [x, y, n] (n)}
            <line x1={x} y1={y} x2={x + 14} y2={y - 14} stroke="rgba(244,243,239,.6)" />
            <circle cx={x + 18} cy={y - 18} r="9" fill="#2450e6" />
            <text x={x + 18} y={y - 14} font-size="11" font-weight="800" text-anchor="middle" fill="#fff">{n}</text>
          {/each}
          <rect x="40" y="170" width="60" height="8" rx="2" fill="#ffc21a" />
          <rect x="110" y="170" width="40" height="8" rx="2" fill="#1db46a" />
          <rect x="160" y="170" width="80" height="8" rx="2" fill="#2450e6" />
        </svg>
      {/if}
    </div>
  </div>
</div>

<style>
  .first {
    display: grid;
    grid-template-rows: var(--key-h) minmax(0, 1fr);
    gap: var(--gap);
    height: 100%;
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 16px;
    font-weight: 800;
  }
  .mark {
    width: 14px;
    height: 14px;
    border-radius: var(--r-xs);
    background: var(--red);
  }
  .body {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--gap);
    min-height: 0;
  }
  .copy {
    display: flex;
    flex-direction: column;
    gap: 14px;
    min-height: 0;
    overflow: auto;
    padding: 28px 28px 22px;
    border-radius: var(--r-lg);
    background: var(--plate);
    box-shadow: 0 0 0 1px var(--line);
  }
  h1 {
    margin: 0;
    font-size: 26px;
    font-weight: 800;
    letter-spacing: -0.02em;
    line-height: 1.15;
  }
  p {
    margin: 0;
    font-size: 15px;
    color: var(--text-2);
  }
  ol {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin: 0;
    padding-left: 20px;
    font-size: 15px;
  }
  .stepper {
    display: flex;
    gap: 6px;
  }
  .stepper i {
    width: 22px;
    height: 4px;
    border-radius: 2px;
    background: var(--line-strong);
  }
  .stepper i.on {
    background: var(--text);
  }
  .choices {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .card {
    display: flex;
    align-items: flex-start;
    gap: 12px;
    width: 100%;
    padding: 10px 12px;
    border: 0;
    border-radius: var(--r-md);
    font: inherit;
    text-align: left;
    color: var(--text);
    background: var(--plate);
    box-shadow: 0 0 0 1px var(--line);
    cursor: pointer;
  }
  .card[aria-checked="true"] {
    box-shadow: 0 0 0 2px var(--text);
  }
  .card b {
    display: block;
  }
  .sub {
    font-size: 13px;
    color: var(--text-2);
  }
  .radio {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex: none;
    width: 18px;
    height: 18px;
    margin-top: 1px;
    border-radius: 50%;
    box-shadow: inset 0 0 0 1.6px var(--line-strong);
  }
  .card[aria-checked="true"] .radio {
    box-shadow: inset 0 0 0 1.6px var(--text);
  }
  .card[aria-checked="true"] .radio::after {
    content: "";
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--text);
  }
  .refused {
    color: var(--red);
    font-size: 13px;
  }
  .notice {
    display: flex;
    gap: 8px;
    padding: 10px 12px;
    border-radius: var(--r-md);
    font-size: 13px;
    background: var(--plate-2);
  }
  .notice :global(svg) {
    flex: none;
    margin-top: 2px;
  }
  .actions {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: auto;
  }
  .spacer {
    flex: 1;
  }
  .rec-dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: currentColor;
  }
  .art {
    display: flex;
    align-items: center;
    justify-content: center;
    overflow: hidden;
    border-radius: var(--r-lg);
    background: var(--stage);
  }
  .art svg {
    max-height: 88%;
  }
  .caps {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 10px;
    color: var(--paper);
  }
  .row {
    display: flex;
    gap: 8px;
  }
  .cap {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    height: 44px;
    min-width: 56px;
    padding: 0 10px;
    border-radius: var(--r-sm);
    font-size: 16px;
    font-weight: 700;
    color: var(--paper);
    background: var(--graphite-800);
    box-shadow:
      0 0 0 1px #4a4a4e,
      0 3px 0 #000;
  }
  .none {
    font-size: 16px;
    color: var(--alu-400);
  }
  .what {
    margin-bottom: 8px;
    font-size: 13px;
    color: var(--alu-400);
  }
</style>
