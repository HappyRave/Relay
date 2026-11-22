<script lang="ts">
  // The Library's macros: a row each, with its hotkey in a column of its own,
  // what else runs it (a link to its triggers) and how its last run went.
  // Search, sort, select several to export or delete, and a menu per row.
  // In a row, the arrow keys go to the next or previous one, Del deletes and
  // Ctrl + D duplicates; Enter on the name opens it.
  import Key from "../kit/Key.svelte";
  import Choice from "../kit/Choice.svelte";
  import Checkbox from "../kit/Checkbox.svelte";
  import Keycaps from "../kit/Keycaps.svelte";
  import Menu, { type MenuItem } from "../kit/Menu.svelte";
  import Icon from "../ui/Icon.svelte";
  import { relay } from "../../lib/state/relay.svelte";
  import { alsoText } from "../../lib/editor/triggers";
  import { fmtLastRun, plural } from "../../lib/format";
  import { failed, outcomeLabel } from "../../lib/runs";
  import type { MacroListItem } from "../../lib/types";

  type Sort = "name" | "newest" | "last_run";
  /** Recently run: in the last week. */
  const RECENT_MS = 7 * 86_400_000;

  let query = $state("");
  let sort = $state<Sort>("name");
  let find: HTMLInputElement | undefined = $state();
  let table: HTMLElement | undefined = $state();

  const recent = (m: MacroListItem) => !!m.last_run && Date.now() - Date.parse(m.last_run) < RECENT_MS;
  const FILTERS = { all: () => true, triggers: (m: MacroListItem) => !!m.hotkey || m.also.length > 0, recent };
  const rows = $derived.by(() => {
    const q = query.trim().toLowerCase();
    const list = relay.library.filter(FILTERS[relay.libFilter]).filter((m) => m.name.toLowerCase().includes(q));
    const by: Record<Sort, (a: MacroListItem, b: MacroListItem) => number> = {
      name: (a, b) => a.name.localeCompare(b.name),
      newest: (a, b) => Date.parse(b.created_at) - Date.parse(a.created_at),
      last_run: (a, b) => (b.last_run ? Date.parse(b.last_run) : 0) - (a.last_run ? Date.parse(a.last_run) : 0),
    };
    return [...list].sort(by[sort]);
  });
  const chosen = $derived(new Set(relay.libSelection));
  const idle = $derived(relay.mode === "idle");
  const editable = $derived(relay.editable && idle);

  function check(id: string, on: boolean) {
    relay.libSelection = on ? [...relay.libSelection, id] : relay.libSelection.filter((s) => s !== id);
  }
  const allChecked = $derived(rows.length > 0 && rows.every((m) => chosen.has(m.id)));

  /** The row's macro, or the selected ones when it's one of them. */
  const targets = (id: string) => (chosen.has(id) ? relay.libSelection : [id]);

  let menu = $state<{ id: string; opener: HTMLElement } | null>(null);
  const MENU = $derived<MenuItem[]>([
    { id: "open", label: "Open", icon: "next" },
    { id: "play", label: "Play", icon: "play", disabled: !idle },
    { id: "duplicate", label: "Duplicate", icon: "duplicate", shortcut: "Ctrl + D", disabled: !editable },
    { id: "export", label: "Export…", icon: "export", disabled: !editable },
    { id: "runs", label: "Runs", icon: "history" },
    "separator",
    { id: "delete", label: "Delete", icon: "trash", shortcut: "Del", danger: true, disabled: !editable },
  ]);

  async function choose(action: string, id: string) {
    if (action === "open") await relay.openMacro(id);
    else if (action === "play") await relay.playMacro(id);
    else if (action === "duplicate") await relay.duplicateMacro(id);
    else if (action === "export") {
      await relay.loadMacro(id);
      if (relay.view?.id === id) relay.exportOpen = true;
    } else if (action === "runs") await relay.showRunsOf(id);
    else if (action === "delete") await relay.deleteMacros(targets(id));
  }

  function rowKey(e: KeyboardEvent, i: number, id: string) {
    const to = { ArrowDown: i + 1, ArrowUp: i - 1 }[e.key];
    if (to !== undefined) {
      e.preventDefault();
      table?.querySelectorAll<HTMLElement>(".mname")[Math.max(0, Math.min(rows.length - 1, to))]?.focus();
    } else if (e.key === "Delete" && editable) {
      e.preventDefault();
      relay.deleteMacros(targets(id));
    } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "d" && editable) {
      e.preventDefault();
      relay.duplicateMacro(id);
    }
  }

  // Ctrl + F searches.
  $effect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && !e.shiftKey && !e.altKey && e.key.toLowerCase() === "f") {
        e.preventDefault();
        find?.focus();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });
