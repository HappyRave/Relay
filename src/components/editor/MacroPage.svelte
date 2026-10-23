<script lang="ts">
  // A macro in the editor: the top bar, the steps list beside the inspector,
  // and the view under them.
  import EditorBar from "./EditorBar.svelte";
  import StepList from "./StepList.svelte";
  import Inspector from "./Inspector.svelte";
  import ViewPane from "./ViewPane.svelte";
  import Toast from "../shared/Toast.svelte";

  let inspector: Inspector | undefined = $state();
</script>

<div class="editor">
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
    display: grid;
    grid-template-columns: minmax(0, 1fr) 340px;
    gap: var(--gap);
    min-height: 0;
  }
  .toasts:empty {
    display: none;
  }
</style>
