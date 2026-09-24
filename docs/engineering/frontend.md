# The frontend

The UI in [`src/`](../../src) is Svelte 5 with runes, TypeScript and Vite 8, with no component library. It renders state and sends intentions. It doesn't own any session logic: the Rust coordinator does.

- [Structure](#structure)
- [The store](#the-store)
- [The backend abstraction](#the-backend-abstraction)
- [The browser preview](#the-browser-preview)
- [Components](#components)
- [The preview and the timeline](#the-preview-and-the-timeline)
- [Styling](#styling)
- [The window from the UI's side](#the-window-from-the-uis-side)
- [Conventions](#conventions)

## Structure

```text
src/
├── main.ts                     mounts App
├── App.svelte                  the Widget in Tauri, the DevDesktop in a browser, the Export dialog
├── components/
│   ├── Widget.svelte           measures itself and calls fit_window
│   ├── CompactBar.svelte       the 604 × 68 player bar
│   ├── ExpandedWidget.svelte   header, preview, side panel, transport, timeline
│   ├── ExportDialog.svelte
│   ├── expanded/               Header, Preview, SidePanel, Transport, Timeline
│   │   └── tabs/               StepsTab, StepEditor, LibraryTab, TriggersTab, SettingsTab
│   ├── shared/                 Grip, RecPlayButtons
│   ├── ui/                     Toggle, Segmented, Kbd, Icon, HotkeyCapture
│   └── dev/DevDesktop.svelte   the demo desktop for the browser preview
├── lib/
│   ├── state/relay.svelte.ts   the RelayStore
│   ├── state/display.ts        badge labels
│   ├── ipc/backend.ts          Tauri and browser backends
│   ├── ipc/bindings/           generated from Rust (don't edit)
│   ├── types.ts                re-exports of the bindings, UI-only types
│   ├── format.ts               time, names, "Next run", "Last run"
│   ├── hotkeys.ts              KeyboardEvent → "Ctrl + Alt + 1"
│   ├── timeline/lanes.ts       lane geometry, current step, prev/next
│   ├── preview/geometry.ts     SVG path, cursor lookup, fitting the view
│   ├── actions/seekable.ts     click-and-drag to seek
│   ├── platform/window.ts      isTauri, fit_window, dragging, hide to tray
│   └── dev/                    the browser fixture and demo desktop state
└── styles/                     tokens.css, base.css, components.css, app.css
```

## The store

[`lib/state/relay.svelte.ts`](../../src/lib/state/relay.svelte.ts) exports one `RelayStore` instance, `relay`. Components import it and read its fields directly. Svelte 5's fine-grained reactivity re-renders only what changed.

| Kind | Fields |
| --- | --- |
| **Session** (from the stream) | `mode`, `cur` (the playhead, ms), `countLeft`, `loopIdx`, `triggersPaused` |
| **Data** (from commands) | `library`, `view` (the open `MacroView`), `settings`, `triggerStatus`, `autostart` |
| **UI** | `expanded`, `tab`, `exportOpen`, `exportFmt`, `toast`, `picking` |
| **Derived** | `recording`, `name`, `playback`, `loops`, `steps`, `moves`, `duration`, `desktop`, `frames`, `triggers`, `error`, `exportName`, `canUndo`, `canRedo`, `longPauses` |

- `view` and `library` use `$state.raw`: they're replaced wholesale by command results, never mutated, so deep proxies would be wasted work.
- While recording, `steps`, `moves` and `desktop` switch to the live `recSteps`, `recMoves` and `recDesktop` fed by `rec_progress`.
- **Actions** are arrow-function fields (`toggleRec`, `togglePlay`, `stop`, `seek`, `jump`, `edit`, `rename`, `insertWait`, `insertPixelCheck`, `pickPixel`, `setPause`, `trimPauses`, `undo`, `redo`, `setPlayback`, `setTriggers`, `duplicateMacro`, `deleteMacro`, `restoreMacro`, `importMacros`, `doExport`…), so they can be passed as event handlers without binding.
- **Errors** from any action go to `fail()`, which shows an error toast for 5 s. `notify()` shows information, for 8 s when it has an action such as **Undo**.

Lifecycle: `App.svelte` calls `relay.start()` (the animation-frame loop and the key listener) and `relay.init()` (read the saved window mode, subscribe to the stream, load settings, the library, the first macro and autostart). The widget only renders once `ready` is set, so it never flashes at the wrong size.

The store is exposed as `window.__relay` for DevTools and the end-to-end tests.

### Handling the stream

`onEngine(msg)` is a `switch` over `EngineMsg`. The non-obvious cases:

- **`session`** to `play` with another `macro_id`: a trigger started a different macro, so it's opened (with `showMacro`, which works mid-playback).
- **`saved`** arrives just before the session goes back to idle, when the store still refuses to load a macro. It's kept in `pendingLoad` and opened on the `idle` message.
- **`finished`**: rewind to 0 unless `completed` or `pixel_timeout`.
- **`play_tick`** and **`rec_progress`** update `tick` for [extrapolation](ipc.md#why-the-ui-extrapolates). When not advancing (paused, frozen on a pixel check), `cur` is set exactly.

### Edits

`edit(op)` → `apply(backend.editMacro(id, op))`. `apply` tags each request with an increasing `editSeq` and drops a response if a newer request started, so a slow response can't overwrite a newer state. Loading a macro bumps `editSeq` too. `rename` updates the view immediately and debounces the command by 250 ms.

**Undo and redo** go through `backend.undoEdit`, after first saving a name still being typed so it's part of the history. <kbd>Ctrl</kbd>+<kbd>Z</kbd>, <kbd>Ctrl</kbd>+<kbd>Y</kbd> and <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>Z</kbd> are handled by the store's window key listener, except when the event comes from an `input`, a `textarea` or an element marked `data-captures-keys` (the hotkey field), where those keys mean something else. Deleting a step and trimming pauses show a toast whose action is `undo`.

## The backend abstraction

[`lib/ipc/backend.ts`](../../src/lib/ipc/backend.ts) defines a `Backend` interface covering every command and the stream. It has two implementations, chosen once by `isTauri()` (the presence of `window.__TAURI_INTERNALS__`):

- **`tauriBackend`**: `invoke` for commands, a `Channel<EngineMsg>` for the stream, and `@tauri-apps/plugin-dialog` for file pickers. `editable: true`.
- **`browserBackend()`**: a simulation for `npm run dev`. `editable: false`.

The store and components only see `Backend`. Features that need the app check `relay.editable` and hide or disable their controls.

## The browser preview

`npm run dev` serves the UI at `http://localhost:1420` in any browser, with no Rust build. It's how the design is iterated on quickly.

- `App.svelte` renders `DevDesktop`, the design's demo desktop (wallpaper, taskbar, clock), with the widget scaled to fit the browser window.
- The browser backend loads [`lib/dev/sample-views.json`](../../src/lib/dev/sample-views.json), the four sample macros as real `MacroView`s generated by relay-core's tests. So the steps, paths and timings are exactly what the app would show.
- It simulates the session: the countdown, playback with a moving playhead (speed changes, loops and seeking included), and trigger settings kept in memory. Recording captures nothing, and edits are refused with *Editing needs the Relay app*.
- With no global hotkeys, the store listens for F9, F10, Esc and Ctrl+Shift+M on the page itself.
- Undo, redo and every other edit are unavailable, and their buttons are hidden.
- Export downloads the file with a Blob.

> [!NOTE]
> Port 1420 is also what `npm run tauri dev` uses for its dev server. Stop one before starting the other.

## Components

The component tree mirrors the design file:

```mermaid
flowchart TD
    App --> Widget
    App --> ExportDialog
    App -.browser.-> DevDesktop --> Widget
    Widget --> CompactBar
    Widget --> ExpandedWidget
    ExpandedWidget --> Header
    ExpandedWidget --> Preview
    ExpandedWidget --> SidePanel
    ExpandedWidget --> Transport
    ExpandedWidget --> Timeline
    SidePanel --> StepsTab --> StepEditor
    SidePanel --> LibraryTab
    SidePanel --> TriggersTab --> HotkeyCapture
    SidePanel --> SettingsTab
    CompactBar & Transport --> RecPlayButtons
```

Components are small and mostly presentational. Logic that deserves a test (formatting, timeline geometry, hotkey parsing, preview fitting) lives in plain `.ts` modules next to their `.test.ts`.

A few that do more:

| Component | Notes |
| --- | --- |
| `Widget` | A `ResizeObserver` measures the widget's border box and calls `fit_window`, so the native window always matches the content exactly. |
| `StepsTab` | Keeps the current row in view while playing. Opens `StepEditor` under the selected row, only for kinds that have something to edit. Shows window-relative positions in *Window* mode. |
| `StepEditor` | *Pause before* for every step, plus the fields of its kind. Commits on `change` (blur or Enter), validates hex colors and numbers before sending an `EditOp`. |
| `HotkeyCapture` | Captures in the capture phase and stops propagation, so a combo being set never triggers anything else. Marked `data-captures-keys` so the undo shortcut leaves it alone. Backspace clears, Esc cancels, blur cancels. |
| `Timeline`, `CompactBar` | Use the `seekable` action: pointer down and drag anywhere seeks, with pointer capture. |
| `Preview` | An SVG in desktop coordinates (see below). |

## The preview and the timeline

**Preview** ([`Preview.svelte`](../../src/components/expanded/Preview.svelte), [`geometry.ts`](../../src/lib/preview/geometry.ts)): the SVG's `viewBox` is a region of the virtual desktop, in the macro's physical pixels, chosen by `fitView` to include everything the macro touches, padded and at the preview's 600:338 aspect ratio. The monitors and the anchor window are drawn as outlines. Everything is in desktop coordinates, and a scale factor `k` keeps strokes and labels the same size at any zoom.

- The **path** is one polyline. The played part is the same path with `stroke-dasharray = "<done length> <total>"`, where the done length comes from precomputed cumulative lengths and a binary search for the current time. So animating the red trail costs nothing per frame.
- **Click markers** are numbered in order, with a ring that expands for 500 ms after each click.
- The **key overlay** shows the `KEYS` or `TYPE` step under the playhead, and for typing, only the characters typed so far.

**Timeline** ([`lanes.ts`](../../src/lib/timeline/lanes.ts)): pure functions turn steps and moves into percentages. Mouse movement becomes bars (samples less than 150 ms apart join), `KEYS` chips grow up to the next chip, `TYPE` chips span their characters, and the ruler picks 1 s, 5 s or 15 s ticks from the macro's length. `currentStepIndex` and `jumpTarget` drive the highlighted row and the ◀ ▶ buttons.

## Styling

The look comes from the design system in [`Design/_ds/modernist-…/styles.css`](../../Design): the "Modernist" style, with an off-white ground, near-black ink, one red accent (`#EC3013`), Archivo, 2 px rules and **zero radius** everywhere.

| File | Contents |
| --- | --- |
| [`tokens.css`](../../src/styles/tokens.css) | The design's tokens, copied verbatim: colors and OKLCH tonal ramps, fonts, spacing, radii (all 0), shadows |
| [`base.css`](../../src/styles/base.css) | Element defaults |
| [`components.css`](../../src/styles/components.css) | Shared classes: `.btn` variants, `.input`, `.tag`, dialogs |
| [`app.css`](../../src/styles/app.css) | App rules: no page scroll, no text selection outside inputs |

Components use scoped `<style>` blocks and only `var(--…)` tokens for colors, so the palette lives in one file. Archivo is bundled with `@fontsource/archivo` (no network at runtime).

## The window from the UI's side

[`lib/platform/window.ts`](../../src/lib/platform/window.ts) holds the only window calls: `fitWindow` (after every resize), `savedExpanded` (before the first render), `startDragging` (the grip, via Tauri) and `hideToTray` (the × button). They're no-ops in a browser. Placement, zoom, the saved anchor and focus behavior are all in Rust (see [The app → The window](app.md#the-window)).

## Conventions

- Components read `relay` and call its actions. They never call `backend` or `invoke` directly.
- New IPC types come from Rust bindings, not hand-written interfaces.
- Pure logic goes in `lib/` with a Vitest test. Components stay thin.
- `npm run check` (svelte-check) must pass with no warnings, including accessibility warnings.

---

<p align="center"><a href="ipc.md">← IPC</a> · <a href="README.md">Contents</a> · <a href="file-formats.md">File formats →</a></p>
