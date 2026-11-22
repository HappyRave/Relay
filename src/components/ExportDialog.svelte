<script lang="ts">
  import Key from "./kit/Key.svelte";
  import { relay } from "../lib/state/relay.svelte";
  import { onMount } from "svelte";
  import type { ExportFormat } from "../lib/types";

  const FORMATS: { id: ExportFormat; label: string; sub: string }[] = [
    { id: "rly", label: "Relay macro", sub: ".rly · editable, keeps timing" },
    { id: "exe", label: "Standalone program", sub: ".exe · plays on any Windows PC, in a small player" },
    { id: "ahk", label: "AutoHotkey v2", sub: ".ahk · a readable script, runs without Relay" },
    { id: "json", label: "JSON events", sub: ".json · the raw events, for your tools" },
  ];

  let el: HTMLDialogElement | undefined = $state();
  onMount(() => el?.showModal());

  /**
   * The dialog element is also the target of clicks in its own padding and
   * between its parts; only a click outside its box is on the backdrop.
   */
  function closeOnBackdrop(e: MouseEvent) {
    if (!el || e.target !== el) return;
    const r = el.getBoundingClientRect();
    const inside = e.clientX >= r.left && e.clientX <= r.right && e.clientY >= r.top && e.clientY <= r.bottom;
    if (!inside) el.close();
  }

  /** Arrow keys move the choice, as in any radio group. */
  function arrows(e: KeyboardEvent) {
    const step = { ArrowDown: 1, ArrowRight: 1, ArrowUp: -1, ArrowLeft: -1 }[e.key];
    if (!step) return;
    e.preventDefault();
    const i = FORMATS.findIndex((f) => f.id === relay.exportFmt);
    const next = FORMATS[(i + step + FORMATS.length) % FORMATS.length];
    relay.exportFmt = next.id;
    el?.querySelector<HTMLElement>(`[data-format="${next.id}"]`)?.focus();
  }
</script>

<!-- A native modal dialog: it traps focus, blocks the page behind it, and
     closes on Esc; a click on the backdrop closes it too. -->
<dialog
  bind:this={el}
  class="dialog"
  aria-labelledby="export-title"
  onclose={() => (relay.exportOpen = false)}
  onclick={closeOnBackdrop}
>
  <div class="title" id="export-title">Export “{relay.name}”</div>
  <!-- svelte-ignore a11y_interactive_supports_focus -->
  <div class="formats" role="radiogroup" aria-label="Format" onkeydown={arrows}>
    {#each FORMATS as f (f.id)}
      {@const checked = f.id === relay.exportFmt}
      <button
        type="button"
        class="fmt"
        role="radio"
        aria-checked={checked}
        tabindex={checked ? 0 : -1}
        data-format={f.id}
        onclick={() => (relay.exportFmt = f.id)}
      >
        <span class="mark" aria-hidden="true"></span>
        <span class="text">
          <b class="label">{f.label}</b>
          <span class="sub">{f.sub}</span>
        </span>
      </button>
    {/each}
  </div>
  {#if relay.exportFmt === "exe"}
    <p class="about">Plays with this macro's saved options. It isn't signed, so Windows may warn on another PC.</p>
  {:else if relay.exportFmt === "ahk"}
    <p class="about">Needs AutoHotkey v2. Clicks at screen coordinates; speed, repeat and Humanize are at the top of the script.</p>
  {/if}
  <div class="file num">{relay.exportName}</div>
  <div class="actions">
    {#if !relay.editable}<span class="note">Exporting needs the Relay app</span>{/if}
    <Key onclick={() => el?.close()}>Cancel</Key>
    <Key variant="ink" disabled={!relay.editable || !relay.view} onclick={relay.doExport}>Save…</Key>
  </div>
</dialog>

<style>
  .dialog {
    display: flex;
    flex-direction: column;
    gap: 12px;
    width: min(460px, calc(100vw - 32px));
    margin: auto;
    padding: 18px;
    border: 0;
    border-radius: var(--r-xl);
    color: var(--text);
    background: var(--float);
    -webkit-backdrop-filter: blur(28px) saturate(1.5);
    backdrop-filter: blur(28px) saturate(1.5);
    box-shadow: var(--shadow-pop);
  }
  .dialog::backdrop {
    background: rgba(20, 20, 21, 0.35);
  }
  .title {
    font-size: 18px;
    font-weight: 800;
  }
  .formats {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .fmt {
    display: flex;
    align-items: flex-start;
    gap: 12px;
    width: 100%;
    padding: 10px 12px;
    border: 0;
    border-radius: var(--r-md);
    background: var(--plate);
    box-shadow: 0 0 0 1px var(--line);
    text-align: left;
    cursor: pointer;
  }
  .fmt:hover {
    background: var(--plate-2);
  }
  .fmt[aria-checked="true"] {
    box-shadow: 0 0 0 2px var(--text);
  }
  .mark {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 18px;
    height: 18px;
    flex: none;
    margin-top: 1px;
    border-radius: 50%;
    box-shadow: inset 0 0 0 1.6px var(--line-strong);
  }
  [aria-checked="true"] .mark {
    box-shadow: inset 0 0 0 1.6px var(--text);
  }
  [aria-checked="true"] .mark::after {
    content: "";
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--text);
  }
  .text {
    display: flex;
    flex-direction: column;
  }
  .sub,
  .about,
  .file,
  .note {
    font-size: 13px;
    color: var(--text-2);
  }
  .about {
    margin: 0;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    align-items: center;
    gap: 8px;
  }
  .note {
    margin-right: auto;
  }
  @media (forced-colors: active) {
    .dialog {
      border: 1px solid CanvasText;
    }
    .fmt[aria-checked="true"] {
      outline: 2px solid Highlight;
    }
  }
</style>
