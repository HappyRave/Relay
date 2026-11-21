# Engineering guide

How Relay is built, and why. For contributors and reviewers.

What Relay *does* is the [behavior spec](../spec/README.md); setup and commands are in [CONTRIBUTING.md](../../CONTRIBUTING.md). This guide is the part between: how the pieces fit, the decisions behind them, and where to make a change.

> [!TIP]
> New here? Read [Architecture](architecture.md) first: the crates, the threads, and how a recording, a playback and a trigger travel through them.

## Pages

| Page | Read it for |
| --- | --- |
| [Architecture](architecture.md) | The big picture, and the rules the code follows |
| [relay-core](core.md) | The model, steps, edits, the session state machine, timing |
| [relay-platform](platform.md) | The OS boundary: hooks, injection, the timer, the screen, the picker |
| [The app](app.md) | The coordinator, the engine, hotkeys, triggers, files, windows |
| [IPC](ipc.md) | Commands, the message stream, errors, generated bindings |
| [The frontend](frontend.md) | The store, its guards, the backends, components, styling |
| [File formats](file-formats.md) | Every file Relay writes |
| [Testing, CI and releases](testing.md) | The three test layers, CI, releasing |
| [Recipes](recipes.md) | Adding an edit, a step type, a command, a trigger, a setting, a format version, an OS backend |

## I want to change…

| …this | Start in |
| --- | --- |
| How recorded input becomes steps | [`steps.rs`](../../crates/relay-core/src/steps.rs), [relay-core → Step grouping](core.md#step-grouping) |
| What an edit does | [`edit.rs`](../../crates/relay-core/src/edit.rs), [Recipes → Add an edit](recipes.md#add-an-edit) |
| What happens when Record, Play or Stop is pressed | [`session.rs`](../../crates/relay-core/src/session.rs), then [`coordinator.rs`](../../src-tauri/src/coordinator.rs) |
| How a macro plays | [`relay-playback/src/engine.rs`](../../crates/relay-playback/src/engine.rs), [The app → The playback engine](app.md#the-playback-engine) |
| What's recorded and left out | [`windows/hook.rs`](../../crates/relay-platform/src/windows/hook.rs), [`recorder.rs`](../../crates/relay-platform/src/recorder.rs) |
| When a trigger fires | [`src-tauri/src/triggers.rs`](../../src-tauri/src/triggers.rs), [Recipes → Add a trigger kind](recipes.md#add-a-trigger-kind) |
| Which window shows, and where | [`presence.rs`](../../src-tauri/src/presence.rs), [`window_ctl.rs`](../../src-tauri/src/window_ctl.rs) |
| Relay's keys and hotkeys | [`hotkeys.rs`](../../src-tauri/src/hotkeys.rs); the keys as settings (`RelayKeys`) in [`settings.rs`](../../src-tauri/src/settings.rs) |
| What a screen shows | the component in [`src/components/`](../../src/components), and the store it reads |
| Colors, spacing, motion | [`theme.css`](../../src/styles/theme.css) |
| A macro file's format (`.rly`, exports) | [`format.rs`](../../crates/relay-core/src/format.rs), [File formats](file-formats.md) (which covers the other files too) |

---

<p align="center"><a href="../README.md">Documentation home</a> · <a href="architecture.md">Architecture →</a></p>
