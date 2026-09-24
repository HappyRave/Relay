<script lang="ts">
  import { relay } from "../lib/state/relay.svelte";
  import type { ExportFormat } from "../lib/types";

  const FORMATS: { id: ExportFormat | "ahk" | "exe"; label: string; sub: string; later?: boolean }[] = [
    { id: "rly", label: "Relay macro", sub: ".rly — editable, keeps timing" },
    { id: "json", label: "JSON events", sub: ".json — raw event stream for devs" },
    { id: "ahk", label: "AutoHotkey v2", sub: ".ahk script — runs without Relay", later: true },
    { id: "exe", label: "Standalone .exe", sub: "Portable runner", later: true },
  ];
</script>

<div class="dialog-backdrop" role="presentation" onclick={() => (relay.exportOpen = false)}>
  <div
    class="dialog"
    role="dialog"
    aria-modal="true"
    aria-labelledby="export-title"
    tabindex="-1"
    onclick={(e) => e.stopPropagation()}
    onkeydown={(e) => e.stopPropagation()}
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
      <button class="btn btn-primary save" onclick={relay.doExport}>{relay.editable ? "Save…" : "Download"}</button>
      <button class="btn btn-ghost cancel" onclick={() => (relay.exportOpen = false)}>Cancel</button>
    </div>
  </div>
</div>

<style>
  .dialog-backdrop {
    z-index: 10;
  }
  .dialog {
    width: 440px;
    background: var(--color-bg);
    border: 2px solid var(--color-text);
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
