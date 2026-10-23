<script lang="ts">
  // A macro in the editor: the top bar, the steps list beside the inspector,
  // and the view under them.
  import EditorBar from "./EditorBar.svelte";
  import StepList from "./StepList.svelte";
  import Inspector from "./Inspector.svelte";
  import ViewPane from "./ViewPane.svelte";
  import Toast from "../shared/Toast.svelte";
  import { relay } from "../../lib/state/relay.svelte";

  /** Narrower than this, the inspector slides over the list. */
  const NARROW = 860;
  /** Shorter than this, the view starts closed. */
  const SHORT = 470;

  let inspector: Inspector | undefined = $state();
  let width = $state(0);
  let height = $state(0);
  const narrow = $derived(width > 0 && width < NARROW);

  // A macro opens with the view closed when the window is short.
  $effect(() => {
    void relay.view?.id;
    if (height > 0 && height < SHORT) relay.viewOpen = false;
  });

  function key(e: KeyboardEvent) {
    if (e.key === "Escape" && narrow && relay.inspectorOpen && !e.defaultPrevented) {
      relay.inspectorOpen = false;
      document.querySelector<HTMLElement>("[role=listbox]")?.focus();
    }
  }
</script>

<svelte:window onkeydown={key} />
<div class="editor" class:narrow bind:clientWidth={width} bind:clientHeight={height}>
  <EditorBar />
  <div class="main">
    <StepList onenter={() => inspector?.focus()} />
    <Inspector bind:this={inspector} />
  </div>
  <ViewPane />
  <div class="toasts"><Toast /></div>
</div>

<style>
  .editor {
    container: editor / inline-size;
    display: grid;
    grid-template-rows: auto minmax(0, 1fr) auto auto;
    gap: var(--gap);
    height: 100%;
    min-height: 0;
  }
  .main {
    position: relative;
    display: grid;
    grid-template-columns: minmax(0, 1fr) 340px;
    gap: var(--gap);
    min-height: 0;
  }
  .narrow .main {
    grid-template-columns: minmax(0, 1fr);
  }
  .narrow :global(.title-block) {
    display: none;
  }
  .toasts:empty {
    display: none;
  }
</style>
