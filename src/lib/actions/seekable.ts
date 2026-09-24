import { relay } from "../state/relay.svelte";

/** Click or drag horizontally on the element to move the playhead. */
export function seekable(node: HTMLElement) {
  const at = (e: PointerEvent) => {
    const r = node.getBoundingClientRect();
    const f = Math.min(1, Math.max(0, (e.clientX - r.left) / r.width));
    relay.seek(f * relay.duration);
  };
  const down = (e: PointerEvent) => {
    if (e.button !== 0 || relay.recording) return;
    e.preventDefault();
    node.setPointerCapture(e.pointerId);
    at(e);
    node.addEventListener("pointermove", at);
  };
  const up = () => node.removeEventListener("pointermove", at);
  node.addEventListener("pointerdown", down);
  node.addEventListener("pointerup", up);
  node.addEventListener("lostpointercapture", up);
  return {
    destroy() {
      node.removeEventListener("pointerdown", down);
      node.removeEventListener("pointerup", up);
      node.removeEventListener("lostpointercapture", up);
      node.removeEventListener("pointermove", at);
    },
  };
}
