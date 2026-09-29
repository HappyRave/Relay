<script lang="ts">
  import Icon from "../ui/Icon.svelte";
  import { relay } from "../../lib/state/relay.svelte";

  /** `bar`: full-height 64px cells (compact player); `strip`: 64 px cells of the editor's 48 px control strip. */
  let { variant }: { variant: "bar" | "strip" } = $props();
</script>

<button
  class="rec {variant}"
  title={relay.recording ? "Stop recording (F9)" : "Record (F9)"}
  aria-label={relay.recording ? "Stop recording" : "Record"}
  disabled={relay.playing}
  onclick={relay.toggleRec}
>
  {#if relay.recording}<Icon name="square" size={20} />{:else}<Icon name="record" size={variant === "strip" ? 20 : 22} />{/if}
</button>
<button
  class="play {variant}"
  title="Play / pause (F10)"
  aria-label={relay.mode === "playing" ? "Pause" : "Play"}
  disabled={relay.recording}
  onclick={relay.togglePlay}
>
  {#if relay.mode === "playing"}<Icon name="pause" size={20} />{:else}<Icon name="play" size={variant === "strip" ? 22 : 20} />{/if}
</button>

<style>
  button {
    border: 0;
    color: var(--color-bg);
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    flex: none;
  }
  .bar {
    width: 64px;
  }
  /* In the strip, each cell draws its own left rule, in its own color. */
  .strip {
    width: 64px;
    border-left: 1px solid;
  }
  /* Stronger than the strip's own cell rules (the transport's), which are for its plain cells. */
  .strip.rec,
  .strip.rec:disabled {
    background: var(--color-accent);
    border-left-color: var(--color-accent);
    color: var(--color-bg);
  }
  .strip.rec:hover:not(:disabled) {
    background: var(--color-accent-600);
  }
  .strip.play,
  .strip.play:disabled {
    background: var(--color-text);
    border-left-color: var(--color-text);
    color: var(--color-bg);
  }
  .strip.play:hover:not(:disabled) {
    background: var(--color-neutral-800);
  }
  .rec {
    background: var(--color-accent);
  }
  .rec:hover:not(:disabled) {
    background: var(--color-accent-600);
  }
  .rec:active:not(:disabled) {
    background: var(--color-accent-700);
  }
  .play {
    background: var(--color-text);
  }
  .play:hover:not(:disabled) {
    background: var(--color-neutral-800);
  }
  button:disabled {
    cursor: not-allowed;
    opacity: 0.45;
  }
</style>
