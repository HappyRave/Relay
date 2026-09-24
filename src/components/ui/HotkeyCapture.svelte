<script lang="ts">
  // Click, then press the combination. Esc cancels, Backspace clears.
  import Kbd from "./Kbd.svelte";
  import { comboOf } from "../../lib/hotkeys";

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
    const onKey = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();
      if (e.key === "Escape") return void (capturing = false);
      if (e.key === "Backspace") {
        capturing = false;
        return onchange("");
      }
      if (["Control", "Alt", "Shift", "Meta"].includes(e.key)) return;
      const c = comboOf(e);
      if (!c) return void (hint = "Use a letter, digit, F-key or arrow");
      capturing = false;
      onchange(c);
    };
    const cancel = () => (capturing = false);
    window.addEventListener("keydown", onKey, true);
    window.addEventListener("blur", cancel);
    return () => {
      window.removeEventListener("keydown", onKey, true);
      window.removeEventListener("blur", cancel);
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
