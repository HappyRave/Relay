<script lang="ts">
  import Icon from "../ui/Icon.svelte";
  import { isTauri, startDragging } from "../../lib/platform/window";
  import { devDesktop } from "../../lib/dev/devDesktop.svelte";

  let drag: { x: number; y: number; dx: number; dy: number } | null = null;

  function down(e: PointerEvent) {
    if (e.button !== 0) return;
    e.preventDefault();
    if (isTauri()) {
      startDragging();
      return;
    }
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    drag = { x: e.clientX, y: e.clientY, dx: devDesktop.dx, dy: devDesktop.dy };
  }
  function move(e: PointerEvent) {
    if (!drag) return;
    devDesktop.dx = drag.dx + e.clientX - drag.x;
    devDesktop.dy = drag.dy + e.clientY - drag.y;
  }
</script>

<div class="grip" role="presentation" onpointerdown={down} onpointermove={move} onpointerup={() => (drag = null)}>
  <Icon name="grip" />
</div>

<style>
  .grip {
    width: 28px;
    flex: none;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: grab;
    border-right: 2px solid var(--color-divider);
    color: var(--color-neutral-600);
  }
  .grip:active {
    cursor: grabbing;
  }
</style>
