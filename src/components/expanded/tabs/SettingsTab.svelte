<script lang="ts">
  import Toggle from "../../ui/Toggle.svelte";
  import Segmented from "../../ui/Segmented.svelte";
  import { relay } from "../../../lib/state/relay.svelte";
  import { plural } from "../../../lib/format";
  import type { CoordMode } from "../../../lib/types";
  import type { PreviewBackground } from "../../../lib/ipc/bindings/PreviewBackground";

  const pb = $derived(relay.playback);
  const st = $derived(relay.settings);
  const data = $derived(relay.dataFile);
  const fileName = (path: string) => path.split(/[\\/]/).pop();

  // The file may have changed on disk since the macro was opened.
  $effect(() => void relay.loadDataFile());
</script>

<div class="list">
  <div class="section">Playback</div>
  <div class="row">
    <div class="grow"><div class="title">Humanize</div><div class="sub">Randomize delays ±{pb.jitter_ms} ms</div></div>
    <Toggle label="Humanize" on={pb.humanize} onchange={(v) => relay.setPlayback({ humanize: v })} />
  </div>
  <div class="row slider">
    <input
      type="range"
      min="0"
      max="200"
      step="5"
      aria-label="Jitter"
      value={pb.jitter_ms}
      disabled={!pb.humanize}
      oninput={(e) => relay.previewPlayback({ jitter_ms: +e.currentTarget.value })}
      onchange={(e) => relay.setPlayback({ jitter_ms: +e.currentTarget.value })}
    />
  </div>
  <div class="row">
    <div class="grow title">Coordinates</div>
    <Segmented
      label="Coordinates"
      options={[["screen", "Screen"], ["window", "Window"]] as [CoordMode, string][]}
      value={pb.coord_mode}
      onchange={(v) => relay.setPlayback({ coord_mode: v })}
    />
  </div>
  <div class="row">
    <div class="grow">
      <div class="title">Data file</div>
      {#if data}
        <div class="sub" title={data.path}>{fileName(data.path)}: {plural(data.rows, "row")}, played once each</div>
        {#if data.error}<div class="sub wrong" role="alert">{data.error}</div>{/if}
      {:else}
        <div class="sub">A CSV file: the macro plays once per row, and Text steps type its columns</div>
      {/if}
    </div>
    {#if data}<button class="btn btn-secondary" onclick={relay.removeDataFile}>Remove</button>{/if}
    <button class="btn btn-secondary" onclick={relay.chooseDataFile}>{data ? "Change…" : "Choose…"}</button>
  </div>
  <div class="row">
    <div class="grow"><div class="title">Stop on key press</div><div class="sub">Any keystroke aborts playback</div></div>
    <Toggle label="Stop on key press" on={pb.stop_on_key} onchange={(v) => relay.setPlayback({ stop_on_key: v })} />
  </div>

  <div class="section">Preview</div>
  <div class="row">
    <div class="grow">
      <div class="title">Background</div>
      <div class="sub">The screenshot, when the macro has one</div>
    </div>
    <Segmented
      label="Background"
      options={[["screen", "Screen"], ["sketch", "Sketch"]] as [PreviewBackground, string][]}
      value={st.preview_background}
      onchange={(v) => relay.updateSettings({ preview_background: v })}
    />
  </div>
</div>

<style>
  .list {
    flex: 1;
    overflow-y: auto;
    font-size: 13px;
  }
  .section {
    padding: 14px 12px 4px;
    font-size: 10px;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    font-weight: 600;
    color: var(--color-accent-700);
  }
  .section:first-child {
    padding-top: 10px;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 12px;
    border-bottom: 1px solid var(--color-neutral-300);
  }
  .slider {
    padding: 6px 12px 10px;
  }
  .slider input {
    width: 100%;
    accent-color: var(--color-accent);
  }
  .grow {
    flex: 1;
  }
  .title {
    font-weight: 600;
  }
  .sub {
    font-size: 11px;
    color: var(--color-neutral-600);
  }
  .wrong {
    color: var(--color-accent-700);
  }
</style>
