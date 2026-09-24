<script lang="ts">
  import Kbd from "../../ui/Kbd.svelte";
  import Icon from "../../ui/Icon.svelte";
  import { relay } from "../../../lib/state/relay.svelte";
  import { fmtLastRun, plural } from "../../../lib/format";
</script>

<div class="list">
  {#each relay.library as e (e.id)}
    <div
      class="item"
      class:active={e.id === relay.view?.id}
      role="button"
      tabindex="0"
      onclick={() => relay.loadMacro(e.id)}
      onkeydown={(ev) => ev.key === "Enter" && relay.loadMacro(e.id)}
    >
      <div class="top">
        <span class="name">{e.id === relay.view?.id ? relay.name : e.name}</span>
        {#if relay.editable && relay.mode === "idle"}
          <span class="actions">
            <button
              title="Duplicate"
              aria-label="Duplicate {e.name}"
              onclick={(ev) => {
                ev.stopPropagation();
                relay.duplicateMacro(e.id);
              }}><Icon name="copy" size={13} /></button
            >
            <button
              title="Delete"
              aria-label="Delete {e.name}"
              onclick={(ev) => {
                ev.stopPropagation();
                relay.deleteMacro(e.id);
              }}><Icon name="trash" size={13} /></button
            >
          </span>
        {/if}
        <Kbd combo={e.hotkey ?? "—"} muted />
      </div>
      <div class="meta">
        <span>{(e.duration / 1000).toFixed(1)} s · {plural(e.step_count, "step")} · {plural(e.runs, "run")}</span>
        <span>{fmtLastRun(e.last_run)}</span>
      </div>
    </div>
  {/each}
  <div class="footer">
    <span class="note">New recordings are saved here automatically.</span>
    {#if relay.editable}
      <button class="btn btn-ghost import" onclick={relay.importMacros}>Import…<Icon name="import" size={13} /></button>
    {/if}
  </div>
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
  .footer {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 8px 8px 12px;
  }
  .note {
    flex: 1;
    font-size: 11px;
    color: var(--color-neutral-600);
  }
  .import {
    font-size: 12px;
    padding: 4px 6px;
  }
  .actions {
    display: none;
    gap: 2px;
  }
  .item:hover .actions,
  .item:focus-within .actions {
    display: flex;
  }
  .actions button {
    width: 22px;
    height: 22px;
    border: 0;
    background: transparent;
    color: var(--color-neutral-700);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0;
  }
  .actions button:hover {
    color: var(--color-accent);
  }
</style>
