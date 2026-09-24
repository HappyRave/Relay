# Changelog

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
