<script lang="ts">
  // The steps list: a listbox, one row per step with cursor moves folded into
  // the step they lead to. Arrows move the selection (and the playhead), Shift
  // and Ctrl select several, Del deletes, Insert or + opens Add at the
  // insertion line (just after the step under the playhead), and Enter goes
  // to the inspector. Hovering or focusing the edge between two rows offers a
  // + to insert right there.
  import { tick } from "svelte";
  import Key from "../kit/Key.svelte";
  import Glyph from "../kit/Glyph.svelte";
  import Menu, { type MenuItem } from "../kit/Menu.svelte";
  import Icon from "../ui/Icon.svelte";
  import { relay, TRIM_PAUSE_MS } from "../../lib/state/relay.svelte";
  import { fmtTime } from "../../lib/format";
  import { category } from "../../lib/categories";
  import { rowMatches, rowOf, rowSteps, rowText, stepRows, type Row } from "../../lib/editor/rows";

  let { onenter }: { onenter?: () => void } = $props();

  const ADD: MenuItem[] = [
    { id: "wait", label: "Wait", icon: "wait" },
    { id: "text", label: "Text", icon: "text" },
    { id: "pixel", label: "Wait for pixel", icon: "pixel" },
    { id: "image", label: "Find image", icon: "image" },
  ];

  const steps = $derived(relay.steps);
  const anchor = $derived(relay.playback.coord_mode === "window" ? (relay.view?.recording.anchor_window?.rect ?? null) : null);
  const rows = $derived(stepRows(steps, relay.settings.show_moves));
  let query = $state("");
  const shown = $derived(
    rows
      .map((row) => ({ row, text: rowText(steps[row.index], row.move == null ? null : steps[row.move], { anchor }) }))
      .filter((r) => rowMatches(r.text, query)),
  );
  const recording = $derived(relay.mode === "recording");
  const playing = $derived(relay.playingHere);
  /** The row of the step under the playhead: the insertion line goes after it (-1: at the top). */
  const here = $derived(rowOf(rows, relay.curStepIdx));
  const inserting = $derived(relay.canEdit && !query);
  const selected = $derived(new Set(relay.selection));
  const cursor = $derived(shown.findIndex((r) => r.row.index === relay.selected));
  const idOf = (r: Row) => `step-${r.index}`;

  let list: HTMLElement | undefined = $state();
  let find: HTMLInputElement | undefined = $state();
  let addKey: HTMLElement | undefined = $state();
  let listKey: HTMLElement | undefined = $state();
  /** Where an open menu hangs from, and which. */
  let menu = $state<{ items: "add" | "list"; opener: HTMLElement } | null>(null);
  /** The row Shift extends the selection from. */
  let from = -1;

  function choose(i: number, how: "only" | "toggle" | "range" = "only") {
    const r = shown[i];
    if (!r) return;
    if (how === "toggle") {
      const now = selected.has(r.row.index) ? relay.selection.filter((s) => s !== r.row.index) : [...relay.selection, r.row.index];
      from = i;
      return relay.select(now, now.includes(r.row.index) ? r.row.index : (now.at(-1) ?? -1));
    }
    if (how === "range" && from >= 0) {
      const [a, b] = from < i ? [from, i] : [i, from];
      return relay.select(
        shown.slice(a, b + 1).map((s) => s.row.index),
        r.row.index,
      );
    }
    from = i;
    relay.select([r.row.index]);
  }

  function click(e: MouseEvent, i: number) {
    const plain = !e.shiftKey && !e.ctrlKey && !e.metaKey;
    choose(i, e.shiftKey ? "range" : plain ? "only" : "toggle");
    if (plain) relay.inspectorOpen = true;
    list?.focus();
  }

  function key(e: KeyboardEvent) {
    const last = shown.length - 1;
    const step = { ArrowDown: 1, ArrowRight: 1, ArrowUp: -1, ArrowLeft: -1 }[e.key];
    if (step || e.key === "Home" || e.key === "End") {
      e.preventDefault();
      const to = step ? Math.max(0, Math.min(last, (cursor < 0 ? (step > 0 ? -1 : last + 1) : cursor) + step)) : e.key === "Home" ? 0 : last;
      if (e.shiftKey && from < 0) from = Math.max(0, cursor);
      choose(to, e.shiftKey ? "range" : "only");
      scrollTo(to);
    } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "a") {
      e.preventDefault();
      relay.select(
        shown.map((s) => s.row.index),
        relay.selected,
      );
    } else if (e.key === "Delete") {
      e.preventDefault();
      if (relay.canEdit) relay.deleteRows(rows.filter((r) => selected.has(r.index)));
    } else if (e.key === "Insert" || e.key === "+") {
      e.preventDefault();
      if (relay.canEdit && addKey) openMenu("add", addKey);
    } else if (e.key === "Enter") {
      e.preventDefault();
      if (relay.selected >= 0) {
        relay.inspectorOpen = true;
        onenter?.();
      }
    } else if (e.key === "Escape" && relay.selection.length > 1) {
      e.preventDefault();
      relay.select([relay.selected]);
    }
  }

  async function scrollTo(i: number) {
    await tick();
    list?.querySelectorAll(".step")[i]?.scrollIntoView({ block: "nearest" });
  }

  function openMenu(items: "add" | "list", at: HTMLElement) {
    menu = { items, opener: at.querySelector("button") ?? at };
  }

  /** The + between two rows: the playhead goes to the row above, so what's added comes right after it. */
  function insertAfter(e: MouseEvent, r: Row) {
    relay.seek(steps[r.index].t);
    openMenu("add", e.currentTarget as HTMLElement);
  }

  function add(id: string) {
    if (id === "wait") relay.insertWait();
    else if (id === "text") relay.insertText();
    else if (id === "pixel") relay.insertPixelCheck();
    else if (id === "image") relay.insertFindImage();
    else if (id === "moves") relay.updateSettings({ show_moves: !relay.settings.show_moves });
    else if (id === "shorten") relay.trimPauses();
  }

  const LIST = $derived<MenuItem[]>([
    { id: "moves", label: "Show moves", checked: relay.settings.show_moves },
    { id: "shorten", label: "Shorten pauses", disabled: !relay.canEdit || relay.longPauses === 0 },
  ]);

  // Ctrl + F finds a step.
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

  // The step playing stays in view.
  $effect(() => {
    if (relay.mode !== "playing" || here < 0) return;
    const i = shown.findIndex((s) => s.row === rows[here]);
    if (i >= 0) list?.querySelectorAll(".step")[i]?.scrollIntoView({ block: "nearest" });
  });
