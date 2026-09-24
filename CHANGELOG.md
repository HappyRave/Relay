# Changelog

## v1.0.0: Hardening and release

- **Hook watchdog:** Windows silently removes input hooks it considers too slow. While recording, Relay now notices the cursor moving without events and reinstalls the hook.
- **Logs:** Relay keeps a log in `%APPDATA%\Relay\logs` (7 days) of sessions, triggers, saved recordings, playback results and crashes. What you type is never logged.
- **Panics:** a crash during playback still releases every key and button the macro was holding.
- **Privacy note:** Settings now says that recordings store what you type.
- **Installer:** a per-user NSIS installer (2.2 MB, no administrator rights needed). It isn't code-signed yet, so SmartScreen may warn on first run.
- **CI:** the Windows job now builds the installer and keeps it as an artifact, and a new Linux job tests the portable crates. A version tag produces a draft GitHub release.
- **Soak test:** 10 minutes of continuous looping on the release build kept memory flat (about 34 MB, plus about 120 MB for the WebView2 runtime). Timing stayed accurate: 12,000 events at p99 0.008 ms late and 3 ms of drift in total.
- **Documentation:** the README covers installing, using Relay, your data, privacy and limitations.

## v0.8.0-m7: Triggers and autostart

- **Triggers:** a macro can now run on its own.
  - **Hotkey:** any combination you capture. Clashes with Relay's own hotkeys or another macro's are refused, and one owned by another app is reported.
  - **Weekly schedule:** handles daylight saving; a run missed while the PC slept is skipped.
  - **App launch:** runs when a program starts, after a configurable delay.
  - **Pixel change:** runs when a pixel turns a given color.
- **When triggers run:** only when Relay is idle and the screen is unlocked. A trigger that fires during another run is skipped with a notice.
- **Pausing:** the kill switch (Ctrl+Alt+End) pauses all triggers. Resume them from the tray's "Triggers active" item or the Triggers tab. The kill switch now also works while "Stop on key press" is watching the keyboard.
- **Start with Windows:** Relay starts hidden in the tray.
- **Safe upgrade:** old hotkey labels and the sample macros' hotkeys come in switched off, so nothing starts playing unexpectedly.

## v0.7.0-m6: Window polish, tray, single instance

- **Position:** the widget remembers where you put it and whether it was compact.
  - It stays anchored at its bottom-center when switching modes and stays inside the monitor's work area.
  - If its monitor is gone, it comes back on the primary one.
- **Small screens:** the widget zooms down to fit (for example 1366×768 at 125%).
- **Tray icon:**
  - The menu has Show/hide, Record, Stop, Open macros folder and Quit.
  - Left-click shows or hides the widget.
  - The tooltip shows whether Relay is recording or playing.
- **Close to tray:** the close button (and Alt+F4) hides Relay to the tray, so hotkeys keep working. There's a setting for this.
- **One copy only:** starting Relay again brings the running one forward.
- **No focus stealing:** clicking the widget during a recording or playback no longer takes the keyboard from the app you're automating.

## v0.6.0-m5: Library, settings, export and import

- **Library actions:** Duplicate, and Delete with Undo, on each Library row.
  - Deleted macros go to `macros\.trash` and keep their position and stats, so restoring puts them back exactly as they were, even after a restart.
  - Deleting is refused while a session is running.
- **Export** `.rly` (editable) or `.json` (pretty, with the derived steps) through the native Save dialog.
- **Import** `.rly` or Relay `.json` files through the native Open dialog. Broken files are reported without stopping the others, and a macro that's already in the library imports as a copy.
- **Toasts:** messages and errors appear at the bottom of the side panel, some with an action (Undo).
- **Settings** have been saved in `settings.json` since M2.

## v0.5.0-m4: Steps editor and pixel checks

- **Pixel checks wait for the real screen:**
  - Playback pauses on the IF step and samples every 30 ms until the pixel matches within tolerance, then continues.
  - If the timeout passes first, playback stops with "Pixel check timed out at step N" and the playhead stays on that step.
  - Time spent paused doesn't count toward the timeout.
- **Inline step editor:** click a step to edit it.
  - Clicks, drags, waits and pixel checks get a label.
  - Waits get a duration.
  - Pixel checks get position, color, tolerance and timeout, plus **Pick**, which samples the pixel under your cursor after a 3-second countdown.
- **+ Pixel check** samples the live color at the macro's cursor position. Pixel rows show a color swatch.
- **Preview zoom:** the preview zooms to the area the macro touches, so recordings on large or multi-monitor desktops stay readable.

## v0.4.0-m3: Playback engine

