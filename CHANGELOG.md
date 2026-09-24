# Changelog

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
