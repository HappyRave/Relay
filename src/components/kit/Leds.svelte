<script lang="ts">
  // A row of LED segments: the progress lit, a red segment where each click is.
  let { value, ticks = [], count = 48 }: { value: number; ticks?: number[]; count?: number } = $props();

  const marked = $derived(new Set(ticks.map((t) => Math.round(t * (count - 1)))));
  const lit = $derived(Math.round(Math.min(1, Math.max(0, value)) * (count - 1)));
</script>

<div class="leds" aria-hidden="true" style:--count={count}>
  {#each { length: count }, i (i)}
    <i class:tick={marked.has(i)} class:on={value > 0 && i <= lit}></i>
  {/each}
</div>

<style>
  .leds {
    display: grid;
    grid-template-columns: repeat(var(--count), 1fr);
    gap: 2px;
    height: 5px;
    forced-color-adjust: none;
  }
  i {
    border-radius: 1px;
    background: #2e2e31;
    transition: background 120ms;
  }
  .on {
    background: var(--paper);
  }
  .tick {
    background: #57352e;
  }
  .tick.on {
    background: var(--red);
  }
</style>
