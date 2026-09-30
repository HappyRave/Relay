<script lang="ts">
  import Icon from "../../ui/Icon.svelte";
  import { relay } from "../../../lib/state/relay.svelte";
  import type { RunEntry } from "../../../lib/ipc/bindings/RunEntry";
  import { checkLabel, failed, fmtDuration, fmtRunTime, outcomeLabel, runSettings, sourceLabel } from "../../../lib/runs";
  import { plural } from "../../../lib/format";

  /** The expanded run, by `key`. */
  let open = $state<string | null>(null);
  const key = (r: RunEntry) => `${r.at} ${r.macro_id}`;
  const hasDetails = (r: RunEntry) => r.checks.length > 0 || runSettings(r) !== "";
</script>

<div class="runs">
  <div class="bar">
    <button class="btn btn-ghost back" onclick={() => relay.showRuns(false)}><Icon name="back" size={13} />Macros</button>
    <select class="input filter" aria-label="Show the runs of" bind:value={relay.runsFilter}>
      <option value={null}>All macros</option>
      {#each relay.library as m (m.id)}
        <option value={m.id}>{m.name}</option>
      {/each}
    </select>
  </div>
  <div class="list">
    {#each relay.shownRuns as r (key(r))}
      {@const expanded = open === key(r)}
      <div class="run" class:failed={failed(r)}>
        <button
          class="head"
          aria-expanded={hasDetails(r) ? expanded : undefined}
          disabled={!hasDetails(r)}
          onclick={() => (open = expanded ? null : key(r))}
        >
          <span class="top">
            <span class="name">{r.macro_name}</span>
            <span class="when" title={new Date(r.at).toLocaleString()}>{fmtRunTime(r.at)}</span>
          </span>
          <span class="meta">
            <span>
              {sourceLabel(r.source)} · <span class="outcome">{outcomeLabel(r)}</span>{r.loops > 1 ? ` · ${plural(r.loops, "loop")}` : ""}
            </span>
            {#if r.outcome.type === "finished"}<span>{fmtDuration(r.duration_ms)}</span>{/if}
          </span>
        </button>
        {#if expanded}
          <ul class="details">
            {#if runSettings(r)}<li class="settings">{runSettings(r)}</li>{/if}
            {#if r.checks_dropped > 0}<li class="dropped">{plural(r.checks_dropped, "earlier check")} not kept</li>{/if}
            {#each r.checks as c, i (i)}
              <li class:bad={c.outcome.type === "timed_out"}>{checkLabel(c, r.loops)}</li>
            {/each}
          </ul>
        {/if}
      </div>
    {:else}
      <div class="empty">No runs yet.</div>
    {/each}
  </div>
  <div class="footer">Relay keeps the last 200 runs.</div>
</div>

<style>
  .runs {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 0;
  }
  .bar {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 8px;
    border-bottom: 1px solid var(--color-neutral-300);
  }
  .back {
    font-size: 12px;
    padding: 4px 6px;
    gap: 4px;
  }
  .filter {
    flex: 1;
    min-width: 0;
    min-height: 28px;
    padding: 2px 6px;
    font-size: 12px;
  }
  .list {
    flex: 1;
    overflow-y: auto;
  }
  .run {
    border-bottom: 1px solid var(--color-neutral-300);
    border-left: 3px solid transparent;
  }
  .run.failed {
    border-left-color: var(--color-accent);
  }
  .head {
    display: flex;
    flex-direction: column;
    gap: 3px;
    width: 100%;
    padding: 8px 12px 8px 9px;
    border: 0;
    background: transparent;
    font: inherit;
    color: inherit;
    text-align: left;
    cursor: pointer;
  }
  .head:hover:not(:disabled) {
    background: var(--color-neutral-200);
  }
  .head:disabled {
    cursor: default;
  }
  .head:focus-visible {
    outline-offset: -2px;
  }
  .top,
  .meta {
    display: flex;
    gap: 8px;
    justify-content: space-between;
  }
  .name {
    flex: 1;
    font-size: 13px;
    font-weight: 800;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .when,
  .meta {
    font-size: 11px;
    color: var(--color-neutral-700);
  }
  .failed .outcome {
    color: var(--color-accent);
    font-weight: 700;
  }
  .details {
    list-style: none;
    margin: 0;
    padding: 0 12px 8px 9px;
    display: flex;
    flex-direction: column;
    gap: 2px;
    font-size: 11px;
    color: var(--color-neutral-800);
  }
  .details .settings,
  .details .dropped {
    color: var(--color-neutral-600);
  }
  .details .bad {
    color: var(--color-accent);
  }
  .empty {
    padding: 12px;
    font-size: 12px;
    color: var(--color-neutral-700);
  }
  .footer {
    padding: 8px 12px;
    font-size: 11px;
    color: var(--color-neutral-600);
  }
</style>
