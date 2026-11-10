// matchMedia for jsdom, which has none: no query matches until a test says so
// with `setMedia`, and the lists hear the change as a browser's would.
type Listener = (e: MediaQueryListEvent) => void;

const matching = new Set<string>();
const lists = new Map<string, Set<Listener>>();

export function setMedia(query: string, matches: boolean) {
  if (matching.has(query) === matches) return;
  if (matches) matching.add(query);
  else matching.delete(query);
  for (const on of lists.get(query) ?? []) on({ matches, media: query } as MediaQueryListEvent);
}

export function resetMedia() {
  matching.clear();
}

export function installMedia() {
  window.matchMedia = (query: string) => {
    const listeners = lists.get(query) ?? new Set<Listener>();
    lists.set(query, listeners);
    return {
      media: query,
      get matches() {
        return matching.has(query);
      },
      addEventListener: (_: string, on: Listener) => listeners.add(on),
      removeEventListener: (_: string, on: Listener) => listeners.delete(on),
    } as unknown as MediaQueryList;
  };
}
