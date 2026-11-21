# The app (src-tauri)

`src-tauri` is the `relay` binary. It wires the other crates into a running app: the coordinator and its threads, the files, the commands the UI calls, Relay's keys, the triggers, the two windows and the tray.

- [Startup and shared state](#startup-and-shared-state)
- [The coordinator](#the-coordinator)
- [The playback engine](#the-playback-engine)
- [Recording and the watchdog](#recording-and-the-watchdog)
- [Hotkeys](#hotkeys)
- [Triggers](#triggers)
- [Files](#files)
- [Windows](#windows)
- [Logging and panics](#logging-and-panics)

## Startup and shared state

[`lib.rs`](../../src-tauri/src/lib.rs) registers the plugins (single instance, global shortcut, dialog, autostart), then `setup` opens settings, the library and the run history, spawns the coordinator and the trigger threads, and only then builds both windows (`"create": false` in `tauri.conf.json`). This order means a window's first commands always find the state they read. The windows start hidden and `presence` shows them once they're placed, so they never jump. With `--autostart`, Relay stays in the tray.

Settings open before the library: a folder an older Relay used gets the old keys (see [Settings](#settings)), and the library needs Relay's keys so no macro hotkey takes one.

Shared state is in Tauri's `app.manage`: the library, settings and run history behind `parking_lot` mutexes (which don't poison), the `Emitter`, the coordinator's `Sender<Cmd>`, the shared `SessionMode`, the hotkeys, the edit history, the window and presence state.

> [!IMPORTANT]
> **Never hold a lock while calling into the main thread.** Window, tray and hotkey calls block until the main thread runs them, and the main thread may be waiting for that lock.

On quit (`RunEvent::Exit`), the coordinator gets `Cmd::Shutdown` and stops what's running, releasing held keys and saving a recording in progress, for up to 3 s.

## The coordinator

[`coordinator.rs`](../../src-tauri/src/coordinator.rs) is a thread looping over `Receiver<Cmd>`. Every input goes through [`session::step`](core.md#the-session-state-machine), and the coordinator performs the effects in order. It owns:

- **The session mode.** Every change goes through `set_mode`, which also updates the `SessionMode` commands read, even after a panic.
- **The current macro**, the one the play key and the player play. `set_current` emits `Current` to both windows, so they always show the same macro. A triggered run doesn't announce its macro (`before_trigger` remembers the current one and puts it back), so a trigger never switches what the user has open.
- **The play key's playhead** (`idle_playhead`). The play key is handled in Rust, so Rust mirrors where the UI's playhead lands after each run (`playhead_after`): 0 after a stop or error, the end after a complete run, the check's time after a timeout.
- **Playback generations.** Each playback gets a number, and `EngineDone` carries it. A late message from a stopped playback is ignored. `Finished` is sent once, by the coordinator, however the playback ended.

The pure decisions are pulled out and unit-tested: `admit` (whether a trigger runs), `playhead_after`, `is_current`, `counts_as_run` (only `Completed` counts).

Each command runs under `catch_unwind`, so a bug ends the session cleanly instead of killing the thread. The coordinator is generic over Tauri's `Runtime`, which lets a unit test run the real thread on the mock runtime and check that a panic mid-countdown leaves it idle.

**Starting a playback** is a chain of small helpers in `start_playback`: load the macro; read its data file (`data_file::for_playing`, which may refuse the run); hand the focus back to the app the user was in (`restore_previous`, with a notice if it runs as administrator); apply the window offset; show the player (`presence::update`) and make it click-through if the macro clicks under it; start the stop-key hook; then spawn the engine with the next generation.

## The playback engine

The engine is in [`relay-playback`](../../crates/relay-playback/src/engine.rs), shared with the exported player. It splits the decision from the thread:

- **`Engine::advance(now)`** injects everything due and returns `Some(reason)` when done. Its injector, pixel reader and image finder are traits, so tests drive it with a fake clock, a recording injector and a fake screen.
- **`spawn`** runs it on `relay-engine` with the platform timer, reporting to a `PlaybackSink`. A guard reports the end even on a panic. Dropping the handle stops it.
- **The app's sink** ([`engine.rs`](../../src-tauri/src/engine.rs)) turns ticks and notices into `EngineMsg`s and the end into `EngineDone`.

What to know about `advance`:

- **Held input is tracked** (`keys_down`, `buttons_down`) and released newest first on stop, seek, loop end, timeout and `Drop`.
- **Waits pause the clock.** A pixel check or Find image seeks to its step, pauses, and polls (30 ms, 250 ms); a match resumes at the step's end. Time paused by the user doesn't count toward the timeout.
- **Text steps freeze the playhead** at their start while typing, then resume from where typing got to, so what follows keeps its time unless the text runs past the step.
- **Lateness goes into a fixed histogram**, so an endless loop measures its timing in constant memory.
- **The first injection error** (usually UIPI) is reported once; playback continues.

## Recording and the watchdog

[`rec_thread.rs`](../../src-tauri/src/rec_thread.rs) drains the hook into the `Recorder` and, every 100 ms, sends `RecProgress` (only the new cursor samples; the steps only when something other than the cursor changed, which keeps long recordings cheap) and runs the **hook watchdog**.

Windows removes a slow hook without telling anyone. The tell is a cursor that moves while no mouse events arrive for a second. The hook beats `MousePulse` on every mouse event it sees, even filtered ones, so ignored simulated input doesn't look like silence. On an alarm, the coordinator reinstalls the hook with the same config and sender: the recording goes on with a small gap, and the user gets a notice.

## Hotkeys

[`hotkeys.rs`](../../src-tauri/src/hotkeys.rs) uses `tauri-plugin-global-shortcut` (`RegisterHotKey`), which fires even when an elevated window has the focus and never sees other keys.

- **Relay's keys are settings** (`RelayKeys`). The handler compares a press with the keys as they are now (`action_for`), so a change applies at the next refresh.
- **Only the keys the session needs are registered:** idle, all of Relay's keys and every enabled macro hotkey; recording, the record key and the kill switch; playing, the play key and the kill switch.
- **Registration runs only on the main thread.** The coordinator records the wanted set (`set_active`) and posts a refresh without waiting. A refresh reads the latest wanted set when it runs, so refreshes from anywhere, in any order, end in the right state. `refresh_and_wait` is for `set_triggers`, whose answer must include current hotkey errors.
- **While a key field listens**, `hold_hotkeys(true)` unregisters everything, so a combination Relay uses can reach the field.
- **The rules** (`check_key`, `conflict`): a key must parse, can't be used twice, and can't be a macro's hotkey; the kill switch can't be none. They're refused before anything is saved, with Rust's message, which the fake core repeats word for word. The rules are [Keys](../spec/keys.md).

## Triggers

[`triggers.rs`](../../src-tauri/src/triggers.rs) runs one thread per kind. Each polls every macro's triggers through `Library::all_triggers`, an `Arc` the library rebuilds when triggers change, so polling copies nothing and changes apply without restarts.

| Thread | Polls | Decides with |
| --- | --- | --- |
| `relay-schedule` | 5 s | `next_run` from the previous tick's time, so sleep and clock changes are safe; `ScheduleWatch::take_missed` skips a run more than 2 min late |
| `relay-interval` | 1 s | `IntervalWatch`: counts restart while Relay is busy, so an interval never meets a session |
| `relay-app-launch` | 2 s | `LaunchWatch` with a `ProcessLaunchEdge` per program; a delayed run re-checks its trigger when the delay ends |
| `relay-pixel-trigger` | 250 ms | `PixelWatch` with a `PixelEdge`; a failed read is no sample, so it can't re-arm |
| `relay-image-trigger` | 500 ms | `ImageWatch`; a `finder::Looker` captures each area once per round and skips a search when the frame hasn't changed |

The watches are pure and tested; the threads only sleep and call them. A firing sends `Cmd::RunMacro`, and the coordinator's `admit` decides. Saving triggers waits up to 1 s for the app watcher's baseline, so a program started right after counts as a launch.

## Files

The data folder is `%APPDATA%\Relay`, or `RELAY_DATA_DIR`. [`storage::write_atomic`](../../src-tauri/src/storage.rs) writes a temporary file and renames it over the target, so a crash never leaves half a file. What's in each file is [File formats](file-formats.md).

### The library

[`library.rs`](../../src-tauri/src/library.rs) keeps every macro in memory, in Library order, and writes each change at once: the `.rly` file, then `library.json` (order, triggers, run counts, trash). Deleted macros move to `macros/.trash/` with enough in `library.json` to restore them in place.

Two guarantees, both tested:

- **A failed save never loses data.** The change stays in memory, the command still returns the new state, and the user gets an error (`report_unsaved`). An unsaved trash is retried with the next save, or at quit.
- **An unreadable file is never overwritten.** An invalid `library.json` is set aside as `.bad`; one that can't be *read* (locked) stops index writes for the run, so it's never replaced by an empty one.

A data file is a path in `library.json`, never a copy. [`data_file.rs`](../../src-tauri/src/data_file.rs) reads it each time the macro plays or exports, and `relay_core::data::check` decides whether it can.

### Undo

[`history.rs`](../../src-tauri/src/history.rs) keeps, per macro, up to 100 undo and redo entries, in memory only. Each entry is the name and a [`Splice`](../../crates/relay-core/src/splice.rs): the run of events the edit replaced and the time shift after it. That's a few events where a copy of the macro would be thousands. A `Batch` is one entry, and renames less than 2 s apart merge. Every `MacroView` carries `can_undo` and `can_redo`.

### Settings

[`settings.rs`](../../src-tauri/src/settings.rs) reads `settings.json` field by field: an invalid value resets only that field, and the file is set aside as `.bad`. A missing value means something different in a folder an older Relay used (`Settings::upgraded`: F9 and F10, the keys screen once) than in a new one (`Settings::default`). `update_settings` checks the keys, saves, applies what changed (hotkeys, look, text size, player) and sends `Settings` to both windows.

## Windows

### Which window shows

[`presence::shown(presence, session, settings)`](../../src-tauri/src/presence.rs) is a pure function, and the only place that decides whether the main window and the player show. It takes what the user chose (player mode, in the tray, stepped aside for the picker), the session, and the *Hide the window while…* settings. `update` applies the result; the player is always shown with `SW_SHOWNOACTIVATE`, so it never takes the keyboard.

Beside the main window, the player goes in the first corner clear of the window and of the macro's clicks (`free_corner`); otherwise at its own anchor, which a drag moves.

### Placement and the look

[`window_ctl.rs`](../../src-tauri/src/window_ctl.rs) places the main window from Rust, since only Rust knows the monitors' physical geometry. The window's anchor is its bottom center; placing it picks the monitor (`choose_monitor`), zooms the page to fit a small work area and to the *Text size* setting, and clamps it inside the work area. Only the user's drag moves the anchor. `layout` and `choose_monitor` are pure and tested.

The hard part is Windows' move loop on monitors with different scales: it keeps re-applying its own suggested size during a drag. A window subclass (`watch_move_size`) holds Relay's size until the drop. Read the code's comments before changing it.

[`look.rs`](../../src-tauri/src/look.rs) frosts both windows with Acrylic where Windows offers it (Windows 11 with transparency on) and sets the frame's theme; the page draws its own ground over the glass. [`tray.rs`](../../src-tauri/src/tray.rs) is the tray icon and menu.

## Logging and panics

[`logging.rs`](../../src-tauri/src/logging.rs): a daily rolling file in `logs/` (7 kept), non-blocking, and a panic hook that logs the thread and message. Logs say **what happened, never what was typed**.

Release builds keep `panic = "unwind"` on purpose: a panic on the engine thread unwinds through `Engine`'s `Drop`, which releases every held key and button.

The player that relay.exe embeds for exports is built by [`build.rs`](../../src-tauri/build.rs) (see [The exported player](architecture.md#the-exported-player)).

---

<p align="center"><a href="platform.md">← relay-platform</a> · <a href="ipc.md">IPC →</a></p>
