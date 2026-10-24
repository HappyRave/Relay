<script lang="ts" module>
  import type { IconName } from "../ui/Icon.svelte";

  export type MenuItem =
    | { id: string; label: string; icon?: IconName; shortcut?: string; danger?: boolean; disabled?: boolean; checked?: boolean }
    | "separator";
</script>

<script lang="ts">
  // A menu under (or above) the button that opened it. It opens on its first item;
  // the arrow keys, Home and End move, Enter chooses, Esc and Tab close it and give
  // the focus back to that button, and so does a click outside.
  import { onMount } from "svelte";
  import Icon from "../ui/Icon.svelte";

  let {
    items,
    opener,
    label,
    onselect,
    onclose,
  }: { items: MenuItem[]; opener: HTMLElement; label: string; onselect: (id: string) => void; onclose: () => void } = $props();

  let menu: HTMLElement | undefined = $state();
  let pos = $state({ left: 0, top: 0 });

  const enabled = () => [...(menu?.querySelectorAll<HTMLButtonElement>("[role^=menuitem]:not(:disabled)") ?? [])];

  function close(refocus = true) {
    // The opener may go with the menu (its parent forgets both).
    const back = opener;
    onclose();
    if (refocus) back.focus();
  }

  onMount(() => {
    const r = opener.getBoundingClientRect();
    const m = menu!.getBoundingClientRect();
    const below = r.bottom + 6 + m.height <= innerHeight;
    pos = { left: Math.max(8, Math.min(innerWidth - m.width - 8, r.left)), top: below ? r.bottom + 6 : Math.max(8, r.top - 6 - m.height) };
    enabled()[0]?.focus();
    const outside = (e: PointerEvent) => {
      const t = e.target as Node;
      if (!menu?.contains(t) && !opener.contains(t)) close(false);
    };
    document.addEventListener("pointerdown", outside, true);
    return () => document.removeEventListener("pointerdown", outside, true);
  });

  function key(e: KeyboardEvent) {
    const list = enabled();
    const i = list.indexOf(document.activeElement as HTMLButtonElement);
    const to = { ArrowDown: i + 1, ArrowUp: i - 1, Home: 0, End: list.length - 1 }[e.key];
    if (to !== undefined) {
      e.preventDefault();
      list[(to + list.length) % list.length]?.focus();
    } else if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      close();
    } else if (e.key === "Tab") {
      e.preventDefault();
      close();
    }
  }

  function choose(id: string) {
    onselect(id);
    close();
  }
</script>

<!-- svelte-ignore a11y_interactive_supports_focus: the items take the focus -->
<div class="menu" role="menu" aria-label={label} bind:this={menu} style:left="{pos.left}px" style:top="{pos.top}px" onkeydown={key}>
  {#each items as item, i (i)}
    {#if item === "separator"}
      <hr />
    {:else}
      <button
        type="button"
        role={item.checked === undefined ? "menuitem" : "menuitemcheckbox"}
        aria-checked={item.checked}
        tabindex="-1"
        class:danger={item.danger}
        disabled={item.disabled}
        onclick={() => choose(item.id)}
      >
        <span class="ic">{#if item.icon}<Icon name={item.icon} />{:else if item.checked}<Icon name="check" />{/if}</span>
        <span class="text">{item.label}</span>
        {#if item.shortcut}<span class="sc">{item.shortcut}</span>{/if}
      </button>
    {/if}
  {/each}
</div>

<style>
  .menu {
    position: fixed;
    z-index: 30;
    min-width: 200px;
    padding: 4px;
    border-radius: var(--r-md);
    background: var(--float);
    -webkit-backdrop-filter: blur(24px) saturate(1.5);
    backdrop-filter: blur(24px) saturate(1.5);
    box-shadow: var(--shadow-pop);
    animation: pop 140ms var(--ease);
  }
  @keyframes pop {
    from {
      opacity: 0;
      transform: translateY(-4px) scale(0.98);
    }
  }
  button {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    height: 32px;
    padding: 0 10px;
    border: 0;
    border-radius: var(--r-sm);
    font: inherit;
    font-weight: 600;
    text-align: left;
    white-space: nowrap;
    color: var(--text);
    background: none;
    cursor: pointer;
  }
  button:hover:not(:disabled),
  button:focus-visible {
    background: var(--plate-2);
    outline: none;
  }
  button:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }
  .ic {
    display: inline-flex;
    width: 16px;
  }
  .text {
    flex: 1;
  }
  .sc {
    font-size: 12px;
    font-weight: 500;
    color: var(--text-2);
  }
  .danger {
    color: var(--red-text);
  }
  hr {
    border: 0;
    border-top: 1px solid var(--line);
    margin: 4px 6px;
  }
  @media (forced-colors: active) {
    .menu {
      background: Canvas;
      border: 1px solid CanvasText;
    }
  }
</style>
