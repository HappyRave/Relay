<script lang="ts">
  // Activity: the run history, every run or only the problems (runs that
  // didn't end as planned, and triggers that were skipped). A run opens to
  // what it did; one that stopped on a step opens the macro there.
  import Key from "../kit/Key.svelte";
  import Icon from "../ui/Icon.svelte";
  import { relay } from "../../lib/state/relay.svelte";
  import { checkLabel, failed, fmtDuration, fmtRunTime, outcomeLabel, runSettings, sourceLabel } from "../../lib/runs";
  import { plural } from "../../lib/format";
  import type { RunEntry } from "../../lib/ipc/bindings/RunEntry";

  let open = $state<string | null>(null);
  const key = (r: RunEntry) => `${r.at} ${r.macro_id}`;
  const problems = $derived(relay.libraryShows === "problems");
  /** The step a run stopped on, when it timed out on one and the macro is still there. */
  const stoppedAt = (r: RunEntry) => {
    const last = r.checks.at(-1);
    const there = relay.library.some((m) => m.id === r.macro_id);
    return r.outcome.type === "finished" && r.outcome.reason === "pixel_timeout" && last && there ? last.step : null;
  };
  const sub = (r: RunEntry) =>
    [sourceLabel(r.source), fmtRunTime(r.at), r.rows ? plural(r.rows, "row") : r.loops > 1 ? plural(r.loops, "loop") : ""].filter(Boolean).join(" · ");
</script>

<div class="tools">
  <h2 class="title">{problems ? "Problems" : "All runs"}</h2>
  <select class="field filter" aria-label="Show the runs of" bind:value={relay.runsFilter}>
    <option value={null}>Every macro</option>
    {#each relay.library as m (m.id)}<option value={m.id}>{m.name}</option>{/each}
  </select>
  <span class="spacer"></span>
  <span class="subtle">Relay keeps the last 200 runs.</span>
</div>
<div class="runs">
  {#each relay.shownRuns as r (key(r))}
    {@const expanded = open === key(r)}
    {@const bad = failed(r)}
    {@const step = stoppedAt(r)}
    <div class="run" class:open={expanded}>
      <button class="head" type="button" aria-expanded={expanded} onclick={() => (open = expanded ? null : key(r))}>
        <span class={r.outcome.type === "skipped" ? "skip" : bad ? "bad" : "ok"}>
          <Icon name={r.outcome.type === "skipped" ? "info" : bad ? "error" : "check"} size={16} />
        </span>
        <span class="what"><b>{r.macro_name}</b><span>{sub(r)}</span></span>
        <span class="outcome" class:bad>{outcomeLabel(r)}</span>
        <span class="dur num">{r.outcome.type === "finished" ? fmtDuration(r.duration_ms) : ""}</span>
      </button>
      {#if expanded}
        <div class="detail">
          {#if runSettings(r)}<span>{runSettings(r)}</span>{/if}
          {#if r.checks_dropped > 0}<span class="subtle">{plural(r.checks_dropped, "earlier check")} not kept</span>{/if}
          {#each r.checks as c, i (i)}
            <span class:bad={c.outcome.type === "timed_out"}>{checkLabel(c, r.loops, r.rows)}</span>
          {:else}
            {#if !runSettings(r)}<span class="subtle">Nothing more to say about this run.</span>{/if}
          {/each}
          {#if step}
            <span><Key size="sm" onclick={() => relay.openAt(r.macro_id, step, true)}>Open at step {step}</Key></span>
          {/if}
        </div>
      {/if}
    </div>
  {:else}
    <div class="empty">
      {#if problems}<b>No problems</b><span>Every run ended as planned.</span>{:else}<b>No runs yet</b><span>Runs show here once a macro has played.</span>{/if}
    </div>
  {/each}
</div>

<style>
  .tools {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px var(--pad) 8px;
  }
  .title {
    margin: 0;
    font-size: 17px;
    font-weight: 800;
  }
  .filter {
    width: auto;
    max-width: 220px;
    height: var(--key-sm);
    font-size: 13px;
  }
  .spacer {
    flex: 1;
  }
  .runs {
    flex: 1;
    min-height: 0;
    overflow: auto;
    padding: 0 6px 6px;
  }
  .run {
    border-radius: var(--r-md);
  }
  .run.open {
    background: var(--plate-2);
  }
  .head {
    display: grid;
    grid-template-columns: 20px minmax(0, 1fr) auto 70px;
    gap: 10px;
    align-items: center;
    width: 100%;
    min-height: 50px;
    padding: 6px 10px;
    border: 0;
    border-radius: var(--r-md);
    font: inherit;
    text-align: left;
    color: var(--text);
    background: none;
    cursor: pointer;
  }
  .head:hover {
    background: var(--hover);
  }
  .what {
    min-width: 0;
  }
  .what b {
    display: block;
  }
  .what span {
    font-size: 12px;
    color: var(--text-2);
  }
  .outcome {
    font-size: 13px;
    color: var(--green-text);
  }
  .ok {
    display: flex;
    color: var(--green-text);
  }
  .bad,
  .outcome.bad {
    display: flex;
    font-weight: 700;
    color: var(--red-text);
  }
  .skip {
    display: flex;
    color: var(--text-2);
  }
  .dur {
    font-size: 12px;
    text-align: right;
    color: var(--text-2);
  }
  .detail {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 0 10px 10px 40px;
    font-size: 13px;
  }
  .detail .bad {
    display: inline;
  }
  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    padding: 40px;
    text-align: center;
    color: var(--text-2);
  }
  .empty b {
    font-size: 17px;
    color: var(--text);
  }
</style>
