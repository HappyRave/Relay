<script lang="ts">
  // The macro editor's top bar: everything about the macro as a whole. Back,
  // its name, Saved, undo and redo, record, play with how it will play, more,
  // the player, and the window's controls.
  import Key from "../kit/Key.svelte";
  import Chip from "../kit/Chip.svelte";
  import Menu, { type MenuItem } from "../kit/Menu.svelte";
  import Icon from "../ui/Icon.svelte";
  import WindowControls from "../window/WindowControls.svelte";
  import { relay } from "../../lib/state/relay.svelte";
  import { playSummary } from "../../lib/editor/summary";
  import { ariaKeys, keyHint } from "../../lib/hotkeys";

  const keys = $derived(relay.settings.keys);

  /** How long "Saved" shows after a change. */
  const SAVED_MS = 2000;

  const summary = $derived(playSummary(relay.playback, relay.dataFile && !relay.dataFile.error ? relay.dataFile.rows : null));
  const idle = $derived(relay.mode === "idle");
  const pausing = $derived(relay.mode === "playing" && relay.playingHere);

  let saved = $state(false);
  $effect(() => {
    if (!relay.saves) return;
    saved = true;
    const timer = setTimeout(() => (saved = false), SAVED_MS);
    return () => clearTimeout(timer);
  });

  let nameField: HTMLInputElement | undefined = $state();
  // A new recording: its name, selected, ready to type.
  $effect(() => {
    if (!nameField || !relay.naming || relay.naming !== relay.view?.id || relay.mode !== "idle") return;
    relay.naming = null;
    nameField.focus();
    nameField.select();
  });

  let more: HTMLElement | undefined = $state();
  let moreOpen = $state(false);
  const MORE: MenuItem[] = [
    { id: "export", label: "Export…", icon: "export" },
    { id: "duplicate", label: "Duplicate", icon: "duplicate" },
    { id: "runs", label: "Runs", icon: "history" },
    "separator",
    { id: "delete", label: "Delete", icon: "trash", danger: true },
  ];

  async function choose(id: string) {
    const macro = relay.view?.id;
    if (!macro) return;
    if (id === "export") relay.exportOpen = true;
    else if (id === "runs") await relay.showRunsOf(macro);
    else if (id === "duplicate") {
      await relay.duplicateMacro(macro);
      relay.go("macro");
    } else if (id === "delete") await relay.deleteMacro(macro);
  }

  function showPlayOptions() {
    relay.inspector = "macro";
    relay.macroSection = "play";
    relay.inspectorOpen = true;
  }
</script>

