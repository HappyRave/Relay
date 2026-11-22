<script lang="ts">
  // The window's top bar: it moves the window, and holds the page's title, its
  // actions and the window controls. With `back`, it starts with the way back
  // to the Library (Alt + ←).
  import type { Snippet } from "svelte";
  import Key from "../kit/Key.svelte";
  import Icon from "../ui/Icon.svelte";
  import WindowControls from "./WindowControls.svelte";
  import { relay } from "../../lib/state/relay.svelte";

  let { back = false, title, actions }: { back?: boolean; title?: Snippet; actions?: Snippet } = $props();
</script>

<header class="topbar" data-tauri-drag-region>
  {#if back}
    <Key
      variant="ghost"
      size="sm"
      aria-label="Back to the Library"
      aria-keyshortcuts="Alt+ArrowLeft"
      title="Back to the Library (Alt + ←)"
      onclick={relay.back}><Icon name="back" />Library</Key
    >
  {/if}
  {@render title?.()}
  <span class="spacer" data-tauri-drag-region></span>
  {@render actions?.()}
  <WindowControls />
</header>

<style>
  .topbar {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    height: var(--key-h);
    padding: 0 2px 0 6px;
  }
  .spacer {
    flex: 1;
    align-self: stretch;
  }
</style>
