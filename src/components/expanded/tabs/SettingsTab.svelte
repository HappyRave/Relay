<script lang="ts">
  import Toggle from "../../ui/Toggle.svelte";
  import Segmented from "../../ui/Segmented.svelte";
  import Kbd from "../../ui/Kbd.svelte";
  import { relay } from "../../../lib/state/relay.svelte";
  import type { CoordMode } from "../../../lib/types";
  import type { PathMode } from "../../../lib/ipc/bindings/PathMode";

  const HOTKEYS = [
    ["Start / stop recording", "F9"],
    ["Play / pause", "F10"],
    ["Stop everything", "Esc"],
    ["Toggle compact player", "Ctrl + Shift + M"],
    ["Emergency kill switch", "Ctrl + Alt + End"],
  ];
  const pb = $derived(relay.playback);
  const st = $derived(relay.settings);
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
      oninput={(e) => relay.setPlayback({ jitter_ms: +e.currentTarget.value })}
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
    <div class="grow"><div class="title">Stop on key press</div><div class="sub">Any keystroke aborts playback</div></div>
    <Toggle label="Stop on key press" on={pb.stop_on_key} onchange={(v) => relay.setPlayback({ stop_on_key: v })} />
  </div>

  <div class="section">Recording</div>
  <div class="row">
    <div class="grow title">Capture mouse path</div>
    <Toggle label="Capture mouse path" on={st.capture_moves} onchange={(v) => relay.updateSettings({ capture_moves: v })} />
  </div>
  <div class="row">
    <div class="grow title">Capture keystrokes</div>
    <Toggle label="Capture keystrokes" on={st.capture_keys} onchange={(v) => relay.updateSettings({ capture_keys: v })} />
  </div>
  <div class="row">
    <div class="grow title">3-second countdown</div>
    <Toggle label="3-second countdown" on={st.countdown} onchange={(v) => relay.updateSettings({ countdown: v })} />
  </div>
  <div class="row">
    <div class="grow">
      <div class="title">Ignore simulated input</div>
      <div class="sub">Turn off for remote-desktop and KVM tools</div>
    </div>
    <Toggle
      label="Ignore simulated input"
      on={st.ignore_injected}
      onchange={(v) => relay.updateSettings({ ignore_injected: v })}
    />
  </div>

  <div class="section">Preview</div>
  <div class="row">
    <div class="grow title">Mouse path</div>
    <Segmented
      label="Mouse path"
      options={[["full", "Full path"], ["trail", "Trail only"]] as [PathMode, string][]}
      value={st.path_mode}
      onchange={(v) => relay.updateSettings({ path_mode: v })}
    />
  </div>
  <div class="row">
    <div class="grow title">Click labels</div>
    <Toggle
      label="Click labels"
      on={st.show_click_labels}
      onchange={(v) => relay.updateSettings({ show_click_labels: v })}
    />
  </div>

  <div class="section">Global hotkeys</div>
  {#each HOTKEYS as [label, key] (key)}
    <div class="row">
      <span class="grow title">{label}</span>
      <Kbd combo={key} />
    </div>
  {/each}
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
</style>