</script>

<div class="tools">
  <span class="search">
    <Icon name="search" size={14} />
    <input class="field" type="search" placeholder="Search macros" aria-label="Search macros" aria-keyshortcuts="Control+F" bind:this={find} bind:value={query} />
  </span>
  <Choice
    label="Sort by"
    options={[
      ["name", "Name"],
      ["newest", "Newest"],
      ["last_run", "Last run"],
    ] as [Sort, string][]}
    value={sort}
    onchange={(v) => (sort = v)}
  />
</div>

<div class="rows">
  {#if !relay.library.length}
    <div class="empty">
      <b>Record what you do once. Relay plays it back.</b>
      <span>Press Record, do the task, and press it again: the steps are yours to edit.</span>
      <Key variant="red" disabled={relay.playing} onclick={relay.toggleRec}><Icon name="record" size={14} />Record</Key>
    </div>
  {:else if !rows.length}
    <div class="empty">
      <b>{query ? `No macros match “${query}”` : "None here yet"}</b>
      <span>{query ? "Try another word, or clear the search." : "Macros show here once they have triggers or have run this week."}</span>
    </div>
  {:else}
    <table aria-label="Macros" bind:this={table}>
      <thead>
        <tr>
          <th class="pick"><Checkbox label="Select all" checked={allChecked} onchange={(on) => (relay.libSelection = on ? rows.map((m) => m.id) : [])} /></th>
          <th class="caps">Name</th>
          <th class="caps">Hotkey</th>
          <th class="caps">Also runs</th>
          <th class="caps">Last run</th>
          <th><span class="sr-only">More</span></th>
        </tr>
      </thead>
      <tbody>
        {#each rows as m, i (m.id)}
          {@const last = relay.lastRuns.get(m.id)}
          {@const also = alsoText(m.also)}
          <!-- svelte-ignore a11y_no_noninteractive_element_interactions: the row's keys act on its macro -->
          <tr class:selected={chosen.has(m.id)} class:open={m.id === relay.view?.id} onkeydown={(e) => rowKey(e, i, m.id)}>
            <td class="pick"><Checkbox label="Select {m.name}" checked={chosen.has(m.id)} onchange={(on) => check(m.id, on)} /></td>
            <td>
              <button type="button" class="mname" onclick={() => relay.openMacro(m.id)}>
                <b>{m.id === relay.view?.id ? relay.name : m.name}</b>
                <span class="num">{(m.duration / 1000).toFixed(1)} s · {plural(m.step_count, "step")} · {plural(m.runs, "run")}</span>
              </button>
            </td>
            <td>{#if m.hotkey}<Keycaps combo={m.hotkey.split(" + ")} />{/if}</td>
            <td class="also">
              {#if also}
                <button type="button" class="linkish" aria-label="{also}: open its triggers" onclick={() => relay.openTriggers(m.id)}>{also}</button>
              {/if}
            </td>
            <td>
              {#if !m.last_run}
                <span class="never">Never</span>
              {:else if last && failed(last)}
                <span class="bad" title={outcomeLabel(last)}><Icon name="error" size={14} />{fmtLastRun(m.last_run)}</span>
              {:else}
                <span class="ok" title={last ? outcomeLabel(last) : undefined}><Icon name="check" size={14} />{fmtLastRun(m.last_run)}</span>
              {/if}
            </td>
            <td class="more">
              <Key
                variant="ghost"
                size="sm"
                iconOnly
                aria-label="More for {m.name}"
                aria-haspopup="menu"
                onclick={(e: MouseEvent) => (menu = { id: m.id, opener: e.currentTarget as HTMLElement })}><Icon name="more" /></Key
              >
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</div>

{#if relay.libSelection.length}
  <div class="bulk" role="group" aria-label="Selected macros">
    <b>{plural(relay.libSelection.length, "macro")} selected</b>
    <span class="spacer"></span>
    <Key size="sm" disabled={!editable} onclick={() => relay.exportMacros(relay.libSelection)}><Icon name="export" size={14} />Export…</Key>
    <Key size="sm" variant="danger" disabled={!editable} onclick={() => relay.deleteMacros(relay.libSelection)}><Icon name="trash" size={14} />Delete</Key>
    <Key size="sm" variant="ghost" onclick={() => (relay.libSelection = [])}>Clear</Key>
  </div>
{/if}

{#if menu}
  {@const id = menu.id}
  <Menu items={MENU} opener={menu.opener} label="More for {relay.library.find((m) => m.id === id)?.name ?? 'the macro'}" onselect={(a) => choose(a, id)} onclose={() => (menu = null)} />
{/if}

<style>
  .tools {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px var(--pad) 8px;
  }
  .search {
    position: relative;
    flex: 1;
    display: flex;
  }
  .search :global(svg) {
    position: absolute;
    left: 9px;
    top: 50%;
    transform: translateY(-50%);
    color: var(--text-2);
    pointer-events: none;
  }
  .search input {
    width: 100%;
    height: var(--key-sm);
    padding-left: 30px;
    font-size: 13px;
  }
  .rows {
    flex: 1;
    min-height: 0;
    overflow: auto;
    padding: 0 6px 6px;
  }
  table {
    width: 100%;
    border-collapse: separate;
    border-spacing: 0;
  }
  th {
    position: sticky;
    top: 0;
    z-index: 1;
    padding: 6px 8px;
    text-align: left;
    background: var(--plate);
  }
  td {
    padding: 6px 8px;
    vertical-align: middle;
    border-top: 1px solid var(--line);
  }
  tr.selected td {
    background: var(--plate-2);
  }
  tr.open td:nth-child(2) {
    box-shadow: inset 3px 0 0 var(--red);
  }
  .pick {
    width: 32px;
  }
  .more {
    width: 40px;
  }
  .mname {
    display: block;
    padding: 2px 4px;
    margin-left: -4px;
    border: 0;
    border-radius: var(--r-sm);
    font: inherit;
    text-align: left;
    color: var(--text);
    background: none;
    cursor: pointer;
  }
  .mname:hover b {
    text-decoration: underline;
    text-underline-offset: 2px;
  }
  .mname b {
    display: block;
    font-size: 14px;
  }
  .mname span {
    font-size: 12px;
    color: var(--text-2);
  }
  .also {
    max-width: 220px;
    font-size: 13px;
    color: var(--text-2);
  }
  .linkish {
    padding: 3px 0;
    min-height: 24px;
    border: 0;
    font: inherit;
    text-align: left;
    color: inherit;
    background: none;
    cursor: pointer;
    text-decoration: underline;
    text-decoration-color: var(--line-strong);
    text-underline-offset: 3px;
  }
  .linkish:hover {
    text-decoration-color: currentColor;
  }
  .ok,
  .bad {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 13px;
    white-space: nowrap;
  }
  .ok {
    font-weight: 600;
    color: var(--green-text);
  }
  .bad {
    font-weight: 700;
    color: var(--red-text);
  }
  .never {
    font-size: 13px;
    color: var(--text-2);
  }
  .bulk {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px var(--pad);
    border-top: 1px solid var(--line);
  }
  .spacer {
    flex: 1;
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
