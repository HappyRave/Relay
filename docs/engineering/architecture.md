# Architecture

Relay is a Tauri 2 app: a Rust process that owns everything OS-related and all state, and a Svelte UI in a WebView2 window that shows it. The Rust side is split into five crates so the logic that matters most can be tested without Windows, and so a macro can play without the app, as an [exported program](#the-exported-player).

- [The crates](#the-crates)
- [Threads](#threads)
- [How a recording flows](#how-a-recording-flows)
- [How a playback flows](#how-a-playback-flows)
- [How a trigger flows](#how-a-trigger-flows)
- [The exported player](#the-exported-player)
- [Design principles](#design-principles)
- [Repository layout](#repository-layout)

## The crates

```mermaid
flowchart TB
    subgraph ui["UI (src/, Svelte 5)"]
        store["RelayStore<br/>(runes)"]
        comps["Components"]
    end
    subgraph app["relay (src-tauri/)"]
        cmds["Tauri commands"]
        coord["Coordinator"]
        rec["Recorder thread"]
        trig["Triggers"]
        lib["Library / storage"]
    end
    subgraph playback["relay-playback"]
        engine["Engine · spawn"]
        finder["finder · plan"]
    end
    player["relay-player<br/>(exported programs)"]
    subgraph platform["relay-platform"]
        traits["InputHook · Injector · Timer<br/>Screen · WindowQuery"]
        win["windows/ backend"]
        stub["stub backend"]
        recorder["Recorder · keymap"]
    end
    subgraph core["relay-core (pure)"]
        model["model · steps · edit"]
        fmt["format · playback · session"]
        sched["schedule · triggers · view"]
    end
    comps --> store
    store -- "invoke()" --> cmds
    coord -- "Channel&lt;EngineMsg&gt;" --> store
    cmds --> coord & lib
    coord --> engine
    coord --> rec
    trig --> coord
    app --> playback & platform & core
    player --> playback
    playback --> platform & core
    platform --> core
    traits -.implemented by.-> win & stub
```

| Crate | Path | Depends on | Role |
| --- | --- | --- | --- |
| **relay-core** | `crates/relay-core` | serde, chrono, uuid, ts-rs | **Pure logic, no I/O.** The macro model, step grouping, edit operations, the `.rly` format and migrations, the playback clock and humanize, the session state machine, schedules and trigger edge detectors. Builds and tests on any OS. |
| **relay-platform** | `crates/relay-platform` | relay-core, `windows` | **The OS boundary.** Traits for input hooks, injection, timers, screen and window queries, a Windows backend, and a stub for other systems. The recorder (raw input → events) and the key map are OS-independent and live here too. |
| **relay-playback** | `crates/relay-playback` | relay-core, relay-platform | **Playing a macro.** The engine and its thread (reporting through a `PlaybackSink`), finding an image on the real screen, and planning a playback (`PlayPlan::for_macro`, `window_offset`). Shared by the app and the player. |
| **relay-player** | `crates/relay-player` | the three above, `windows` | **The exported program.** A small Win32 player (about 1 MB) that plays the macro appended to it. Its logic (options, exit codes, status text, placement) is a library tested on any OS. |
| **relay** | `src-tauri` | all but the player, Tauri 2 and plugins | **The app.** Threads, state, persistence, commands for the UI, the tray, the window and triggers. It embeds the player's release build, for exporting. |

The UI never touches the OS or the files. It calls commands and listens to one message stream (see [IPC](ipc.md)).

## Threads

Relay is a handful of long-lived threads that talk over [crossbeam](https://crates.io/crates/crossbeam-channel) channels. Nothing shares mutable state except behind a `Mutex` held briefly (the library and settings).

```mermaid
flowchart LR
    hk["Hotkey handler<br/>(Tauri main thread)"] -- Cmd --> C
    ui["Commands<br/>(main thread, or the async pool)"] -- Cmd --> C
    tray["Tray menu"] -- Cmd --> C
    T["relay-schedule<br/>relay-app-launch<br/>relay-pixel-trigger"] -- "Cmd::RunMacro" --> C
    C(["relay-coordinator<br/>session state machine"])
    C -- "start / stop" --> H["relay-hook<br/>WH_KEYBOARD_LL + WH_MOUSE_LL"]
    H -- RawInput --> R["relay-recorder"]
    R -- "RecProgress" --> E[("Emitter<br/>Channel&lt;EngineMsg&gt;")]
    R -- "Escape / HookLost" --> C
    C -- "spawn / EngineCmd" --> P["relay-engine<br/>high priority"]
    P -- "PlayTick" --> E
    P -- "EngineDone" --> C
    C -- "Session / Finished / Notice" --> E
    E --> W["WebView (UI)"]
```

| Thread | Lives | Does |
| --- | --- | --- |
| **relay-coordinator** | Whole run | Owns the session mode. Applies `session::step`, then performs the effects: start or stop the hook, recorder and engine, ask for the global hotkeys of the new state, emit mode changes. Runs the recording countdown in its own loop. The single place where sessions change. |
| **relay-hook** | During a session | Installs the low-level hooks and pumps their messages. Callbacks only filter, timestamp and `try_send`. |
| **relay-recorder** | While recording | Turns raw input into events, sends live progress 10 times a second, runs the hook watchdog. |
| **relay-engine** | While playing | Injects events on time with the precision timer, polls pixel checks, reports ticks 30 times a second. Runs at `THREAD_PRIORITY_HIGHEST`. |
| **relay-schedule**, **relay-app-launch**, **relay-pixel-trigger** | Whole run | Poll every 5 s, 2 s and 250 ms, and ask the coordinator to run a macro when a trigger fires. |
| **Tauri main thread** | Whole run | The window, the tray, the global hotkey handler (`RegisterHotKey` messages arrive here), hotkey registration, and the commands that don't write files. |
| **relay-stop-keys** | While playing | Forwards Esc and stop keys from the watching hook to the coordinator. |

## How a recording flows

```mermaid
sequenceDiagram
    autonumber
    actor U as User
    participant HK as Global hotkey
    participant C as Coordinator
    participant H as Hook thread
    participant R as Recorder thread
    participant L as Library
    participant UI

    U->>HK: F9
    HK->>C: Cmd::Input(ToggleRecord)
    C->>C: step(Idle, ToggleRecord) → Countdown
    C-->>UI: Session{countdown}, Countdown{left_ms}…
    C->>C: CountdownDone → Recording
    C->>H: start(HookMode::Record{own window, skip F9, Esc stops})
    C->>R: spawn(Recorder)
    loop every input event
        U->>H: mouse / keyboard
        H->>R: RawInput (QPC time)
    end
    loop every 100 ms
        R-->>UI: RecProgress{moves, steps?}
    end
    U->>HK: F9
    HK->>C: ToggleRecord → Idle
    C->>H: drop the hook session
    C->>R: finish() → events
    C->>L: insert_front(Macro) and write .rly
    C-->>UI: Saved{id}, LibraryChanged, Session{idle}
```

What happens to an event on the way:

1. **Hook callback** ([`windows/hook.rs`](../../crates/relay-platform/src/windows/hook.rs)): drops Relay's own injected input (tagged with `RELAY_MAGIC` in `dwExtraInfo`) and, by default, anyone else's injected input. It leaves out clicks on Relay's window (the window under the cursor), keys typed while Relay is in front, and F9, and a release always goes the way its press went. Esc is swallowed and reported as `RawKind::Escape`. Everything else is timestamped with `QueryPerformanceCounter` and sent without blocking.
2. **Recorder** ([`recorder.rs`](../../crates/relay-platform/src/recorder.rs)): converts to macro time, throttles cursor samples to one per 16 ms, translates key presses into the characters they type (with the current layout, without disturbing dead keys), maps scan codes to W3C key codes, and drops the kill switch.
3. **On stop**: trailing modifiers (the Ctrl and Alt of a kill switch) are trimmed, presses are balanced by `normalize`, and a recording without real content is discarded. The window under the first click becomes the macro's *anchor window*.

## How a playback flows

```mermaid
sequenceDiagram
    autonumber
    actor U as User
    participant C as Coordinator
    participant W as WindowQuery
    participant H as Hook thread
    participant P as Engine thread
    participant OS as SendInput
    participant UI

    U->>C: F10 / play button (from = playhead)
    C->>C: step(Idle, TogglePlay) → Playing, hotkeys = {F10, Kill}, generation += 1
    C->>W: foreground is Relay? restore_previous()
    C->>W: input blocked (integrity)? → Notice
    C->>C: Window mode? offset = anchor moved by (dx, dy)
    C->>H: start(HookMode::Watch{stop_on_key, pass F10})
    C->>P: spawn(PlayPlan{events, steps, speed, repeat, jitter, offset, from})
    loop until done
        P->>P: timer.wait_until(next deadline)
        P->>OS: move / button / wheel / key (tagged RELAY_MAGIC)
        P-->>UI: PlayTick{t, speed, loop} every 33 ms
    end
    alt Esc or any key
        H->>C: Escape / StopKey → StopPlayback
        C->>P: stop() → timing (Drop releases held input)
        C-->>UI: Finished{stopped, timing}
    else finished
        P->>C: EngineDone{generation, completed, timing}
        C-->>UI: Finished{completed, timing}, count the run
    end
    C->>C: run history += RunEntry{source, outcome, checks}
    C-->>UI: RunsChanged, Session{idle}
```

The engine computes each event's deadline from a `PlayClock` (macro time = anchor + (wall time − anchor wall) × speed), sleeps until it with a high-resolution waitable timer plus a 1 ms spin, and injects. Speed changes, pauses and seeks re-anchor the clock, so timing never drifts. In a 10-minute soak test, 12,000 events were injected with a p99 lateness of 0.008 ms and a maximum of about 1 ms.

Every playback ends in the coordinator's `finish_playback`, however it ends. It takes the engine's `RunReport` (loops played, each pixel check's and Find image step's result and wait, the timing) and the `RunStart` it kept when the playback began (the source, the options, the wall clock), and adds a `RunEntry` to the [run history](file-formats.md#runsjson). A run still going when Relay quits isn't recorded.

## How a trigger flows

```mermaid
flowchart LR
    S["Schedule tick (5 s)"] --> F{"fire()"}
    A["Process appeared (2 s poll)<br/>+ delay"] --> F
    X["Pixel edge (250 ms poll)"] --> F
    K["Macro hotkey<br/>(RegisterHotKey)"] --> F
    F -- "Cmd::RunMacro" --> C{"Coordinator"}
    C -- "triggers paused" --> N1["drop"]
    C -- "not idle" --> N2["Notice: Skipped … Relay was busy<br/>+ run history"]
    C -- "desktop locked / UAC" --> N3["log and skip<br/>+ run history"]
    C -- "idle" --> P["step(Idle, Trigger) → Playing from 0"]
```

The trigger threads don't decide whether a macro can run. They only detect the event and send `Cmd::RunMacro`. The coordinator, which knows the session mode and whether triggers are paused, decides, and adds a skipped run to the history. A schedule tick that finds a run more than 2 minutes late sends `Cmd::LogSkip` instead (`ScheduleWatch::take_missed`). See [The app → Triggers](app.md#triggers).

## The exported player

Exporting a macro as a **Standalone program** writes the player's exe with the macro's `.rly` appended (with its data file's rows, if it has one), then a 20-byte trailer (see [File formats](file-formats.md#exported-programs)). The player reads its own file, finds the macro, and plays it with the same engine as the app.

```mermaid
flowchart LR
    B["src-tauri/build.rs<br/>cargo build -p relay-player --release<br/>(target/player)"] --> I["relay.exe<br/>include_bytes!"]
    I -- "export_macro(exe)" --> X["program.exe =<br/>player ‖ .rly ‖ trailer"]
    X -- "runs" --> P["relay-player:<br/>from_program(own exe)"]
    P --> E["relay_playback::spawn"]
    X -- "import_macros" --> R["Relay: from_file"]
```

- **Build.** `src-tauri/build.rs` builds the player in release, in a target folder of its own (`target/player`: cargo locks the folder of the build that runs the script), without the outer build's wrappers and flags (clippy, coverage). relay.exe embeds it, so the portable exe exports too. `RELAY_PLAYER_EXE` uses a prebuilt player instead.
- **Flow.** Parse the options over the saved playback options, place a small window (`WS_EX_TOPMOST | WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW`, never focused) in the first corner of the primary work area where the macro doesn't click, count down, check the input desktop (a locked screen exits with 6), warn about an elevated app in front, apply the window offset, then `relay_playback::spawn` with a sink that posts `WM_APP` messages to the window. Ticks are coalesced, so the window redraws at most once per message.
- **Stopping.** The Stop button, a Watch hook (Esc, stop on key press, and the kill switch via `report_kill_switch`, since Relay may own the hotkey), and `RegisterHotKey` for Ctrl + Alt + End, which also works when an elevated window is in front. Stopping drops the `EngineHandle`, which releases held input.
- **The window** is passed as `own_window`, so Find image paints it over and never finds it. When every corner is busy, it's click-through (`WS_EX_TRANSPARENT`), as the app's widget is during playback.
- **Exit codes** say how it ended: 0 completed, 1 error, 2 bad options, 3 stopped, 4 kill switch, 5 a check timed out, 6 screen locked. `--quiet` uses a message-only window, so the hotkey and the timers still work with nothing shown.
- **Resources.** Its manifest makes it per-monitor DPI aware (v2) like Relay, and `asInvoker`; it carries Relay's icon and a version resource.

## Design principles

- **A pure core.** Everything that can be a pure function is one, in `relay-core`: grouping, edits, the clock, the session state machine, schedules. They take time as a parameter (`now: f64`) instead of reading a clock, so they're tested exhaustively, including with property tests, and run on Linux CI.
- **One owner per piece of state.** The coordinator owns the session. The engine owns what it pressed. The hook thread owns the hooks. Other threads send messages.
- **Never wait on the main thread while holding a lock.** Window, tray and hotkey calls from other threads block until the main thread runs them; hotkey registration only ever runs there.
- **Effects, not calls.** `session::step(mode, input) → (mode, effects)` returns what should happen, and the coordinator performs it. The table of transitions is readable in one screen, and tests assert it.
- **Never leave input stuck.** The engine tracks every key and button it pressed and releases them on stop, seek, loop end, `Drop` and panic (the release profile unwinds instead of aborting for this reason). Edits keep presses balanced.
- **Don't fight Windows.** Hook callbacks do the minimum, since Windows silently removes slow hooks (and Relay detects and reinstalls one when it happens). Hotkeys go through `RegisterHotKey`, which works even when an elevated window is in front. Elevated targets, the lock screen and UAC are detected and explained instead of failing silently.
- **Local and private.** No network. Files are plain JSON, written atomically. Logs record what happened, never what was typed.
- **Portable later.** OS access is behind traits, so a macOS or Linux backend is a new module in `relay-platform`, not a rewrite.

## Repository layout

```text
Relay/
├── crates/
│   ├── relay-core/          pure logic (model, steps, edit, format, playback, session, schedule, triggers, view)
│   │   └── src/snapshots/   insta snapshots of the file format
│   ├── relay-platform/      OS traits, recorder and key map
│   │   └── src/windows/     hook, inject, timer, screen, window, text
│   ├── relay-playback/      the engine, finding images on screen, planning a playback
│   └── relay-player/        the exported program: its logic, and the Win32 shell (win.rs)
├── src-tauri/               the app
│   ├── build.rs             Tauri's build, and the release player that relay.exe embeds
│   ├── src/                 coordinator, engine (the app's sink), rec_thread, hotkeys, triggers, commands, ipc, library, …
│   ├── tauri.conf.json      window, bundle and installer config
│   └── app.manifest         PerMonitorV2 DPI awareness
├── src/                     the Svelte UI
│   ├── components/          widget, compact bar, expanded editor, tabs, dialogs
│   ├── lib/state/           the RelayStore
│   ├── lib/ipc/             backend abstraction and generated bindings
│   └── styles/              design tokens and base styles
├── Design/                  the Claude Design source files
├── docs/                    this documentation
└── .github/workflows/       CI and releases
```

---

<p align="center"><a href="README.md">← Engineering guide</a> · <a href="README.md">Contents</a> · <a href="core.md">relay-core →</a></p>
