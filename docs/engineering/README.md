# Engineering guide

How Relay works inside: for contributors, reviewers and anyone curious about building a reliable macro recorder.

> [!TIP]
> New here? Read [Architecture](architecture.md) first. It gives the big picture, the threads and the two main data flows (recording and playback) in about ten minutes. The other pages go deeper into one layer each.

## Reading order

| Page | What it covers |
| --- | --- |
| 1. [Architecture](architecture.md) | The three crates, the threads, how a recording and a playback travel through the system, and the design principles |
| 2. [relay-core](core.md) | The macro model, grouping raw events into steps, edits and their invariants, the file format, playback timing, the session state machine, schedules and trigger edges |
| 3. [relay-platform](platform.md) | The OS traits, low-level hooks, input injection, the precision timer, window and screen queries, the key map and the recorder |
| 4. [The app (src-tauri)](app.md) | The coordinator, the playback engine, the recorder thread and watchdog, hotkeys, the trigger runtime, storage, the window and the tray |
| 5. [IPC](ipc.md) | Every Tauri command, the session stream, generated TypeScript bindings and sequence diagrams |
| 6. [The frontend](frontend.md) | The Svelte 5 store, the backend abstraction, the browser preview, components, styling and playhead extrapolation |
| 7. [File formats](file-formats.md) | `.rly`, the JSON export, `library.json`, `settings.json`, `window.json` and migrations |
| 8. [Testing and CI](testing.md) | Unit tests, property tests, snapshots, frontend tests, end-to-end testing against the real app, CI and releases |

## Quick facts

| | |
| --- | --- |
| **Languages** | Rust (edition 2024, 1.95 or later) and TypeScript |
| **App shell** | [Tauri 2](https://tauri.app/) with WebView2 |
| **UI** | [Svelte 5](https://svelte.dev/) (runes), Vite 8, no UI framework |
| **OS access** | [`windows`](https://crates.io/crates/windows) crate (Win32), behind traits in `relay-platform` |
| **Size** | about 4,500 lines in the two library crates, 3,100 in the app, 4,000 in the UI |
| **Tests** | about 180 Rust tests (including property tests and snapshots), 390 Vitest tests of the store and every component, and 77 end-to-end tests against the real app |
| **Installer** | NSIS, per-user, about 2 MB |

## Getting set up

See [CONTRIBUTING.md](../../CONTRIBUTING.md) for requirements, commands and conventions.

---

<p align="center"><a href="../README.md">Documentation home</a> · <a href="architecture.md">Architecture →</a></p>
