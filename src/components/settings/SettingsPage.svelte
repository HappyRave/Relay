<script lang="ts">
  // Settings: Relay itself, in sections. What a macro does when it plays is
  // the macro's own, in the editor.
  import TopBar from "../window/TopBar.svelte";
  import Switch from "../kit/Switch.svelte";
  import Choice from "../kit/Choice.svelte";
  import Keycaps from "../kit/Keycaps.svelte";
  import Toast from "../shared/Toast.svelte";
  import { relay } from "../../lib/state/relay.svelte";
  import { isTauri } from "../../lib/platform/window";
  import { glassOn } from "../../lib/look.svelte";
  import type { Settings } from "../../lib/types";

  type SectionId = "recording" | "keys" | "window" | "appearance" | "sound";
  // The window settings need the app: the browser preview has no window to set.
  const SECTIONS = (
    [
      ["recording", "Recording"],
      ["keys", "Keys"],
      ["window", "Window"],
      ["appearance", "Appearance"],
      ["sound", "Sound"],
    ] as [SectionId, string][]
  ).filter(([id]) => id !== "window" || isTauri());
  const KEYS: [string, string, string[]][] = [
    ["Record", "Start and stop a recording", ["F9"]],
    ["Play / pause", "Play the open macro from the playhead", ["F10"]],
    ["Stop", "Stops a recording or playback", ["Esc"]],
    ["Show the player", "Switch between the window and the player", ["Ctrl", "Shift", "M"]],
    ["Stop everything", "Stops any recording or playback and pauses triggers", ["Ctrl", "Alt", "End"]],
  ];

  let section = $state<SectionId>("recording");
  const st = $derived(relay.settings);
  const look = $derived(relay.systemLook);
  const set = (patch: Partial<Settings>) => relay.updateSettings(patch);
  const glassHint = $derived(
    !look.transparency
      ? "Windows' transparency effects are off, so Relay is solid."
      : look.glass_default
        ? "On by default on Windows 11."
        : "Off by default on Windows 10, where it can slow down moving the window.",
  );
</script>

