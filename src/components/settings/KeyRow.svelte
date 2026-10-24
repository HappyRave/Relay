<script lang="ts">
  // One of Relay's keys in Settings → Keys: its keycaps, Change to press a new
  // combination (Backspace: None, Esc: cancel), and what Relay says about it:
  // why it can't be, or what setting it costs (an AltGr character, an app that
  // uses it).
  import Key from "../kit/Key.svelte";
  import Keycaps from "../kit/Keycaps.svelte";
  import Icon from "../ui/Icon.svelte";
  import { relay } from "../../lib/state/relay.svelte";
  import { listenForCombo } from "../../lib/keycapture";
  import type { RelayKey } from "../../lib/ipc/bindings/RelayKey";

  let { key, label, hint }: { key: RelayKey; label: string; hint: string } = $props();

  const combo = $derived(relay.settings.keys[key]);
  let capturing = $state(false);
  let said = $state<{ text: string; ok: boolean } | null>(null);

  async function chosen(c: string | null) {
    capturing = false;
    const refused = await relay.setRelayKey(key, c);
    if (refused) return void (said = { text: refused, ok: false });
    const warning = c ? await relay.keyWarning(c) : null;
    said = warning
      ? { text: warning, ok: false }
      : { text: c ? `Set to ${c}.` : "Set to None: use the buttons instead.", ok: true };
  }

  $effect(() => {
    if (!capturing) return;
    relay.holdHotkeys(true);
    const stop = listenForCombo({
      done: chosen,
      cancel: () => (capturing = false),
      hint: (text) => (said = { text, ok: false }),
    });
    return () => {
      stop();
      relay.holdHotkeys(false);
    };
  });

  function change() {
    said = null;
    capturing = !capturing;
  }
</script>

<div class="setting" data-captures-keys={capturing || undefined}>
  <span class="what"><b id="key-{key}">{label}</b><span>{hint}</span></span>
  <span class="value">
    {#if capturing}
      <span class="listening" aria-live="polite">Press keys…</span>
    {:else}
      <Keycaps combo={combo ? combo.split(" + ") : []} />
    {/if}
    <Key size="sm" aria-describedby="key-{key}" disabled={!relay.editable} onclick={change}>{capturing ? "Cancel" : "Change"}</Key>
  </span>
  {#if said}
    <span class="said" class:ok={said.ok} role="status">
      <Icon name={said.ok ? "check" : "warning"} size={14} /><span>{said.text}</span>
    </span>
  {/if}
</div>

<style>
  .value {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .listening {
    font-size: 12px;
    font-weight: 700;
    color: var(--red);
  }
  .said {
    grid-column: 1 / -1;
    display: flex;
    align-items: flex-start;
    gap: 6px;
    font-size: 12px;
    color: var(--text);
  }
  .said :global(svg) {
    flex: none;
    margin-top: 1px;
    color: var(--red);
  }
  .said.ok :global(svg) {
    color: var(--green);
  }
</style>
