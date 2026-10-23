<script lang="ts">
  // The Library, Relay's home: every macro, filtered by what runs them or when
  // they last ran, and Activity, the run history and its problems.
  import TopBar from "../window/TopBar.svelte";
  import Key from "../kit/Key.svelte";
  import Icon from "../ui/Icon.svelte";
  import Toast from "../shared/Toast.svelte";
  import MacroTable from "./MacroTable.svelte";
  import Activity from "./Activity.svelte";
  import { relay } from "../../lib/state/relay.svelte";

  const RECENT_MS = 7 * 86_400_000;
  const count = $derived({
    all: relay.library.length,
    triggers: relay.library.filter((m) => !!m.hotkey || m.also.length > 0).length,
    recent: relay.library.filter((m) => !!m.last_run && Date.now() - Date.parse(m.last_run) < RECENT_MS).length,
  });
  const macros = (filter: "all" | "triggers" | "recent") => {
    relay.libFilter = filter;
    relay.showLibrary("macros");
  };
  const current = (filter: string) => (relay.libraryShows === "macros" ? relay.libFilter === filter : relay.libraryShows === filter);
</script>

<div class="library">
  <TopBar>
    {#snippet title()}<span class="brand" data-tauri-drag-region><span class="mark"></span>Relay</span>{/snippet}
    {#snippet actions()}
      <Key
        variant="red"
        live={relay.recording}
        title="{relay.recording ? 'Stop recording' : 'Record a new macro'} (F9)"
        disabled={relay.playing}
        onclick={relay.toggleRec}><Icon name="record" size={14} />{relay.recording ? "Stop" : "Record"}</Key
      >
      <Key
        size="sm"
        iconOnly
        aria-label="Switch to the player"
        aria-keyshortcuts="Control+Shift+M"
        title="Switch to the player (Ctrl + Shift + M)"
        onclick={() => (relay.expanded = false)}><Icon name="collapse" /></Key
      >
      <Key size="sm" onclick={() => relay.go("settings")}><Icon name="sliders" />Settings</Key>
    {/snippet}
  </TopBar>
  <nav class="side" aria-label="Library">
    <span class="caps head">Macros</span>
    {#each [["all", "All"], ["triggers", "With triggers"], ["recent", "Recently run"]] as const as [id, label] (id)}
      <button type="button" class="navi" aria-current={current(id) ? "page" : undefined} onclick={() => macros(id)}>
        {label}<span class="n num">{count[id]}</span>
      </button>
    {/each}
    <span class="caps head">Activity</span>
    <button type="button" class="navi" aria-current={current("runs") ? "page" : undefined} onclick={() => relay.showLibrary("runs")}>All runs</button>
    <button type="button" class="navi" aria-current={current("problems") ? "page" : undefined} onclick={() => relay.showLibrary("problems")}>
      Problems{#if relay.problems}<span class="badge" aria-label="{relay.problems} problems">{relay.problems}</span>{/if}
    </button>
    <span class="spacer"></span>
    {#if relay.editable}
      <button type="button" class="navi" disabled={relay.mode !== "idle"} onclick={relay.importMacros}><span class="with-icon"><Icon name="import" />Import…</span></button>
    {/if}
  </nav>
  <main class="plate body">
    {#if relay.triggersPaused}
      <div class="notice" role="status">
        <Icon name="warning" size={14} /><span>Triggers are paused (the kill switch).</span>
        <Key size="sm" onclick={() => relay.setTriggersPaused(false)}>Resume</Key>
      </div>
    {/if}
    {#if relay.libraryShows === "macros"}<MacroTable />{:else}<Activity />{/if}
    <Toast />
  </main>
</div>

<style>
  .library {
    display: grid;
    grid-template-columns: 196px minmax(0, 1fr);
    grid-template-rows: auto minmax(0, 1fr);
    gap: var(--gap);
    height: 100%;
    min-height: 0;
  }
  .library > :global(.topbar) {
    grid-column: 1 / -1;
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
  .side {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-height: 0;
    padding: 2px 2px 4px;
  }
  .head {
    padding: 10px 10px 4px;
    color: var(--text);
  }
  .navi {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
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
  .navi:hover:not(:disabled) {
    background: var(--hover);
  }
  .navi[aria-current="page"] {
    background: var(--plate);
    box-shadow: 0 0 0 1px var(--line-strong);
  }
  .n {
    font-size: 13px;
    font-weight: 500;
  }
  .badge {
    min-width: 20px;
    height: 20px;
    padding: 0 6px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: 10px;
    font-size: 12px;
    font-weight: 800;
    color: #fff;
    background: var(--red);
  }
  .with-icon {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .spacer {
    flex: 1;
  }
  .body {
    display: flex;
    flex-direction: column;
    min-height: 0;
    overflow: hidden;
  }
  .notice {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 10px var(--pad) 0;
    padding: 8px 10px;
    border-radius: var(--r-md);
    font-size: 13px;
    background: var(--plate-2);
  }
  .notice span {
    flex: 1;
  }
</style>