{#snippet toggle(key: keyof Settings, label: string, hint = "")}
  <div class="setting">
    <span class="what"><b id="s-{key}">{label}</b>{#if hint}<span>{hint}</span>{/if}</span>
    <Switch labelledby="s-{key}" on={st[key] as boolean} onchange={(v) => set({ [key]: v })} />
  </div>
{/snippet}

{#snippet choice<T>(key: keyof Settings, label: string, options: [T, string][], hint = "")}
  <div class="setting">
    <span class="what"><b id="s-{key}">{label}</b>{#if hint}<span>{hint}</span>{/if}</span>
    <Choice labelledby="s-{key}" {options} value={st[key] as T} onchange={(v) => set({ [key]: v })} />
  </div>
{/snippet}

<div class="settings">
  <TopBar back>
    {#snippet title()}<h1 class="page-title" data-tauri-drag-region>Settings</h1>{/snippet}
  </TopBar>
  <nav class="side" aria-label="Settings sections">
    {#each SECTIONS as [id, label] (id)}
      <button type="button" class="navi" aria-current={section === id ? "page" : undefined} onclick={() => (section = id)}>
        {label}
      </button>
    {/each}
  </nav>
  <div class="plate body">
    <section class="sections" aria-labelledby="sec-{section}">
      <h2 class="caps" id="sec-{section}">{SECTIONS.find(([id]) => id === section)?.[1]}</h2>
      {#if section === "recording"}
        <p class="note">Recordings store what you type, passwords included. Stop recording before typing anything secret.</p>
        {@render toggle("capture_moves", "Capture the mouse path", "Off records only where you clicked.")}
        {@render toggle("capture_keys", "Capture keystrokes")}
        {@render toggle("capture_screen", "Screenshot", "A picture of the screen as recording starts, for the preview. Kept on this PC only.")}
        {@render toggle("countdown", "Count down 3, 2, 1")}
        {@render toggle("esc_stops_recording", "Esc stops recording", "Turn off to record Esc; stop with F9.")}
        {@render toggle("ignore_injected", "Ignore simulated input", "Turn off for Remote Desktop and KVM tools: they send your typing as simulated input.")}
      {:else if section === "keys"}
        <p class="note">These work from any app.</p>
        {#each KEYS as [label, hint, combo] (label)}
          <div class="setting">
            <span class="what"><b>{label}</b><span>{hint}</span></span>
            <Keycaps {combo} />
          </div>
        {/each}
      {:else if section === "window"}
        {@render choice("keep_on_top", "Keep the player on top", [
          ["always", "Always"],
          ["sessions", "While working"],
          ["never", "Never"],
        ])}
        {@render toggle("hide_while_recording", "Hide the window while recording", "The player shows instead, so Relay never covers what you record.")}
        {@render toggle("hide_while_playing", "Hide the window while playing", "Off, the window stays where it is, behind your apps, so you can watch the steps.")}
        {@render toggle("close_to_tray", "Close to the tray", "Hotkeys and triggers keep running; quit from the tray icon.")}
        <div class="setting">
          <span class="what"><b id="s-autostart">Start with Windows</b><span>Starts in the tray, so triggers run after you sign in.</span></span>
          <Switch labelledby="s-autostart" on={relay.autostart} onchange={relay.setAutostart} />
        </div>
      {:else if section === "appearance"}
        {@render choice("theme", "Theme", [
          ["system", "Like Windows"],
          ["light", "Light"],
          ["dark", "Dark"],
        ])}
        <div class="setting">
          <span class="what"><b id="s-glass">Frosted glass</b><span>{glassHint}</span></span>
          <Switch
            labelledby="s-glass"
            on={glassOn(st, look)}
            disabled={!look.transparency}
            onchange={(v) => set({ glass: v })}
          />
        </div>
        {@render choice("spacing", "Spacing", [
          ["compact", "Compact"],
          ["comfortable", "Comfortable"],
          ["large", "Large"],
        ])}
        {@render choice("text_size", "Text size", [
          [100, "100%"],
          [125, "125%"],
          [150, "150%"],
          [200, "200%"],
        ], "Everything grows with it, the window too.")}
        {@render choice("player_size", "Player size", [
          ["compact", "Compact"],
          ["large", "Large"],
        ])}
        {@render choice("reduce_motion", "Reduce motion", [
          ["system", "Like Windows"],
          ["always", "Always"],
        ])}
        {@render toggle("show_click_labels", "Show click labels", "Next to the numbered clicks in the preview.")}
        {@render choice("path_mode", "Mouse path", [
          ["full", "Whole path"],
          ["trail", "Played part"],
        ])}
      {:else}
        {@render toggle("sounds", "Sounds", "Short clicks when recording starts and stops, and when a macro finishes or fails.")}
      {/if}
    </section>
    <div class="toasts"><Toast /></div>
  </div>
</div>

<style>
  .settings {
    display: grid;
    grid-template-columns: 196px minmax(0, 1fr);
    grid-template-rows: auto minmax(0, 1fr);
    gap: var(--gap);
    height: 100%;
    min-height: 0;
  }
  .settings > :global(.topbar) {
    grid-column: 1 / -1;
  }
  .page-title {
    margin: 0;
    font-size: 17px;
    font-weight: 800;
  }
  .side {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 2px;
  }
  .navi {
    display: flex;
    align-items: center;
    height: var(--row-h);
    padding: 0 10px;
    border: 0;
    border-radius: var(--r-md);
    font: inherit;
    font-weight: 600;
    text-align: left;
    color: var(--text);
    background: none;
    cursor: pointer;
  }
  .navi:hover {
    background: var(--hover);
  }
  .navi[aria-current="page"] {
    background: var(--plate);
    box-shadow: 0 0 0 1px var(--line-strong);
  }
  .body {
    display: flex;
    flex-direction: column;
    min-height: 0;
    overflow: hidden;
  }
  .sections {
    flex: 1;
    overflow: auto;
    padding: var(--pad) calc(var(--pad) + 4px);
  }
  h2 {
    margin: 0 0 2px;
  }
  .note {
    margin: 6px 0 2px;
    font-size: 13px;
    color: var(--text-2);
  }
  .setting {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 4px 16px;
    align-items: center;
    padding: 10px 0;
    border-bottom: 1px solid var(--line);
  }
  .setting:last-child {
    border-bottom: 0;
  }
  .what b {
    display: block;
    font-weight: 700;
  }
  .what span {
    font-size: 13px;
    color: var(--text-2);
  }
  .toasts:empty {
    display: none;
  }
</style>
