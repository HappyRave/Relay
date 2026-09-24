<script lang="ts">
  let { combo, muted = false }: { combo: string; muted?: boolean } = $props();
</script>

<span class="kbd" class:muted>{combo}</span>

<style>
  .kbd {
    font-size: 11px;
    font-weight: 800;
    padding: 2px 7px;
    border: 1px solid var(--color-text);
    border-bottom-width: 3px;
    white-space: nowrap;
  }
  .muted {
    font-size: 10px;
    padding: 1px 6px;
    border: 1px solid var(--color-neutral-500);
  }
</style>
