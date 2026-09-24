# Relay

A desktop macro recorder for Windows. Record mouse and keyboard input, edit it as a list of steps, and play it back at any speed, on a loop, from a hotkey, on a schedule, when an app launches or when a pixel changes.

Relay lives in a small always-on-top widget with a compact player bar and an expanded editor: a preview of the mouse path, the steps, your library, triggers, settings and a four-lane timeline. The design is in [`Design/`](Design/).

## Install

Download `Relay_x.y.z_x64-setup.exe` from the [releases](https://github.com/HappyRave/Relay/releases) and run it. It installs for your user only and needs no administrator rights. The installer isn't code-signed yet, so Windows SmartScreen may warn the first time: choose **More info → Run anyway**.

## Use

| Action | How |
| --- | --- |
| Record | **F9**, or the red button. A 3-second countdown, then everything you do is captured until **F9** again. |
| Play / pause | **F10**, or the play button. |
| Stop | **Esc**. It only does this while recording or playing, so it's free the rest of the time. |
| Emergency stop | **Ctrl + Alt + End** stops everything and pauses all triggers. |
| Compact player | **Ctrl + Shift + M**, or the arrows button. |

- **Steps:** a recording becomes steps: clicks, drags, scrolls, key combinations, typed text, waits and pixel checks. Click a step to jump there and edit it (labels, wait length, pixel check position, color, tolerance, timeout). **+ Wait** and **+ Pixel check** insert at the playhead.
- **Playback:** speed (0.5×–4×), repeat N times or forever, and *Humanize* (small random timing changes). *Stop on key press* aborts playback on any key and swallows it. *Window* coordinates follow the recorded window if it has moved.
- **Triggers** (per macro): a hotkey, a weekly schedule, when an app launches, or when a pixel turns a given color. Triggers only run when Relay is idle and the screen is unlocked.
- **Library:** every recording is saved automatically. Duplicate, delete (with Undo), export as `.rly` or `.json`, and import `.rly` files.
- **Tray:** closing the widget keeps Relay running in the tray, so hotkeys and triggers keep working. *Start with Windows* is in Settings.

## Your data

Everything stays on your PC, in `%APPDATA%\Relay`:

```text
settings.json        your settings
library.json         order, run counts and triggers
window.json          where the widget sits
macros\<id>.rly      one file per macro (portable, plain JSON)
macros\.trash\       deleted macros
logs\relay.*.log     the last 7 days of diagnostics
```

Relay never connects to the internet.

> **Privacy:** a recording stores what you type, passwords included. Stop recording before typing anything secret. The logs never contain what you type.

## Limitations

- **Apps running as administrator:** Windows doesn't let a normal app control them, so Relay warns when the app in front is elevated. Run Relay as administrator to automate those apps.
- **Lock screen and UAC:** nothing can be automated on the lock screen or while a UAC prompt is up. Triggers skip those moments.
- **Games and anti-cheat software** may ignore or penalize simulated input.
- **Pixel checks:** HDR and protected content (some video players) can change the colors Relay reads.
- **Other layouts and monitors:** a macro replays physical keys and screen positions. Replaying on a different keyboard layout or monitor arrangement can give different results.

## Development

Requirements: Rust (stable, MSVC), Node 20 or later, and WebView2 (included with Windows 11).

```bash
npm install
npm run tauri dev   # the app
npm run dev         # UI only, in a browser, on a demo desktop
npm test            # frontend unit tests
cargo test          # Rust tests; also regenerates the TS bindings and the browser fixture
npx tauri build     # the installer, in target/release/bundle/nsis
```

`src/lib/ipc/bindings/` (TypeScript types) and `src/lib/dev/sample-views.json` are generated from `relay-core` by `cargo test`. Commit them when they change; CI fails if they're stale.

Set `RELAY_DATA_DIR` to use another data folder, for example a throwaway one while testing:

```powershell
$env:RELAY_DATA_DIR = "$env:TEMP\relay-test"; npm run tauri dev
```

### Layout

- `crates/relay-core`: the macro model, step grouping, edits, the `.rly` format, playback timing, schedules and the session state machine. It has no OS dependencies.
- `crates/relay-platform`: input hooks, injection, the screen and processes behind traits. The Windows backend is in `src/windows`, with a stub for other systems.
- `src-tauri`: the app: coordinator, recorder and engine threads, triggers, storage, window, tray.
- `src`: the Svelte 5 UI.

### Releasing

Pushing a version tag such as `v1.0.0` builds the installer in GitHub Actions and attaches it to a **draft** release, to review and publish by hand. Milestone tags (`v0.x.0-mN`) don't trigger a release.

## Roadmap

- [x] M0: Scaffold and skin
- [x] M1: relay-core (model, steps, edits, format)
- [x] M2: Recording
- [x] M3: Playback engine
- [x] M4: Steps editor and pixel checks
- [x] M5: Library, settings, export and import
- [x] M6: Window polish, tray, single instance
- [x] M7: Triggers and autostart
- [x] M8: Hardening and release
- [ ] Later: AutoHotkey and standalone `.exe` export, code signing, per-monitor remapping for different monitor layouts
