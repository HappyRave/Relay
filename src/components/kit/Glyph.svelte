<script lang="ts">
  // A step's category as a color block with a shape, so it reads without color too.
  import type { Category } from "../../lib/categories";

  let { category, small = false }: { category: Category; small?: boolean } = $props();
</script>

<span class="glyph {category}" class:small aria-hidden="true"><i></i></span>

<style>
  .glyph {
    width: 20px;
    height: 20px;
    border-radius: var(--r-xs);
    flex: none;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    forced-color-adjust: none;
  }
  .small {
    width: 14px;
    height: 14px;
    border-radius: 3px;
  }
  .small i {
    transform: scale(0.72);
  }
  i {
    display: block;
  }
  .pointer {
    background: var(--blue);
    color: #fff;
  }
  .keys {
    background: var(--yellow);
    color: var(--ink);
  }
  .logic {
    background: var(--green);
    color: var(--ink);
  }
  .move {
    background: var(--blue-soft);
    color: var(--blue);
  }
  :global([data-theme="dark"]) .move {
    color: var(--blue-line);
  }
  .pointer i,
  .move i {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: currentColor;
  }
  .move i {
    background: none;
    border: 1.6px solid currentColor;
  }
  .keys i {
    width: 8px;
    height: 8px;
    border-radius: 1px;
    background: currentColor;
  }
  .logic i {
    width: 0;
    height: 0;
    border-left: 5px solid transparent;
    border-right: 5px solid transparent;
    border-bottom: 8px solid currentColor;
  }
</style>
