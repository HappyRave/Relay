<script lang="ts">
  // Click, then press the combination. Esc cancels, Backspace clears. While it
  // listens, the hotkeys let go of the keys, so a taken combination arrives too.
  import Kbd from "./Kbd.svelte";
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
    <Kbd combo={combo || "Set…"} />
  {/if}
</button>

<style>
  .capture {
    border: 0;
    background: transparent;
    padding: 2px;
    cursor: pointer;
    font: inherit;
    color: var(--color-text);
  }
  .capture:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }
  .capturing {
    outline: 2px solid var(--color-accent);
  }
  .hint {
    font-size: 11px;
    font-weight: 800;
    padding: 2px 7px;
    color: var(--color-accent-700);
  }
</style>
