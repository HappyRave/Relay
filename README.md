# Relay

A desktop macro recorder: record mouse and keyboard input, edit it as a list of steps, and play it back at any speed, on a loop, from a hotkey, on a schedule or when an app launches.

Relay lives in a floating, always-on-top widget with a compact player bar and an expanded editor (preview, steps, library, triggers, settings and a four-lane timeline). The UI follows the Modernist design in [`Design/`](Design/).

## Stack

- **Core:** Rust (`crates/relay-core`): model, step grouping, edits, scheduling. No OS dependencies.
- **Platform:** Rust (`crates/relay-platform`): input hooks, injection, pixels and processes behind traits. Windows backend first.
- **App:** Tauri 2 (`src-tauri`), with a Svelte 5 + TypeScript frontend (`src`).

## Development

Requirements: Rust (stable, MSVC), Node 20 or later, and WebView2 (included with Windows 11).

```bash
npm install
npm run tauri dev
```

## Roadmap

- [ ] M0: Scaffold and skin
- [ ] M1: relay-core (model, steps, edits, format)
- [ ] M2: Recording
- [ ] M3: Playback engine
- [ ] M4: Steps editor and pixel checks
- [ ] M5: Library, settings, export and import
- [ ] M6: Window polish, tray, single instance
- [ ] M7: Triggers and autostart
- [ ] M8: Hardening and release
