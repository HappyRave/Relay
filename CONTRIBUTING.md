# Contributing to Relay

Thanks for helping. This page covers setting up, the everyday commands and the conventions. For how Relay works inside, read the [engineering guide](docs/engineering/README.md), starting with [Architecture](docs/engineering/architecture.md).

- [Requirements](#requirements)
- [First run](#first-run)
- [Everyday commands](#everyday-commands)
- [Where things go](#where-things-go)
- [Code conventions](#code-conventions)
- [Git workflow](#git-workflow)
- [Pull request checklist](#pull-request-checklist)
- [Reporting bugs](#reporting-bugs)

## Requirements

| Tool | Version | Notes |
| --- | --- | --- |
| Windows | 10 or 11, 64-bit | The app. The core crates also build on Linux and macOS. |
| Rust | stable, 1.95 or later | MSVC toolchain (`rustup default stable-x86_64-pc-windows-msvc`). 1.95 is `rust-version` in `Cargo.toml`, set by the dependencies and checked in CI. |
| Visual Studio Build Tools | 2022 | "Desktop development with C++" |
| Node.js | 20 or later | CI uses 24 |
| WebView2 runtime | | Included with Windows 11 |

## First run

```bash
git clone https://github.com/HappyRave/Relay.git
cd Relay
npm install
npm run tauri dev
```

The first build takes a few minutes. After that, Rust changes rebuild incrementally and UI changes hot-reload.

> [!TIP]
> Use a throwaway data folder, so development doesn't touch your real macros:
>
> ```powershell
> $env:RELAY_DATA_DIR = "$env:TEMP\relay-dev"; npm run tauri dev
> ```
>
> Delete the folder to start over with the sample macros.

## Everyday commands

| Command | Does |
| --- | --- |
| `npm run tauri dev` | Runs the app with hot reload |
| `npm run dev` | Runs only the UI, in a browser at <http://localhost:1420>, with the sample macros and a simulated session. Quickest for UI work. |
| `cargo test --workspace` | Every Rust test. **Also regenerates** the TypeScript bindings and the browser fixture. |
| `cargo test -p relay-core` | Just the core, in seconds |
| `cargo clippy --workspace --all-targets -- -D warnings` | Lints, as strict as CI |
| `npm test` | Vitest |
| `npm run check` | svelte-check: types and accessibility |
| `npx tauri build` | The release build and installer in `target/release/bundle/nsis/` |

`npm run dev` and `npm run tauri dev` both use port 1420, so stop one before starting the other.

## Where things go

| If you're changing… | Look in | Test with |
| --- | --- | --- |
| How events become steps, edits, the file format, timing, sessions, schedules | `crates/relay-core` | Unit tests and property tests in the same crate |
| Hooks, injection, the timer, screen or window queries | `crates/relay-platform/src/windows` | The end-to-end harness (see [Testing](docs/engineering/testing.md#end-to-end-testing)) |
| Recording conversion or the key map | `crates/relay-platform/src/{recorder,keymap}.rs` | Unit tests with synthetic input |
| Threads, commands, storage, triggers, the window, the tray | `src-tauri/src` | Unit tests, and the app |
| The UI | `src/` | Vitest for `lib/`, `npm run dev` for components |

A rule of thumb: if it can be a pure function, it goes in `relay-core`, takes time as an argument, and gets a test.

## Code conventions

**Rust**
- `cargo fmt` and zero clippy warnings.
- Module docs (`//!`) say what a module is for. Doc comments explain *why*, not what the code already says.
- No `unwrap()` on anything that can fail at runtime (I/O, parsing user files, OS calls). Locks and thread spawns may `unwrap`/`expect`.
- Hook callbacks must stay minimal: no allocation, locks, logging or blocking.
- Anything that presses a key or button must release it on every path. Test it with the fake injector.
- Logs describe what happened, **never what the user typed**.
- A type that crosses IPC derives `TS` with `#[ts(export)]`. Commit the regenerated bindings.

**TypeScript and Svelte**
- Svelte 5 runes only (`$state`, `$derived`, `$effect`, `$props`).
- Components talk to the `relay` store, never to `backend` or `invoke`.
- Colors and spacing come from `var(--…)` tokens in `src/styles/tokens.css`. Corners stay square.
- Match the surrounding formatting: 2-space indent, double quotes, semicolons, lines up to about 120 columns.

**User-facing text**
- Short, plain sentences that say what happened and what to do: *"Couldn't find EXCEL.EXE; playing at screen coordinates."*
- Sentence case for labels and buttons.

## Git workflow

- **Branches**: work happens on a branch, never directly on `main`. Milestones use `mN-short-name`, fixes `fix-short-name`, docs `docs-short-name`.
- **Commits**: small and focused, in [Conventional Commits](https://www.conventionalcommits.org/) style: `feat(recorder): …`, `fix(engine): …`, `docs: …`, `test: …`, `ci: …`, `chore: …`.
- **Merging**: `git merge --no-ff` into `main`, so each branch stays visible in history. Milestones are tagged `v0.N.0-mN`.
- **Releases**: a `vX.Y.Z` tag builds a draft release (see [Releases](docs/engineering/testing.md#releases)).

## Pull request checklist

- [ ] `cargo test --workspace` passes and regenerated files are committed
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` is clean
- [ ] `npm test` and `npm run check` pass
- [ ] New logic in `relay-core` or `lib/` has tests
- [ ] Behavior changes are tried in the real app (`npm run tauri dev`)
- [ ] The [user guide](docs/user-guide/README.md) and [engineering guide](docs/engineering/README.md) are updated if behavior or architecture changed
- [ ] `CHANGELOG.md` has an entry under *Unreleased* for anything users will notice

## Reporting bugs

[Open an issue](https://github.com/HappyRave/Relay/issues) with:

1. what you did, what you expected and what happened,
2. your Windows version, display scaling and keyboard layout,
3. the log from `%APPDATA%\Relay\logs` for that day (it never contains what you typed),
4. if it's about a specific macro, the exported `.rly`. Check it for anything private first, since macros contain typed text.
