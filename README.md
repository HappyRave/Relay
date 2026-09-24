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
npm run tauri dev   # the app
npm run dev         # UI only, in a browser, on a demo desktop
npm test            # frontend unit tests
cargo test          # Rust tests; also regenerates the TS bindings and the browser fixture
```

Macros and settings live in `%APPDATA%\Relay`. Set `RELAY_DATA_DIR` to use another folder, for example a throwaway one while testing:

```powershell
$env:RELAY_DATA_DIR = "$env:TEMP\relay-test"; npm run tauri dev
```

`src/lib/ipc/bindings/` (TypeScript types) and `src/lib/dev/sample-views.json` are generated from `relay-core` by `cargo test`. Commit them when they change; CI fails if they're stale.

## Roadmap

- [x] M0: Scaffold and skin
- [x] M1: relay-core (model, steps, edits, format)
- [x] M2: Recording
- [x] M3: Playback engine
- [x] M4: Steps editor and pixel checks
- [x] M5: Library, settings, export and import
- [x] M6: Window polish, tray, single instance
- [x] M7: Triggers and autostart
- [ ] M8: Hardening and release
