<script lang="ts">
  import { relay } from "../lib/state/relay.svelte";
  import { onMount } from "svelte";
  import type { ExportFormat } from "../lib/types";

  const FORMATS: { id: ExportFormat | "ahk" | "exe"; label: string; sub: string; later?: boolean }[] = [
    { id: "rly", label: "Relay macro", sub: ".rly — editable, keeps timing" },
    { id: "json", label: "JSON events", sub: ".json — raw event stream for devs" },
    { id: "ahk", label: "AutoHotkey v2", sub: ".ahk script — runs without Relay", later: true },
    { id: "exe", label: "Standalone .exe", sub: "Portable runner", later: true },
  ];

  let el: HTMLDialogElement | undefined = $state();
  onMount(() => el?.showModal());
</script>

<!-- A native modal dialog: it traps focus, blocks the page behind it, and
     closes on Esc; a click on the backdrop closes it too. -->
<dialog
  bind:this={el}
  class="dialog"
  aria-labelledby="export-title"
  onclose={() => (relay.exportOpen = false)}
  onclick={(e) => e.target === el && el.close()}
>
  <div class="dialog-title" id="export-title">Export macro</div>
  <div class="formats">
    {#each FORMATS as f (f.id)}
      {@const selected = f.id === relay.exportFmt}
      <button class="fmt" class:selected disabled={f.later} onclick={() => !f.later && (relay.exportFmt = f.id as ExportFormat)}>
        <span class="dot"></span>
        <span class="text">
          <span class="label">{f.label}{#if f.later}<span class="tag tag-neutral">Coming later</span>{/if}</span>
          <span class="sub">{f.sub}</span>
        </span>
      </button>
    {/each}
  </div>
  <div class="file">{relay.exportName}</div>
  <div class="dialog-actions">
    {#if !relay.editable}<span class="note">Exporting needs the Relay app</span>{/if}
    <button class="btn btn-primary save" disabled={!relay.editable} onclick={relay.doExport}>Save…</button>
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
  .label .tag {
    font-size: 10px;
    padding: 1px 6px;
    font-weight: 600;
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