- **Playback injects real input** through SendInput:
  - The cursor moves to the exact pixel on the virtual desktop.
  - All mouse buttons and the wheel are replayed.
  - Keys replay by recorded scan code. When none was recorded they use the virtual key, which keeps AZERTY and other layouts correct.
  - Relay's own hook ignores its injected input.
- **Timing** uses a high-resolution waitable timer plus a short spin, on a high-priority thread with Windows power throttling turned off. Measured lateness was under 1 ms at p99 in testing.
- **Controls:** speed (0.5×–4×), repeat N times or forever, and Humanize (seeded per-step jitter, with presses and releases shifted together). Pause, resume and seeking re-anchor the clock without drift.
- **Stopping:** Esc, "Stop on key press" (the key is swallowed so it doesn't type into the target), the Ctrl+Alt+End kill switch and the Stop button. Every key and button the engine holds is released on stop, seek, loop end and drop, so nothing gets stuck.
- **Focus:** pressing Play in Relay's window hands focus back to the app you were using, via a foreground tracker.
- **"Window" coordinates** follow the anchor window if it has moved.
- **Warnings** appear for an elevated (administrator) target, which Windows won't let Relay control.
- **Click-through:** the widget lets clicks through while a macro clicks under it.
- **Runs:** completed runs are counted, with the last-run time, in the library.

## v0.3.0-m2: Recording

- **Recording on Windows** uses system-wide keyboard and mouse hooks (WH_KEYBOARD_LL / WH_MOUSE_LL) with high-resolution timestamps.
  - Clicks on Relay's own window, keys typed into it and the control hotkeys are not recorded. Input injected by other programs is ignored unless you turn that off in Settings.
  - Keys are stored by physical scan code, with the character they typed in the current layout.
  - The window under the first click becomes the macro's anchor window.
  - Each recording saves the monitor layout, DPI and double-click settings.
- **Global hotkeys** (F9 record, F10 play/pause, Ctrl+Shift+M compact player, Ctrl+Alt+End kill switch) are registered according to the session state. Esc stops a session and is swallowed, but only while a session is running.
- **Session control** moved to Rust. The coordinator runs relay-core's state machine, the 3-second countdown and live recording progress. It streams to the UI over one channel.
- **Playback** timing comes from a Rust clock (speed, pause, seek, loops). Input injection comes in M3.
- **Storage:** macros and settings are saved under `%APPDATA%\Relay` (`RELAY_DATA_DIR` overrides it) with atomic writes. The first run seeds the four sample macros.
- **Per-monitor DPI awareness** (v2) is declared in the app manifest.

## v0.2.0-m1: relay-core

- `relay-core` crate with no OS dependencies:
  - **Model:** events store presses and releases in virtual-desktop pixels, with recording metadata (monitors, DPI, double-click settings, anchor window) and per-macro playback options.
  - **Step grouping:** CLICK (with double clicks), DRAG, SCROLL, KEYS, TYPE (including Shift and AltGr text), WAIT and IF.
  - **Edits:** insert, delete and retime waits and pixel checks, labels and renames. Normalization keeps every press balanced.
  - **`.rly` format:** versioned, with a migration from the M0 prototype export (v0) and a pretty JSON export that includes the derived steps.
  - **Session state machine:** the transitions and hotkey sets for idle, countdown, recording, playing and paused.
  - **Schedules:** weekly schedules that handle DST gaps and overlaps.
  - **Triggers:** edge detectors for "pixel changes" and "app launches".
- 68 tests, including property tests over random recordings and edit sequences, insta snapshots of the format, and the design's invoice macro grouping into its 12 steps.
- TypeScript bindings generated with ts-rs. The UI now gets macros, steps and edits from Rust through Tauri commands; the design's four samples seed an in-memory library.
- A browser preview (`npm run dev`) with a read-only fixture generated from the same samples.
- CI on Windows: tests, clippy, type check, and a check that the generated files are current.

## v0.1.0-m0: Scaffold and skin

- Cargo workspace (`relay-core`, `relay-platform`, `src-tauri`) on Tauri 2.11.
- Svelte 5 + TypeScript frontend on Vite 8, with the Modernist design tokens and Archivo bundled locally (no network requests).
- Frameless, always-on-top widget matching the design at 944×612 (expanded) and 604×68 (compact). Switching modes keeps the bottom-center fixed.
- The full UI from the prototype: path preview, Steps / Library / Triggers / Settings tabs, transport, four-lane timeline and export dialog.
- An in-browser mock engine (`npm run dev`) that runs the prototype's logic on its demo desktop. Recording, playback, loops, seek and step edits all work there.
- Frontend unit tests for the timeline and preview math (`npm test`).
