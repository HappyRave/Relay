<script lang="ts">
  import Kbd from "../../ui/Kbd.svelte";
  import { relay } from "../../../lib/state/relay.svelte";
</script>

<div class="list">
  {#each relay.libraryItems as e (e.id)}
    <div
      class="item"
      class:active={e.id === relay.current.id}
      role="button"
      tabindex="0"
      onclick={() => relay.loadMacro(e.id)}
      onkeydown={(ev) => ev.key === "Enter" && relay.loadMacro(e.id)}
    >
      <div class="top">
        <span class="name">{e.name}</span>
        <Kbd combo={e.hotkey} muted />
      </div>
      <div class="meta">
        <span>{(e.durationMs / 1000).toFixed(1)} s · {e.stepCount} events · {e.runs} runs</span>
        <span>{e.lastRun}</span>
      </div>
    </div>
  {/each}
  <div class="note">New recordings are saved here automatically.</div>
</div>

<style>
  .list {
    flex: 1;
    overflow-y: auto;
  }
  .item {
    display: flex;
    flex-direction: column;
    gap: 3px;
    padding: 10px 12px 10px 9px;
    border-left: 3px solid transparent;
    border-bottom: 1px solid var(--color-neutral-300);
    cursor: pointer;
  }
  .item:hover {
    background: var(--color-neutral-200);
  }
  .item.active {
    background: var(--color-accent-100);
    border-left-color: var(--color-accent);
  }
  .item:focus-visible {
    outline-offset: -2px;
  }
  .top {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .name {
    flex: 1;
    font-size: 13px;
    font-weight: 800;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .meta {
    display: flex;
    gap: 8px;
    justify-content: space-between;
    font-size: 11px;
    color: var(--color-neutral-700);
  }
  .note {
    padding: 10px 12px;
    font-size: 11px;
    color: var(--color-neutral-600);
  }
</style>