</script>

<section class="plate steps" aria-labelledby="steps-label">
  <div class="head">
    <span class="caps" id="steps-label">Steps <span class="num">{rows.length}</span></span>
    <span class="find">
      <Icon name="search" size={14} />
      <input class="field" type="search" placeholder="Find a step" aria-label="Find a step" aria-keyshortcuts="Control+F" bind:this={find} bind:value={query} />
    </span>
    <span bind:this={addKey}>
      <Key size="sm" aria-haspopup="menu" title="Add a step at the insertion line (Insert)" disabled={!relay.canEdit || !!relay.imaging} onclick={() => addKey && openMenu("add", addKey)}>
        <Icon name="plus" size={14} />Add
      </Key>
    </span>
    <span bind:this={listKey}>
      <Key variant="ghost" size="sm" iconOnly aria-label="Steps options" title="Steps options" aria-haspopup="menu" onclick={() => listKey && openMenu("list", listKey)}>
        <Icon name="more" />
      </Key>
    </span>
  </div>
  {#if relay.imaging?.index === -1}
    <div class="notice" role="status">
      {#if relay.imaging.source === "snip"}
        <span>Snip the image to find, or</span>
        <Key size="sm" onclick={() => relay.insertFindImage("paste")}>Paste</Key>
        <Key size="sm" onclick={() => relay.insertFindImage("file")}>File…</Key>
        <Key size="sm" variant="ghost" onclick={relay.cancelImage}>Cancel</Key>
      {:else}
        <span>{relay.imaging.source === "paste" ? "Reading the clipboard…" : "Choose an image file…"}</span>
      {/if}
    </div>
  {/if}
  <!-- svelte-ignore a11y_no_noninteractive_tabindex: a listbox takes the focus, its options don't -->
  <div
    class="list"
    role="listbox"
    aria-multiselectable="true"
    aria-labelledby="steps-label"
    tabindex="0"
    aria-activedescendant={cursor >= 0 ? idOf(shown[cursor].row) : undefined}
    bind:this={list}
    onkeydown={key}
  >
    {#if inserting && here < 0}<div class="insert" aria-hidden="true">+ insert here</div>{/if}
    {#each shown as { row, text }, i (row.index + ":" + (row.move ?? ""))}
      {@const s = steps[row.index]}
      {@const first = steps[rowSteps(row)[0]]}
      {@const failed = relay.failedStep >= 0 && rowSteps(row).includes(relay.failedStep)}
      {#if first.pause > TRIM_PAUSE_MS && !query && !recording}
        <div class="pause" aria-hidden="true">{(first.pause / 1000).toFixed(1)} s pause</div>
      {/if}
      <!-- svelte-ignore a11y_click_events_have_key_events: the listbox has the focus and the keys -->
      <div
        class="step {category(s.kind)}"
        class:cursor={row.index === relay.selected}
        class:played={playing && s.t <= relay.cur}
        class:now={relay.mode === "playing" && relay.playingHere && rows[here] === row}
        class:failed
        id={idOf(row)}
        role="option"
        tabindex="-1"
        aria-selected={selected.has(row.index)}
        aria-label="{text.kind}, {text.what}{failed ? ', timed out in the last run' : ''}"
        onclick={(e) => click(e, i)}
      >
        <span class="t num">{fmtTime(s.t)}</span>
        <Glyph category={category(s.kind)} />
        <span class="what"><b>{text.kind}</b> <span>· {text.what}</span></span>
        <span class="aside num">{failed ? "Timed out" : text.aside}</span>
      </div>
      {#if inserting && rows[here] === row}
        <div class="insert" aria-hidden="true">+ insert here</div>
      {:else if inserting && i < shown.length - 1}
        <div class="gap">
          <button type="button" tabindex="-1" aria-label="Insert after this step" title="Insert here" onclick={(e) => insertAfter(e, row)}>
            <Icon name="plus" size={12} />
          </button>
        </div>
      {/if}
    {:else}
      <div class="empty">
        {#if query}<b>No steps match “{query}”</b>{:else}<b>No steps yet</b><span>Record to add some, or add one.</span>{/if}
      </div>
    {/each}
  </div>
</section>
{#if menu}
  <Menu
    items={menu.items === "add" ? ADD : LIST}
    opener={menu.opener}
    label={menu.items === "add" ? "Add a step" : "Steps options"}
    onselect={add}
    onclose={() => (menu = null)}
  />
{/if}

<style>
  .steps {
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
  }
  .head {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 10px 8px var(--pad);
  }
  .find {
    position: relative;
    flex: 1;
    min-width: 80px;
    display: flex;
  }
  .find :global(svg) {
    position: absolute;
    left: 9px;
    top: 50%;
    transform: translateY(-50%);
    color: var(--text-2);
    pointer-events: none;
  }
  .find input {
    width: 100%;
    height: var(--key-sm);
    padding-left: 30px;
    font-size: 13px;
  }
  .notice {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0 10px 8px;
    padding: 6px 10px;
    border-radius: var(--r-md);
    font-size: 13px;
    background: var(--plate-2);
  }
  .list {
    flex: 1;
    min-height: 0;
    overflow: auto;
    padding: 0 6px 6px;
  }
  .list:focus-visible {
    outline: 2px solid var(--text);
    outline-offset: -2px;
    border-radius: var(--r-md);
  }
  .step {
    position: relative;
    display: grid;
    grid-template-columns: 64px 20px minmax(0, 1fr) auto;
    align-items: center;
    gap: 10px;
    height: var(--row-h);
    padding: 0 8px;
    border-radius: var(--r-md);
    user-select: none;
  }
  .step:hover {
    background: var(--hover);
  }
  .step > * {
    position: relative;
  }
  .t,
  .aside {
    font-size: 12px;
    color: var(--text-2);
    white-space: nowrap;
  }
  .what {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .what b {
    font-weight: 700;
  }
  .what span {
    color: var(--text-2);
  }
  .step.move .what b {
    font-weight: 600;
  }
  .step[aria-selected="true"] {
    background: var(--plate-2);
    box-shadow: inset 0 0 0 1.5px var(--text);
  }
  .step.cursor:not([aria-selected="true"]) {
    box-shadow: inset 0 0 0 1px var(--line-strong);
  }
  /* While playing, each step's color washes in from the left once it's played. */
  .step::before {
    content: "";
    position: absolute;
    inset: 0 auto 0 0;
    width: 0;
    border-radius: var(--r-md);
    transition: width var(--t-wash) var(--ease);
  }
  .step.pointer::before,
  .step.move::before {
    background: var(--wash-pointer);
  }
  .step.keys::before {
    background: var(--wash-keys);
  }
  .step.logic::before {
    background: var(--wash-logic);
  }
  .step.played::before {
    width: 100%;
  }
  .step.now {
    box-shadow: inset 0 0 0 1.5px var(--red);
  }
  .step.failed {
    background: color-mix(in srgb, var(--red) 10%, transparent);
    box-shadow: inset 0 0 0 1.5px var(--red);
  }
  .step.failed .aside {
    font-weight: 700;
    color: var(--red-text);
  }
  .insert {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 22px;
    margin: 0 8px;
    font-size: 12px;
    font-weight: 600;
    color: var(--text-2);
  }
  .insert::before,
  .insert::after {
    content: "";
    flex: 1;
    border-top: 1.5px dashed var(--line-strong);
  }
  .gap {
    position: relative;
    height: 0;
    z-index: 2;
  }
  .gap button {
    position: absolute;
    left: 2px;
    top: 0;
    width: 24px;
    height: 24px;
    display: flex;
    align-items: center;
    justify-content: center;
    border: 0;
    border-radius: 50%;
    color: var(--text-inv);
    background: var(--text);
    opacity: 0;
    pointer-events: none;
    transform: translateY(-50%) scale(0.6);
    transition:
      opacity 120ms,
      transform var(--t-release) var(--spring);
    cursor: pointer;
  }
  .step:hover + .gap button,
  .gap:hover button,
  .gap button:focus-visible {
    opacity: 1;
    pointer-events: auto;
    transform: translateY(-50%) scale(1);
  }
  .pause {
    display: flex;
    align-items: center;
    height: 18px;
    margin: 1px 8px;
    padding-left: 84px;
    border-radius: var(--r-xs);
    font-size: 12px;
    font-weight: 600;
    color: var(--text-2);
    background: repeating-linear-gradient(-45deg, transparent 0 5px, var(--line) 5px 6px);
  }
  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    padding: 40px 16px;
    text-align: center;
    color: var(--text-2);
  }
  .empty b {
    font-size: 15px;
    color: var(--text);
  }
</style>
