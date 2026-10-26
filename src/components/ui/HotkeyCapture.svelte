<script lang="ts">
  // Click, then press the combination. Esc cancels, Backspace clears. While it
  // listens, the hotkeys let go of the keys, so a taken combination arrives too.
  import Keycaps from "../kit/Keycaps.svelte";
  import { relay } from "../../lib/state/relay.svelte";
  import { listenForCombo } from "../../lib/keycapture";

  let { combo, onchange, disabled = false }: { combo: string; onchange: (combo: string) => void; disabled?: boolean } =
    $props();
  let capturing = $state(false);
  let hint = $state("Press keys…");

  function start() {
    if (disabled) return;
    capturing = true;
    hint = "Press keys…";
  }

  $effect(() => {
    if (!capturing) return;
    relay.holdHotkeys(true);
    const stop = listenForCombo({
      done: (c) => {
        capturing = false;
        onchange(c ?? "");
      },
      cancel: () => (capturing = false),
      hint: (text) => (hint = text),
    });
    return () => {
      stop();
      relay.holdHotkeys(false);
    };
  });
</script>

<button
  class="capture"
  data-captures-keys
  class:capturing
  {disabled}
  title="Click, then press the keys (Backspace clears, Esc cancels)"
  onclick={start}
>
  {#if capturing}
    <span class="hint">{hint}</span>
  {:else}
    <Keycaps combo={combo ? combo.split(" + ") : []} none="Set…" />
  {/if}
</button>

<style>
  .capture {
    display: inline-flex;
    align-items: center;
    min-height: var(--key-sm);
    padding: 2px 4px;
    border: 0;
    border-radius: var(--r-sm);
    background: transparent;
    cursor: pointer;
    font: inherit;
    color: var(--text);
  }
  .capture:hover:not(:disabled) {
    background: var(--hover);
  }
  .capture:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }
  .capturing {
    box-shadow: inset 0 0 0 1.5px var(--text);
  }
  .hint {
    font-size: 12px;
    font-weight: 700;
    color: var(--red-text);
  }
</style>
