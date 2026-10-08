# Before merging and releasing 1.5.0

A working checklist for m21–m23 and the 1.5.0 release. Delete this file on the `release-1.5.0` branch, once everything is ticked.

## Where things stand

| PR | What | CI | State |
| --- | --- | --- | --- |
| [#9](https://github.com/HappyRave/Relay/pull/9) m21 | Data files that drive repeats | Full run green (E2E included) | Ready, can merge |
| [#10](https://github.com/HappyRave/Relay/pull/10) m22 | AutoHotkey v2 export | Quick only | Draft, on top of m21 |
| [#11](https://github.com/HappyRave/Relay/pull/11) m23 | Known limitations | Quick only | Draft, on top of m22 (this file is here) |

Each PR's description has its decisions, the smaller choices made along the way, and what was verified.

## Checks by hand

Real input and a real desktop, which the tests don't cover.

- [ ] **Text steps (m20):** `{date}`, `{time}`, `{n}`, a new line and `{clipboard}` (several lines, accents, emoji) type right into Notepad, a browser and an Office app, on an AZERTY layout too; a long clipboard pushes the next click back.
- [ ] **Data files (m21):**
  - [ ] a CSV saved by Excel ("CSV" and "CSV UTF-8", with accents) types each row's `{col:…}` into Notepad, once per row
  - [ ] Choose… and Remove in Settings → Playback
  - [ ] the file changed in Excel between two runs
  - [ ] an exported `.exe` types the rows it was exported with
- [ ] **AutoHotkey export (m22):** run a sample macro's script with AutoHotkey v2 (CI only checks that it loads):
  - [ ] clicks, a shortcut, typed text, a Text step with `{date}`, a pixel check, a Find image step (the riskiest: ImageSearch only finds the picture at its size)
  - [ ] Esc and Ctrl+Alt+End stop it
  - [ ] `Speed` and `Repeat` at the top change it
- [ ] **m23:**
  - [ ] several startup errors stack
  - [ ] undo and redo on a long recording behave as before
  - [ ] the app still looks and scales right on your monitors (`relay.exe` now gets its manifest from `build.rs`; checked present, not launched)
- [ ] **Choices to review** (in the PR descriptions): a Text step typing a column the file lacks is still saved; Duplicate keeps the data file; AutoHotkey drags move at speed 5, MOVE steps jump to their end.

## Deferred

- [ ] `npm run verify` locally (or rely on each PR's full CI)
- [ ] `scripts/docs-screenshots.ps1`: the Settings tab's Data file row, the Export dialog with AutoHotkey

## Merging (in order)

- [ ] Merge m21 (#9)
- [ ] m22 (#10): merge `main` in, mark ready, full CI green (first run of the AutoHotkey syntax check and m22's E2E test), merge
- [ ] m23 (#11): merge `main` in, mark ready, full CI green (its E2E changes haven't run yet), merge

## Release 1.5.0

- [ ] `release-1.5.0` branch: bump `Cargo.toml`, `package.json`, `src-tauri/tauri.conf.json`; `npm install --package-lock-only` and `cargo check`; *Unreleased* → `## v1.5.0: <title>` in `CHANGELOG.md`; tick the 1.5 and AutoHotkey lines of the README roadmap; delete this file
- [ ] `npm run verify`, PR, green, merge (`--subject "Release 1.5.0"`)
- [ ] Tag `v1.5.0` on `main` and push it; `release.yml` makes the draft release
- [ ] Publish the notes (style of `gh release view v1.4.0`), check both assets, Latest, and the download URLs
