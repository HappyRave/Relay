// Helpers for store and component tests.
import { tick } from "svelte";
import { vi } from "vitest";
import { core } from "./fake-core";
import { resetRelay, type RelayStore } from "../lib/state/relay.svelte";
import { tauriBackend, type Backend } from "../lib/ipc/backend";

export { core };

/**
 * A fresh store on a fresh fake core; `init` loads it like the app does at
 * startup (subscribe, settings, library, first macro, autostart).
 */
export async function freshStore({ init = true, backend = tauriBackend }: { init?: boolean; backend?: Backend } = {}): Promise<RelayStore> {
  core.reset();
  const relay = resetRelay(backend);
  if (init) {
    await relay.init();
    await settle();
    core.clearCalls();
  }
  return relay;
}

/** Lets pending IPC responses and Svelte updates run (works with fake timers too). */
export async function settle(rounds = 5) {
  for (let i = 0; i < rounds; i++) {
    if (vi.isFakeTimers()) await vi.advanceTimersByTimeAsync(0);
    else await new Promise((r) => setTimeout(r, 0));
    await tick();
  }
}

/** Runs the next animation frame (seeks are sent once per frame). */
export async function nextFrame() {
  await new Promise((r) => requestAnimationFrame(() => r(null)));
  await settle();
}
