<script lang="ts">
  import { relay } from "../lib/state/relay.svelte";
  import { onMount } from "svelte";
  import type { ExportFormat } from "../lib/types";

  const FORMATS: { id: ExportFormat; label: string; sub: string }[] = [
    { id: "rly", label: "Relay macro", sub: ".rly — editable, keeps timing" },
    { id: "json", label: "JSON events", sub: ".json — raw event stream for devs" },
    { id: "exe", label: "Standalone program", sub: ".exe — plays on any Windows PC, no install" },
    { id: "ahk", label: "AutoHotkey v2", sub: ".ahk script — readable, runs without Relay" },
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
  <div class="dialog-title" id="export-title">Export macro</div>
  <div class="formats">
    {#each FORMATS as f (f.id)}
      {@const selected = f.id === relay.exportFmt}
      <button class="fmt" class:selected onclick={() => (relay.exportFmt = f.id)}>
        <span class="dot"></span>
        <span class="text">
          <span class="label">{f.label}</span>
          <span class="sub">{f.sub}</span>
        </span>
      </button>
    {/each}
  </div>
  <div class="file">{relay.exportName}</div>
  {#if relay.exportFmt === "exe"}
    <p class="about">Plays with this macro's saved options. It isn't signed, so Windows may warn on another PC.</p>
  {:else if relay.exportFmt === "ahk"}
    <p class="about">Needs AutoHotkey v2. Clicks at screen coordinates; speed, repeat and Humanize are at the top of the script.</p>
  {/if}
  <div class="dialog-actions">
    {#if !relay.editable}<span class="note">Exporting needs the Relay app</span>{/if}
    <button class="btn btn-primary save" disabled={!relay.editable || !relay.view} onclick={relay.doExport}>Save…</button>
    <button class="btn btn-ghost cancel" onclick={() => el?.close()}>Cancel</button>
  </div>
</dialog>

<style>
  .dialog {
    width: 440px;
    max-width: calc(100vw - 32px);
    margin: auto;
    color: var(--color-text);
    background: var(--color-bg);
    border: 2px solid var(--color-text);
  }
  .dialog::backdrop {
    background: color-mix(in srgb, var(--color-neutral-900) 50%, transparent);
  }
  .note {
    margin-right: auto;
    align-self: center;
    font-size: 12px;
    color: var(--color-neutral-700);
  }
  .formats {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .fmt {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 12px;
    border: 2px solid var(--color-divider);
    background: transparent;
    font: inherit;
    text-align: left;
    cursor: pointer;
    color: var(--color-text);
  }
  .fmt:hover:not(:disabled) {
    background: var(--color-neutral-200);
  }
  .fmt:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }
  .fmt.selected {
    border-color: var(--color-accent);
  }
  .dot {
    width: 12px;
    height: 12px;
    flex: none;
    border: 2px solid var(--color-text);
  }
  .selected .dot {
    background: var(--color-accent);
  }
  .text {
    display: flex;
    flex-direction: column;
  }
  .label {
    font-weight: 800;
    font-size: 14px;
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .sub {
    font-size: 12px;
    color: var(--color-neutral-700);
  }
  .file {
    font-size: 12px;
    color: var(--color-neutral-700);
    font-variant-numeric: tabular-nums;
  }
  .about {
    margin: 0;
    font-size: 12px;
    color: var(--color-neutral-700);
  }
  .dialog-actions {
    justify-content: flex-start;
  }
  .save {
    justify-content: flex-start;
    min-width: 140px;
  }
  .cancel {
    margin-left: auto;
  }
</style>