<header class="bar" data-tauri-drag-region>
  <Key
    variant="ghost"
    size="sm"
    aria-label="Back to the Library"
    aria-keyshortcuts="Alt+ArrowLeft"
    title="Back to the Library (Alt + ←)"
    disabled={!idle}
    onclick={relay.back}><Icon name="back" /><span class="wide">Library</span></Key
  >
  <!-- While recording, the name is the new recording's (it's named once saved). -->
  <input
    bind:this={nameField}
    class="name"
    aria-label="Macro name"
    spellcheck="false"
    value={relay.recording ? "New recording" : relay.name}
    disabled={!idle || !relay.view}
    readonly={!relay.editable}
    oninput={(e) => relay.rename(e.currentTarget.value)}
    onblur={relay.endRename}
  />
  <span class="saved" class:failed={relay.unsaved} role="status">
    {#if relay.unsaved}<Icon name="warning" size={12} /><span class="wide">Couldn't save</span>
    {:else if saved}<Icon name="check" size={12} /><span class="wide">Saved</span>{/if}
  </span>
  <span class="spacer" data-tauri-drag-region></span>
  {#if relay.editable}
    <Key size="sm" iconOnly aria-label="Undo" title="Undo (Ctrl + Z)" disabled={!relay.canUndo} onclick={relay.undo}>
      <Icon name="undo" />
    </Key>
    <Key size="sm" iconOnly aria-label="Redo" title="Redo (Ctrl + Y)" disabled={!relay.canRedo} onclick={relay.redo}>
      <Icon name="redo" />
    </Key>
  {/if}
  <Key
    variant="red"
    round
    live={relay.recording}
    aria-label={relay.recording ? "Stop recording" : "Record a new macro"}
    aria-keyshortcuts={ariaKeys(keys.record)}
    title="{relay.recording ? 'Stop recording' : 'Record a new macro'}{keyHint(keys.record)}"
    disabled={relay.playing}
    onclick={relay.toggleRec}><Icon name="record" size={14} /></Key
  >
  <Key
    variant="ink"
    aria-label={pausing ? "Pause" : "Play"}
    aria-keyshortcuts={ariaKeys(keys.play)}
    title="{pausing ? 'Pause' : 'Play from the playhead'}{keyHint(keys.play)}"
    disabled={!relay.view || relay.recording || relay.playingElsewhere}
    onclick={relay.togglePlay}><Icon name={pausing ? "pause" : "play"} size={14} /><span class="wide">{pausing ? "Pause" : "Play"}</span></Key
  >
  {#if relay.playingHere}
    <Key size="sm" iconOnly aria-label="Stop" title="Stop (Esc)" onclick={relay.stop}><Icon name="stop" size={14} /></Key>
  {/if}
  <Chip label={summary.label} onclick={showPlayOptions}>
    {#each summary.chips as c, i (i)}{#if i}<span class="sep" aria-hidden="true">·</span>{/if}<span>{c}</span>{/each}
    {#if summary.keysDontStop}<span class="sep" aria-hidden="true">·</span><span class="warn">keys don't stop it</span>{/if}
  </Chip>
  <span bind:this={more}>
    <Key
      variant="ghost"
      size="sm"
      iconOnly
      aria-label="More"
      title="More"
      aria-haspopup="menu"
      aria-expanded={moreOpen}
      disabled={!relay.view || !idle}
      onclick={() => (moreOpen = !moreOpen)}><Icon name="more" /></Key
    >
  </span>
  <Key
    size="sm"
    iconOnly
    aria-label="Switch to the player"
    aria-keyshortcuts={ariaKeys(keys.player)}
    title="Switch to the player{keyHint(keys.player)}"
    onclick={() => relay.setPlayerMode(true)}><Icon name="collapse" /></Key
  >
  <WindowControls />
</header>
{#if moreOpen && more}
  <Menu items={MORE} opener={more.querySelector("button")!} label="More" onselect={choose} onclose={() => (moreOpen = false)} />
{/if}

<style>
  .bar {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    height: var(--key-h);
    padding: 0 2px 0 6px;
  }
  .name {
    flex: 0 1 auto;
    min-width: 80px;
    max-width: 46%;
    field-sizing: content;
    padding: 2px 6px;
    margin-left: -2px;
    border: 0;
    border-radius: var(--r-sm);
    font: inherit;
    font-size: 17px;
    font-weight: 800;
    letter-spacing: -0.01em;
    color: var(--text);
    background: none;
    user-select: text;
  }
  .name:hover:not(:disabled) {
    background: var(--hover);
  }
  .name:focus {
    outline: none;
    background: var(--plate);
    box-shadow: inset 0 0 0 1.5px var(--text);
  }
  .saved {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    min-width: 0;
    font-size: 12px;
    font-weight: 600;
    white-space: nowrap;
    color: var(--text-2);
  }
  .saved :global(svg) {
    color: var(--green);
  }
  .saved.failed,
  .saved.failed :global(svg) {
    color: var(--red-text);
  }
  .spacer {
    flex: 1;
    align-self: stretch;
  }
  .sep {
    color: var(--text-2);
  }
  .warn {
    color: var(--red-text);
  }
  @container editor (max-width: 860px) {
    .wide {
      display: none;
    }
  }
</style>
