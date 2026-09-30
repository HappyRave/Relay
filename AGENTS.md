# AGENTS.md

Guidance for AI coding agents (and humans) working on Relay. It's the project's working memory: what Relay is, where it stands, how to build and test it, the rules the code follows, and the lessons that aren't obvious from the code. Read it before changing anything.

The deeper references are in the repo, and this file links to them rather than repeating them:

- [README.md](README.md): what Relay is, for users, and the roadmap
- [CONTRIBUTING.md](CONTRIBUTING.md): setup, commands, code conventions, git workflow, PR checklist
- [docs/engineering/](docs/engineering/README.md): how it works inside. Start with [architecture.md](docs/engineering/architecture.md); [testing.md](docs/engineering/testing.md) covers the tests, CI and releases.
- [docs/user-guide/](docs/user-guide/README.md): the behavior users are promised. **This is the spec**: when code and guide disagree, one of them is a bug.
- [CHANGELOG.md](CHANGELOG.md): what changed, per version

## Contents

- [The project in brief](#the-project-in-brief)
- [Where things stand](#where-things-stand)
- [Setting up on a new machine](#setting-up-on-a-new-machine)
- [Repository map](#repository-map)
- [Commands](#commands)
- [Definition of done](#definition-of-done)
- [Rules the code follows](#rules-the-code-follows)
- [Testing](#testing)
- [Documentation](#documentation)
- [Git, CI and GitHub](#git-ci-and-github)
- [Releasing](#releasing)
- [Gotchas](#gotchas)
- [Known limitations and open items](#known-limitations-and-open-items)
- [Working with the maintainer](#working-with-the-maintainer)

## The project in brief

Relay is a **Windows desktop macro recorder**: it records mouse and keyboard input, shows it as editable steps (*Click · Save*, *Ctrl + S*, *"invoice_2026"*), and plays it back with the original timing, faster, looped or slightly randomized. Macros can be triggered by a hotkey, a weekly schedule, a program starting, or a pixel changing color. It's a small floating widget (a 600×64 compact player, or a ~944×612 editor) that stays on top of other windows.

- **Stack:** Rust (edition 2024) + [Tauri 2](https://tauri.app/) + a [Svelte 5](https://svelte.dev/) (runes) and TypeScript UI built with Vite. WebView2 on Windows.
- **Three crates:** `crates/relay-core` (pure logic, no OS, builds anywhere), `crates/relay-platform` (OS traits plus the Windows backend and a stub for other OSes), `src-tauri` (the app: threads, commands, storage, window, tray).
- **Design:** built from the prototype in `Design/Macro Recorder.dc.html` (the *Modernist* design system in `Design/_ds/`): Archivo font, red accent `#EC3013`, square corners, 2 px rules. The demo "desktop" drawn around the widget in the prototype is only the browser preview's backdrop.
- **License:** MIT. Repository: https://github.com/HappyRave/Relay (public).

## Where things stand

- **Latest release: v1.3.0** (2026-09-29), from `main`. Releases so far: v1.0.0, v1.1.0, v1.2.0, v1.3.0. Each has an NSIS installer and a portable exe.
- **Milestone history:** M0–M8 built v1.0. Then: m9 editor polish (v1.1.0), m10 Keep on top, m11 dependency upgrades, m12 a four-reviewer architecture refactor, m13 the three-layer test suite, m14 a four-reviewer audit of every test against the user guide (about 90 findings fixed), then v1.2.0. Then fix-multi-monitor-scale and m15 the control bar, preview bar, screenshots and resizable editor (v1.3.0). Since then, unreleased: m16 mouse moves as steps, m17 Find image (with fix-find-image-own-window), m18 the run history. `git log --first-parent main` shows them as merges.
- **Tests at v1.3.0:** about 290 Rust (unit, property, snapshot), about 570 Vitest (store, backend contract, fake core, every component), about 85 end-to-end tests against the built app. Rust unit coverage is about 76% of lines; the coordinator, commands, tray and Windows backend are exercised end to end instead. The frontend is at about 99.8% of lines.
- **Next, per the [roadmap](README.md#roadmap):** AutoHotkey v2 and standalone `.exe` export (the Export dialog already shows them as "Coming later"), code signing (needs a certificate; free options for open source: SignPath Foundation, Certum's open-source certificate, Azure Trusted Signing), remapping macros to a different monitor layout, macOS and Linux backends.
- **Also open:** see [Known limitations and open items](#known-limitations-and-open-items).

Before starting work, check the open pull requests (`gh pr list`, and the description of the one you're continuing), `git log --oneline -15`, `git status`, the README roadmap and the *Unreleased* section of `CHANGELOG.md`.

## Setting up on a new machine

| Tool | Version used | Notes |
| --- | --- | --- |
| Windows | 10 or 11, 64-bit | The app is Windows-only; `relay-core` and `relay-platform` also build and test on Linux |
| Rust | 1.98.1 stable (MSVC) | `rust-version` (MSRV) is **1.95**, set by `sysinfo`, checked by the CI `msrv` job |
| Visual Studio Build Tools | 2022 | "Desktop development with C++" |
| Node.js / npm | 26.10 / 12.1 | CI uses Node 26 |
| WebView2 runtime | | Included with Windows 11 |
| `cargo-llvm-cov` | 0.9 | Optional, for Rust coverage (`cargo install cargo-llvm-cov`) |

```bash
git clone https://github.com/HappyRave/Relay.git
cd Relay
npm ci
cargo test --workspace      # first build takes a few minutes
npm test
```

Install `gh` (the GitHub CLI: `winget install GitHub.cli`) and sign in with `gh auth login`: pull requests are how changes reach `main`. A shell opened before installing it may need `C:\Program Files\GitHub CLI` added to its `PATH`.

**Never develop against your real data.** Set `RELAY_DATA_DIR` to a scratch folder (`$env:RELAY_DATA_DIR = "$env:TEMP\relay-dev"` in PowerShell) before `npm run tauri dev`. A fresh folder seeds the four sample macros.

## Repository map

```text
crates/relay-core/src/       pure logic, heavily tested
  model.rs  keys.rs          Event, Macro, Rect/Rgb; key labels, split_combo, key_for_char
  runlog.rs                  the run history: RunEntry, RunLog (newest 200), CheckLog
  steps.rs                   raw events → editor steps (click/drag/scroll/keys/type/wait/pixel_wait)
  edit.rs                    EditOp, apply, normalize, check_invariants
  format.rs                  .rly / JSON export, migrations (v0 → v1)
  playback.rs  timeline.rs   PlayClock, plan_times (humanize); durations
  session.rs                 the session state machine: Mode × Input → effects
  schedule.rs  triggers.rs   next scheduled run (DST-safe); PixelEdge, ProcessLaunchEdge
  image.rs                   finding an image on a capture (gray NCC, coarse to fine, scales 0.5–2); PNG/JPEG/DIB
  samples.rs  proptests.rs   the four design samples; property tests
crates/relay-platform/src/
  lib.rs  types.rs           the OS traits (InputHook, Screen, WindowQuery, Injector, Timer…), RawInput, HookConfig
  recorder.rs  keymap.rs     RawInput → Events; scan code ↔ W3C code
  processes.rs               running programs (sysinfo)
  windows/                   the Win32 backend: clipboard (and the snip), hook, inject, screen, text, timer, window
  stub.rs                    placeholder backend for non-Windows builds
src-tauri/src/
  lib.rs                     app setup, plugins, commands list, RELAY_DEVTOOLS_PORT, shutdown
  coordinator.rs             owns the session: commands in, effects out; F10 playhead; trigger admission
  engine.rs                  the playback engine (pure Engine + thread)
  finder.rs                  an image on the real screen (1:1 capture, Relay's window painted over); the trigger's Looker
  rec_thread.rs              recorder thread and hook watchdog
  commands.rs  ipc.rs        Tauri commands; the EngineMsg stream (queues errors until the UI subscribes)
  library.rs  history.rs     macros on disk + trash + import; undo/redo (in memory)
  run_history.rs             runs.json, the run history
  settings.rs  storage.rs    settings.json (field by field); data dir, atomic writes
  triggers.rs  hotkeys.rs    schedule/app-launch/pixel watchers; global hotkeys (RegisterHotKey)
  window_ctl.rs  tray.rs     placement (anchor, choose_monitor, layout), zoom, frame; tray menu
  logging.rs                 daily rolling log, flushed on quit
src/                         the Svelte UI
  lib/state/relay.svelte.ts  THE store (RelayStore); resetRelay() for tests
  lib/state/selection.ts     the step editor's selection following its step across edits
  lib/ipc/backend.ts         Backend interface: tauriBackend and the read-only browserBackend
  lib/ipc/bindings/          GENERATED from Rust by ts-rs (never edit by hand)
  lib/dev/sample-views.json  GENERATED browser fixture (by relay-core's tests)
  lib/fields.ts  format.ts  hotkeys.ts  image.ts  timeline/  preview/  platform/window.ts
  components/                Widget, CompactBar, ExportDialog, expanded/{Header,Preview,SidePanel,Transport,Timeline,tabs/*}, ui/*, shared/*
  test/                      fake-core.ts (+ fake-edit.ts, fake-core.test.ts), app.ts, setup.ts
e2e/                         end-to-end suites (*.e2e.test.mjs), harness.mjs, image-window.ps1, github-reporter.mjs, ci-diagnose.mjs
scripts/                     cdp.mjs (run JS in a running Relay), docs-screenshots.ps1 (regenerates docs/images)
docs/                        user-guide/, engineering/, images/
Design/                      the original prototype and design system (reference only)
.github/workflows/           ci.yml, release.yml
```

## Commands

| Command | What it does |
| --- | --- |
| `cargo test --workspace` | Every Rust test. **Also regenerates** `src/lib/ipc/bindings/*` and `src/lib/dev/sample-views.json`: commit them if they change (CI fails otherwise). |
| `cargo test -p relay-core` | The core only, in seconds |
| `cargo fmt --all` / `cargo fmt --all -- --check` | 120 columns (`rustfmt.toml`); CI checks it |
| `cargo clippy --workspace --all-targets -- -D warnings` | Zero warnings, as in CI |
| `npm test` | Vitest (store, backend, fake core, components) |
| `npm run check` | svelte-check: 0 errors **and 0 warnings** |
| `npm run test:coverage` | Vitest with coverage in `coverage/` (git-ignored) |
| `npx tauri build --debug --no-bundle` | `target/debug/relay.exe` **with the UI built in**, for the end-to-end tests |
| `npm run test:e2e` | The end-to-end suites against `target/debug/relay.exe` (or `RELAY_EXE`). Takes about 3 minutes. |
| `npx tauri build` | Release exe `target/release/relay.exe` and installer `target/release/bundle/nsis/Relay_X.Y.Z_x64-setup.exe` |
| `npm run verify` | What CI checks, locally: fmt, clippy, `cargo test`, `npm run check`, `npm test`, then the debug build and E2E (quit Relay first). About 10 minutes. |
| `npm run verify:quick` | The same without the build and E2E |
| `npm run tauri dev` | The app with hot reload (uses port 1420) |
| `npm run dev` | The UI alone in a browser (port 1420), with the samples and a simulated session. Read-only: edits say they need the app. |
| `cargo llvm-cov --workspace --summary-only` | Rust unit-test coverage |

Plain `cargo build -p relay` makes a debug exe that loads the UI from the dev server (`devUrl`), not from `dist/`. For a runnable debug exe use `npx tauri build --debug --no-bundle`, or `npm run build` then `cargo build -p relay --features tauri/custom-protocol`.

## Definition of done

For any change:

1. Tests for the new behavior, at the lowest layer that can express it (see [Testing](#testing)). New UI controls get a component test that checks the exact command sent.
2. `npm run verify:quick` passes (fmt, clippy, `cargo test`, `npm run check`, `npm test`); commit the bindings and fixture `cargo test` regenerates.
3. If the app, the UI or IPC changed: `npm run verify` instead, which also builds the debug exe and runs E2E (quit any running Relay first).
4. If the fake core's counterpart changed in Rust (commands, library, edits, history, hotkey rules, settings), update `src/test/fake-core.ts` / `fake-edit.ts` to match, and `fake-core.test.ts`.
5. Docs: update the user guide page for any behavior change and the engineering page for any design change; add a `CHANGELOG.md` entry under *Unreleased* for anything users notice.
6. Commit on a branch (Conventional Commits) and push, update the draft PR's description, then mark it ready and get its checks green (see [Git, CI and GitHub](#git-ci-and-github)).

## Rules the code follows

These came out of two full reviews (m12, m14). Breaking one has caused real bugs before.

**Architecture**
- **`relay-core` owns the model and every edit.** The UI never edits steps itself; it sends an `EditOp` and shows the returned `MacroView`. Pure logic goes in `relay-core`, takes time as an argument, and gets a test.
- **Extract decisions from threads.** Threads only sleep and feed pure types: `Engine::advance(now)`, `ScheduleWatch`/`LaunchWatch`/`PixelWatch` (triggers.rs), `window_ctl::layout`/`choose_monitor`, coordinator's `admit`/`playhead_after`/`is_current`/`counts_as_run`. Test those, not the threads.
- **Never hold a lock while calling into the Tauri main thread** (window, tray, hotkey calls): the main thread may need that lock → deadlock. Hotkeys (un)register only on the main thread via `hotkeys::set_active`/`refresh`/`refresh_and_wait`.
- **Locks are `parking_lot`** (no poisoning).
- **Every mode change goes through `coordinator::set_mode`**, which also updates the shared `SessionMode` the commands read (including after a panic, so nothing stays "busy").
- **Playbacks carry a generation**; a late `EngineDone` from an older one is ignored. `Finished` is emitted only by the coordinator.
- **The F10 playhead is Rust's (`idle_playhead`)** and mirrors the UI's after each finish: Stop/key/kill/error → 0, Completed → end (F10 restarts), pixel timeout → the check's time.
- **Trigger admission:** paused → ignore; deleted macro → ignore; busy → notice *"Skipped “X”: Relay was busy"*; locked screen → silent skip. A delayed app-launch run re-checks the trigger when the delay ends.
- **Saving triggers waits (≤ 1 s) for the app watcher's baseline**, so a program started right after is a launch; a program already running then isn't.
- **Hook callbacks stay minimal:** no allocation, locks, logging or blocking. The hook beats `HookConfig.mouse_pulse` (an atomic) on every mouse event it sees, filtered or not, so the watchdog doesn't false-alarm.
- **Anything that presses a key or button releases it on every path** (stop, loop end, seek, drop, panic). Release builds use `panic = unwind` on purpose so the engine's `Drop` still releases keys.

**Data and files**
- **A failed save never loses data:** the change stays in memory, the command still succeeds, and the user gets an `error` message (`report_unsaved`). This holds for edits, triggers, duplicate, delete, restore.
- **A file that exists but can't be read is never overwritten** (`library.json` and `settings.json`: locked by another program, say). An invalid `library.json` is set aside as `library.json.bad`; an invalid value in `settings.json` resets only that setting and the file is set aside as `settings.json.bad`.
- **Writes are atomic** (`storage::write_atomic`: temp file + rename).
- **Edits are refused while a session runs** (`busy`), in Rust and in the UI (`canEdit`). Playback options (speed…) may change mid-playback.
- **Logs say what happened, never what the user typed.**
- `.rly` holds portable content only; triggers, run counts and the trash live in machine-local `library.json`. Formats: [file-formats.md](docs/engineering/file-formats.md).

**IPC and UI**
- A type crossing IPC derives `TS` with `#[ts(export)]`; the bindings are generated, never hand-edited.
- **Components talk to the `relay` store only**, never to `backend` or `invoke`.
- **Every async result is guarded** by macro id and `viewSeq` (`apply()`), so a slow response can't land on another macro or overwrite a newer state. Settings, trigger and trigger-pause saves carry sequence numbers too.
- **Undo toasts belong to their macro** (`undoes`) and are withdrawn by another edit, a rename, or opening another macro. **Error toasts stay until dismissed**; info toasts time out (5 s, 8 s with an action).
- Svelte 5 runes only. Colors and spacing from `var(--…)` tokens (`src/styles/tokens.css`); corners stay square.
- User-facing text: short plain sentences saying what happened and what to do; sentence case for labels.

## Testing

Full description: [docs/engineering/testing.md](docs/engineering/testing.md). The three layers:

1. **Rust unit, property and snapshot tests**, next to the code. Property tests in `crates/relay-core/src/proptests.rs` generate random recordings and edit sequences; failures are saved in `crates/relay-core/proptest-regressions/` (commit that file). Snapshots with `insta` (`cargo insta review`).
2. **Vitest in jsdom** with Testing Library. The UI runs its **real** `tauriBackend`; Tauri's `mockIPC` routes every `invoke` to **`src/test/fake-core.ts`, which must answer exactly like the Rust side** (library rules, edit semantics incl. event renumbering, history, hotkey refusals with Rust's messages, `busy`, argument types). A fake that drifts from Rust makes tests pass on behavior the app doesn't have; that happened before and the m14 audit fixed it. Helpers: `freshStore()`, `settle()`, `core.fail/on/hold/held/emit/emitLater/saveError/mode/files`. Compare against literal expected values, never against the fake's own post-call state.
3. **End to end** (`e2e/`, node:test): the harness starts the **real built app** on a scratch data folder with `RELAY_DEVTOOLS_PORT`, drives the page over the Chrome DevTools Protocol (clicks by accessible name, reads `window.__relay`, invokes commands where the UI would open a native dialog), and checks the files Relay writes.

**Hard rule: never write code that synthesizes OS keyboard or mouse input for testing.** No fake OS input layers, SendInput helpers or input-generating companion apps. The E2E tests only use the page's DOM and Relay's commands. Macros they play contain only waits and pixel checks (`waitingMacro()`), and triggers fire on real events: `ping.exe` launching, a scheduled minute, and a patch of Relay's own window changing color (for the pixel trigger). Playback injection is covered by the engine's unit tests with a recording injector. What needs real input (recording actual clicks/keys, Esc, stop on key press, pressing a macro hotkey, the kill switch, the tray menu) stays on the manual checklist in testing.md.

**E2E harness notes** (`e2e/harness.mjs`):
- Only one Relay can run (single-instance plugin): the harness refuses to start if one is running. Ask the user to quit theirs; don't kill it.
- `page.click(name)` needs exactly one **visible** control with that exact accessible name; pass `{ nth }` for repeated names (the steps' ×) or `{ within: ".header" }` to scope (Undo is both in the header and in toasts).
- Library Duplicate/Delete only show on hover or focus: use `page.rowAction("Delete", "Macro name")`, which focuses the row first.
- `page.tab("Steps")` and `page.open(id)` wait for what they did; `page.reset()` runs after every test (stops any session, expands the widget, closes the step editor, dismisses the toast); `page.playTimed()` times a playback inside the page.
- **After a file changes on disk, wait for the UI too**: Rust writes the file slightly before the UI gets its answer (the editing suite's `saved()` compares `view.modified_at` with the file's).
- Before a keyboard shortcut or a click, wait until the UI state allows it (e.g. `canUndo` before Ctrl+Z).
- Watch "nothing happened" assertions for their whole window (`staysIdle`), and check run counts in `library.json`, not a single sample.
- A launch is only seen if the app watcher has seen the program *not* running first (it polls every 2 s): the `ping()` helper waits a poll.
- `RELAY_DEVTOOLS_PORT` also disables WebView2's background throttling, since the CI runner's window isn't in front and throttled timers skewed timings.

## Documentation

- The user guide is the spec: behavior changes update it in the same branch. The engineering guide explains how and why.
- Style: GitHub-flavored Markdown with good navigation (a contents list per page, prev/next links at the bottom, tables, `> [!TIP]`/`> [!NOTE]` callouts, `<kbd>` for keys, mermaid diagrams where they help). Short, plain, active sentences.
- `scripts/docs-screenshots.ps1` regenerates `docs/images/` from the real app. Rerun it after visible UI changes. It only plays inside a stretched pause, so it never clicks on the desktop.
- `CHANGELOG.md`: entries go under *Unreleased*; the release renames it to `## vX.Y.Z: <title>`.

## Git, CI and GitHub

`main` only changes through pull requests that pass CI. CI minutes are limited (2,000 a month, and Windows minutes count double), so everyday checking happens locally and the expensive CI job runs once per PR, when it's ready.

- **Branches:** short-lived, off `main`. Milestones `mN-short-name` (next is `m19-…`), fixes `fix-…`, docs `docs-…`, CI and tooling `ci-…`, releases `release-X.Y.Z`.
- **Commits:** small, [Conventional Commits](https://www.conventionalcommits.org/) (`feat(recorder): …`, `fix(engine): …`, `test(e2e): …`, `docs: …`, `ci: …`, `chore: …`), with a body explaining why when it isn't obvious. Push as often as you like: pushing a branch runs nothing.
- **The pull request is the milestone's workspace.** Open it as a **draft** when the branch starts (`gh pr create --draft --base main`), with the plan and progress in its description, kept up to date. That's where the next session (or contributor) picks up. Drafts only run the quick CI job.
- **Ready means checked:** run `npm run verify` locally, then `gh pr ready`. That runs the full CI once; every later push to a ready PR runs it again, so push fixes in one go. Put a PR back to draft (`gh pr ready --undo`) to keep working on it.
- **Merging:** only merge commits (squash and rebase are off), so history keeps each branch as a `--no-ff` merge: `gh pr merge <n> --merge --subject "Merge mN-…: <summary>"`. Merge when the checks are green, but **wait for the maintainer's go** when they're checking something by hand. Since 1.0, versions are tagged only at release.
- **`main` is protected** by a repository ruleset (*Settings → Rules*): a PR is required, and so are the `quick`, `windows` and `msrv` checks; no force-push or deletion; nobody bypasses it. No approving review is required while there's one maintainer.
- **Authorized:** pushing branches to `HappyRave/Relay`, and opening, updating, marking ready and merging PRs (following the rule above). Never push to `main` directly; the ruleset refuses it anyway.
- **CI** (`.github/workflows/ci.yml`), on pull requests to `main`:
  - **quick** (Linux, every push, drafts too): fmt, tests and clippy for `relay-core` and `relay-platform` (they must stay portable), `npm run check`, `npm test`. It also decides whether the PR changes more than docs (`docs/**`, `*.md`).
  - **windows** (ready PRs that change code, after `quick` passes): `cargo test`, generated files up to date, clippy, `npx tauri build`, upload the installer, then the E2E suites against the release build. If E2E fails, `e2e/ci-diagnose.mjs` reports WebView2 details as annotations.
  - **msrv** (same condition): `cargo check` with Rust 1.95.
  - A skipped job counts as a passed check, so docs-only PRs merge after `quick`. A newer push cancels the PR's run in progress. *Actions → CI → Run workflow* runs everything on any branch by hand.
- **Reading CI:** `gh pr checks <n>`, and `gh run view <run id> --log-failed` for a failure. `e2e/github-reporter.mjs` turns each E2E failure (and each suite that failed to start) into an annotation with its message. Without `gh`, the public API works too: `https://api.github.com/repos/HappyRave/Relay/actions/runs?head_sha=<sha>`, then `/actions/runs/<id>/jobs`, then `/check-runs/<job id>/annotations`.
- A full run takes about 20–25 minutes (`quick` first, then the Windows job builds the release and runs E2E). Its Rust cache starts cold for each PR, since nothing runs on `main` to share one.
- **When contributors join:** require 1 approving review and add a `CODEOWNERS`; set *Settings → Actions → Fork pull request workflows* to require approval for all outside collaborators; and turn on *Require branches to be up to date before merging* in the ruleset, so `main` is tested with each PR on top of the latest code (merge queues need an organization-owned repo).

## Releasing

1. On a `release-X.Y.Z` branch: bump the version in **`Cargo.toml`** (workspace), **`package.json`** and **`src-tauri/tauri.conf.json`**; run `npm install --package-lock-only` and `cargo check` to refresh the lockfiles; rename *Unreleased* in `CHANGELOG.md` to `## vX.Y.Z: <title>`; tick the roadmap line in `README.md` if relevant.
2. Run `npm run verify`, commit (`chore(release): X.Y.Z`), and open a PR, ready (`gh pr create --base main`). Once green, merge it (`--subject "Release X.Y.Z"`), then `git checkout main && git pull`, tag `git tag -a vX.Y.Z -m "Relay X.Y.Z"` and push the tag.
3. `release.yml` runs on the tag: tests, builds with tauri-action, and creates a **draft** release with `Relay_X.Y.Z_x64-setup.exe` and `Relay_X.Y.Z_x64-portable.exe` (the portable file is `target/release/relay.exe`, attached by a separate step).
4. Publish with `gh`: `gh release edit vX.Y.Z --draft=false --latest --title "Relay vX.Y.Z" --notes-file <notes.md>` (`gh release view vX.Y.Z` shows the draft first).
   - Write the notes in the style of the previous releases (`gh release view v1.3.0`): *What's new* bullets, a link to the changelog and user guide, then *Downloads* (both files) and the SmartScreen note, since the builds aren't code-signed.
   - After publishing, check that `gh release view vX.Y.Z` lists both assets, that `gh release list` marks it Latest, and that both download URLs respond.
   - In the web UI instead: open the draft from the Releases list (`/releases/edit/untagged-…`). **Don't use `/releases/edit/vX.Y.Z`**: for a tag without a published release, that's a *new release* form, and publishing it creates a second, empty release (that happened with v1.2.0).
   - Deleting a release has to be done by the maintainer.
5. `release.yml` also has a manual `workflow_dispatch` with a `tag` input that rebuilds a tag and (re)attaches its portable exe.

## Gotchas

**Windows and shells**
- The agent shell was Git Bash on Windows, with PowerShell available. Python heredocs that contain `\s`, `\x..` or quotes mangle easily: a literal NUL byte once ended up in a source file. For multi-line edits, prefer an editor tool or a script file over inline heredocs, and check the result with `node --check` or the compiler.
- Git warns about CRLF → LF conversions on most commits. That's harmless: the repo normalizes to LF.
- PowerShell 5 mangles non-ASCII characters in inline scripts; in JS strings use `\u` escapes.
- `cargo llvm-cov` runs leave `*.profraw` files; they're git-ignored now (three were once committed by mistake).
- A coverage-instrumented app exits without writing its profile, so merged unit + E2E coverage isn't possible; report them separately.
- With Node 25.7, `npm run test:e2e` fails with "Cannot find module …e2e": that Node doesn't take the folder argument. Node 26 is fine; otherwise pass the files: `node --test --test-concurrency=1 --test-timeout=120000 e2e/*.e2e.test.mjs`.

**WebView2 and the app**
- The GitHub runner's WebView2 ignores `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS` and its registry override (`HKCU\Software\Policies\Microsoft\Edge\WebView2\AdditionalBrowserArguments`). Relay passes the DevTools port itself when `RELAY_DEVTOOLS_PORT` is set (`src-tauri/src/lib.rs`, `context()`), which also keeps wry's default `--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection`.
- Relay's WebView2 user data folder is `%LOCALAPPDATA%\com.happyrave.relay\EBWebView`, shared by every instance.
- The UI store is on `window.__relay` and commands on `window.__TAURI_INTERNALS__.invoke` (for DevTools, `scripts/cdp.mjs` and the E2E tests). Synthetic DOM events need `{ bubbles: true }`, because Svelte 5 delegates events.
- `npm run dev` and `npm run tauri dev` share port 1420.
- jsdom has no layout: component tests that seek by pointer stub `getBoundingClientRect`. `src/test/setup.ts` fills in `<dialog>`, `ResizeObserver` and animation frames stamped with `performance.now()` (jsdom's own use another time origin).
- `Screen::pixel` (GetPixel) costs about 10 ms, because it waits for the compositor.
- `Screen::double_click` returns the **whole** `SM_CXDOUBLECLK` width; step grouping takes half of it on each side.
- Tauri pins the `windows` crate: the app uses 0.61, while `relay-platform` uses 0.62.

**Behavior worth knowing** (all tested and documented)
- Inserts (+ Wait, + Pixel check) go just after the step under the playhead, and push everything after them later.
- Every cursor move belongs to a step: the click or drag it happened during, or a MOVE step (the run between two other events). MOVE steps are grouped after everything else, so they never break a double click or a Ctrl-click, and humanize ignores them (their samples follow the step before). `pause` is idle time only. Smooth and Straighten keep every sample's time and both ends, because playback replays each sample without interpolating.
- The sample macros have a MOVE before each click: the invoice sample has 18 steps, 12 of them the design's.
- A trailing `+` in a hotkey is the plus key ("Ctrl + +"). "Ctrl + + K" is refused. Shift alone with a key that types (Shift + A) is refused. So is a hotkey ending in a modifier.
- A pixel trigger fires when the pixel matches twice in a row after two non-matching samples, and re-arms only after two non-matches. The image trigger works the same way, a sample being "the image is on screen".
- Image searches never look inside Relay's own window: what shows of it is painted flat in the capture (`shown_rect` minus `covering`: a window in front of Relay is still searched), so the editor's thumbnail is never found. `WDA_EXCLUDEFROMCAPTURE` isn't enough for this: on multiple monitors it can still be applying when the capture is taken. The E2E tests show their image in a separate PowerShell window for this reason.
- A Find image step that finds its image clicks it: E2E only plays one whose image is absent.
- `.rly` is written as v2 only when the macro has a `find_image` event, so other macros still open in older Relays.
- relay-core is built at `opt-level = 3` in dev too: unoptimized, an image search takes seconds instead of ~25 ms.
- App launches are detected by process name: a second instance of a program that's already running isn't a launch (`chrome.exe` starts many processes).
- A schedule run found up to 2 minutes late (after waking) still runs; later than that, it's skipped. A clock set back never repeats a run.

## Known limitations and open items

Not fixed yet; each is a candidate task:

- **One toast at a time:** several startup errors delivered together show only the last one. Stacking or combining them would be a `src/` change.
- **Unsaved trash after a restart:** a macro whose file never saved, then trashed, lives in memory only and can't be restored after a restart.
- **Undo history memory:** it keeps up to 100 full snapshots per macro, which could be large for very long recordings.
- **Crash recovery isn't unit tested:** the coordinator's recovery path (busy state cleared after a panic) needs Tauri's `test` feature (`MockRuntime`), which isn't enabled.
- **Zoom margin:** `zoom_for_work_area` keeps one 16 px margin vertically, so on a short work area the expanded widget sits 8 px from the top and bottom.
- **Real-input paths** are on the manual checklist only (see [Testing](#testing)).
- **Dead keys:** a dead key alone records as a KEYS step, and the following letter records unaccented ("e", not "ê"). It's display only, and pinned by a test.
- **Roadmap:** AHK and `.exe` export, code signing, monitor remapping, other OS backends.

## Working with the maintainer

- They want work done thoroughly and **verified**: tests at every layer, CI green, and docs updated in the same change. Report results as they are, including failures and what wasn't done.
- **You own the versioning here**: commit, push and keep the history clean without being asked, following [Git, CI and GitHub](#git-ci-and-github). They like regular, well-described commits and visible milestones (branches merged with `--no-ff`). Merging into `main` still waits for their go when they're checking something by hand.
- Documentation quality and GitHub UX matter to them (a user guide and an engineering guide, both polished).
- Ask before anything outward-facing that can't be undone (deleting releases, force-pushing, publishing something they didn't ask for). A release they asked for is authorized to publish.
- Prefer to fix root causes rather than loosening tests. When a test fails, first decide whether the test or the app is wrong, using the user guide as the spec.

**How to respond**
- **Precise and concise.** Answer the question in the first sentence, then only what's genuinely needed.
- No preamble, no restating the question, no recap of what the diff already shows.
- No caveats or alternatives nobody asked for; one line if there's a real risk.
- Short prose or a tight list. Headings only when the answer really is that big.
- Report failures with the actual tool or compiler output, not a paraphrase.
- Say what was verified and how. A green build proves it builds: don't imply you checked behavior you didn't run (real mouse and keyboard input is theirs to try).

**Ask rather than guess**
- Any choice beyond the cosmetic (a design fork, a name that will spread, which existing system to hook into, a behavior the user guide doesn't settle) comes as a one-line question **before** building it. Semi-important counts: an answer is cheaper than unpicking a wrong 300-line change.
- Also ask when they'd know instantly and you'd need a long search to find out. One question beats twenty tool calls.

**Scope and simplicity**
- The simplest, most direct design that solves the actual problem. No overengineering, no elaborate heuristics when a clean setup solves it outright.
- **Change only what was asked.** No drive-by refactors, renames, reformatting or cleanups, and no rewriting logic that works. Spotted something worth fixing outside the scope? Say it in one line and leave the code alone.
- Touch the fewest files that do the job, and reuse what exists before adding something new.

**Code style**
- **Code must look human-made.** No comments narrating what changed or why an earlier approach was wrong; no rationale essays in the source. Comment what isn't obvious from the code, briefly. The short doc comments on public items are the house style: keep to that density, don't pad it.
- The formatters and lint configs are the authority (`rustfmt.toml`, clippy, svelte-check). Match the surrounding file's idiom.
- No summary or notes markdown files unless asked (the docs and `CHANGELOG.md` entries the [Definition of done](#definition-of-done) asks for are part of the work, not extra).
