<script lang="ts">
  import Icon from "../ui/Icon.svelte";
  import { relay } from "../../lib/state/relay.svelte";

  /** `bar`: full-height 64px cells (compact player); `square`: 52×52 (transport row). */
  let { variant }: { variant: "bar" | "square" } = $props();
</script>

<button
  class="rec {variant}"
  title={relay.recording ? "Stop recording (F9)" : "Record (F9)"}
  aria-label={relay.recording ? "Stop recording" : "Record"}
  onclick={relay.toggleRec}
>
  {#if relay.recording}<Icon name="square" size={20} />{:else}<Icon name="record" size={22} />{/if}
</button>
<button
  class="play {variant}"
  title="Play / pause (F10)"
  aria-label={relay.mode === "playing" ? "Pause" : "Play"}
  onclick={relay.togglePlay}
>
  {#if relay.mode === "playing"}<Icon name="pause" size={20} />{:else}<Icon name="play" size={20} />{/if}
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
  .square {
    width: 52px;
    height: 52px;
  }
  .rec {
    background: var(--color-accent);
  }
  .rec:hover {
    background: var(--color-accent-600);
  }
  .rec:active {
    background: var(--color-accent-700);
  }
  .play {
    background: var(--color-text);
  }
  .play:hover {
    background: var(--color-neutral-800);
  }
</style>
