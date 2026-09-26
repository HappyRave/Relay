// Runs before every test file: the DOM matchers, what jsdom lacks, and the
// fake Rust side behind Tauri's IPC mock (installed before any app module
// loads, so the app's backend is the real `tauriBackend`).
import "@testing-library/jest-dom/vitest";
import { afterEach, vi } from "vitest";
import { core } from "./fake-core";

class NoResizeObserver {
  observe() {}
  unobserve() {}
  disconnect() {}
}
globalThis.ResizeObserver ??= NoResizeObserver as unknown as typeof ResizeObserver;
Element.prototype.scrollIntoView ??= function () {};
Element.prototype.setPointerCapture ??= function () {};
Element.prototype.releasePointerCapture ??= function () {};
Element.prototype.hasPointerCapture ??= () => false;

// Frames at 60 Hz, stamped with performance.now() as browsers do (jsdom's
// own frames use another time origin, which breaks playhead extrapolation).
let frames = 0;
const pending = new Map<number, ReturnType<typeof setTimeout>>();
window.requestAnimationFrame = (cb) => {
  const id = ++frames;
  pending.set(
    id,
    setTimeout(() => {
      pending.delete(id);
      cb(performance.now());
    }, 16),
  );
  return id;
};
window.cancelAnimationFrame = (id) => {
  clearTimeout(pending.get(id));
  pending.delete(id);
};

// jsdom has <dialog> but not its modal behavior.
const dialog = HTMLDialogElement.prototype;
dialog.showModal ??= function (this: HTMLDialogElement) {
  this.setAttribute("open", "");
};
dialog.close ??= function (this: HTMLDialogElement) {
  if (!this.hasAttribute("open")) return;
  this.removeAttribute("open");
  this.dispatchEvent(new Event("close"));
};

core.install();

afterEach(() => {
  vi.useRealTimers();
});
