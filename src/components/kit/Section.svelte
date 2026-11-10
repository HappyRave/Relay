<script lang="ts">
  // A section header that opens and closes its section, with what's in it in one line.
  import Icon from "../ui/Icon.svelte";

  let {
    label,
    summary = "",
    open,
    controls,
    onclick,
  }: { label: string; summary?: string; open: boolean; controls: string; onclick: () => void } = $props();
</script>

<button class="section" type="button" aria-expanded={open} aria-controls={controls} {onclick}>
  <span class="caps name">{label}</span>
  <span class="summary">{summary}</span>
  <span class="chevron" class:open><Icon name="down" size={14} /></span>
</button>

<style>
  .section {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    min-height: 36px;
    padding: 6px 2px;
    border: 0;
    border-bottom: 1px solid var(--line);
    font: inherit;
    text-align: left;
    color: var(--text);
    background: none;
    cursor: pointer;
  }
  .section:hover {
    background: var(--hover);
  }
  .section:focus-visible {
    outline: 2px solid var(--text);
    outline-offset: 1px;
  }
  .name {
    color: var(--text);
  }
  .summary {
    flex: 1;
    min-width: 0;
    font-size: 13px;
    color: var(--text-2);
    text-align: right;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .chevron {
    display: inline-flex;
    transition: transform var(--t-release) var(--spring);
  }
  .open {
    transform: rotate(180deg);
  }
</style>
