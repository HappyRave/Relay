# Recipes

Step-by-step guides for the changes Relay gets most often. Each lists the files to touch, in order, and the tests at each layer. Every recipe ends with the [definition of done](#every-change-ends-the-same-way).

- [Add an edit](#add-an-edit)
- [Add a step type](#add-a-step-type)
- [Add a command](#add-a-command)
- [Add a trigger kind](#add-a-trigger-kind)
- [Add a setting](#add-a-setting)
- [Change the file format](#change-the-file-format)
- [Add an OS backend](#add-an-os-backend)
- [Every change ends the same way](#every-change-ends-the-same-way)

## Add an edit

A new way to change a macro's steps. Example: `eb1c2fe` (several steps at once).

1. **relay-core** [`edit.rs`](../../crates/relay-core/src/edit.rs): add the `EditOp` variant and its arm in `apply`. Keep presses balanced, and retime monotonically so order survives. Refuse bad input with an `EditError` whose message the user can read.
2. **Tests** in `edit.rs`, and add the op to the edit generator in [`proptests.rs`](../../crates/relay-core/src/proptests.rs), so random sequences check the invariants and the undo splice.
3. **Undo** needs nothing new: [`history.rs`](../../src-tauri/src/history.rs) records any op's splice, a `Batch` included. Only `Rename` is special. Add a test there if the op could break that.
4. **Fake core**: the same semantics in [`fake-edit.ts`](../../src/test/fake-edit.ts), event renumbering included, and the op's arguments in `EDIT_OPS` in [`fake-core.ts`](../../src/test/fake-core.ts), so the fake refuses bad arguments as serde does. Pin it in `fake-core.test.ts`.
5. **Store** (if the UI needs a new action) in [`relay.svelte.ts`](../../src/lib/state/relay.svelte.ts): call `edit(op)`, which refuses edits while a session runs and guards the answer through `apply`.
6. **UI** (if there's a new control): the control, and a component test that checks the exact op sent.
7. **Spec**: a rule in [Editing](../spec/editing.md).

## Add a step type

A new kind of inserted event, like Wait, Text or Find image. Example: the `m20-text-placeholders` merge (`bd267cb`).

1. **Model** [`model.rs`](../../crates/relay-core/src/model.rs): an `Event` variant. If it has a duration, `Event::end()` must include it, so `normalize` keeps other events out of it.
2. **Grouping** [`steps.rs`](../../crates/relay-core/src/steps.rs): a `StepKind`, one step per event.
3. **Edits** [`edit.rs`](../../crates/relay-core/src/edit.rs): an `Insert…` op (snapping past the step at the playhead) and an `Update…` op; add both to the property tests.
4. **Format** [`format.rs`](../../crates/relay-core/src/format.rs): the new event needs a [new version](#change-the-file-format), chosen in `Envelope::new` only for macros that have one.
5. **Platform** (if playback needs a new OS ability, as typing text did): a trait method in [`relay-platform/src/lib.rs`](../../crates/relay-platform/src/lib.rs), implemented in `windows/` and in `stub.rs`.
6. **Playback** [`relay-playback/src/engine.rs`](../../crates/relay-playback/src/engine.rs): what `advance` does with it, tested with the fake clock and recording injector. Anything it presses is released on every path.
7. **Exports** [`ahk.rs`](../../crates/relay-core/src/ahk.rs): a block for it, snapshot updated. Exported programs come free with the engine.
8. **The app**: any command the inspector needs (see [Add a command](#add-a-command)).
9. **Frontend**:
   - the fake core: `fake-edit.ts`, and the new ops in `EDIT_OPS` in `fake-core.ts`;
   - the store's insert and update actions in `relay.svelte.ts`;
   - the row's text in [`rows.ts`](../../src/lib/editor/rows.ts) and its category in [`categories.ts`](../../src/lib/categories.ts);
   - the timeline in [`lanes.ts`](../../src/lib/timeline/lanes.ts) and `ViewTimeline.svelte`;
   - its fields in `StepPane.svelte`, and its item in the **Add** menu in `StepList.svelte`.

   Component tests for each.
10. **E2E** in `editing.e2e.test.mjs`: insert, edit, save, reopen. Don't play it if it would act on the desktop.
11. **Spec**: [Steps](../spec/steps.md), and a page of its own if it has its own rules.

## Add a command

1. **Rust** [`commands.rs`](../../src-tauri/src/commands.rs): a `#[tauri::command]` returning `Result<T>` (the module's alias over `IpcError`). Use `#[tauri::command(async)]` if it writes files. Start with `when_idle(&app)?` if it edits, so it's refused with `busy` during a session.
2. **Register it** in `generate_handler![…]` in [`lib.rs`](../../src-tauri/src/lib.rs).
3. **Types**: derive `Serialize` and `TS` with `#[ts(export)]` on new types, and run `cargo test` to generate the bindings.
4. **Backend** [`backend.ts`](../../src/lib/ipc/backend.ts): the method on `Backend`, the Tauri implementation, and a browser one (a simulation, or a rejection with `code: "unavailable"`). `backend.test.ts` checks the argument names Rust expects.
5. **Fake core**: answer it exactly as Rust does, errors included, and pin that in `fake-core.test.ts`.
6. **Store**: call it from an action, never from a component. Guard the answer by macro id (and a sequence number if answers can race).

## Add a trigger kind

Examples: `96e2b02`, the interval trigger (*Every…*), for one that only needs the clock; `a64e2bc`, the image trigger, for one that polls the screen or the OS.

1. **Model** [`relay-core/src/triggers.rs`](../../crates/relay-core/src/triggers.rs): a trigger type modelled on `AppLaunchTrigger` (serde, `#[ts(export)]`, a `Default`), a field for it on `MacroTriggers` (the struct is `#[serde(default)]`, so older `library.json` files load), its `TriggerOn` value and `also_on`. Add its `RunSource` in [`session.rs`](../../crates/relay-core/src/session.rs).
2. **The firing rule**, if it fires on a change (something appears): an edge detector beside `PixelEdge` and `ProcessLaunchEdge` in relay-core, with tests.
3. **Platform**, if polling needs a new OS ability (listing windows, say): a trait method in [`relay-platform/src/lib.rs`](../../crates/relay-platform/src/lib.rs), implemented in `windows/` and in `stub.rs`.
4. **The watch** [`src-tauri/src/triggers.rs`](../../src-tauri/src/triggers.rs): a pure `…Watch` type whose `tick` returns the macros to fire, unit-tested with fixed inputs (on time, late, after sleep, busy, changed). Then a thread that sleeps, reads the triggers with `snapshot(&app)`, calls `tick` and `fire(&app, id, RunSource::…)`; start it in `spawn`.
5. **Baseline**, if the thing can already be there when the trigger is saved (a program running, a window open): make `set_triggers` wait for the watch's first look, as it waits for the app watcher (`triggers_changed`), so only what appears afterwards fires.
6. **Admission** [`coordinator.rs`](../../src-tauri/src/coordinator.rs): `admit` already handles paused, deleted, busy and locked. Change it only if the new kind needs different treatment (the interval trigger did: an interval meeting a busy Relay is ignored quietly), and test that.
7. **Frontend**: run `cargo test` first; `npm run check` then lists most places to update (the `Record<TriggerKind, …>` maps: `TRIGGER_NAME` and the card's `ICON`); add the kind to `TRIGGER_KINDS` by hand. Then:
   - the card in `MacroPane.svelte`, and a component test for each of its fields. A card that lists something from the OS needs a command ([Add a command](#add-a-command));
   - the text in [`editor/triggers.ts`](../../src/lib/editor/triggers.ts) and [`runs.ts`](../../src/lib/runs.ts);
   - the defaults, which have three copies that must match Rust's `Default`: `editor/triggers.ts`, the browser backend in `backend.ts`, and the fake core's `defaultTriggers` (with `withDefaults`, `alsoOn` and the `TRIGGERS` spec).
8. **E2E** in `triggers.e2e.test.mjs`, firing on a real event, never on synthesized input: a program starting, or a separate PowerShell window like [`image-window.ps1`](../../e2e/image-window.ps1).
9. **Docs**: new rules in [Triggers](../spec/triggers.md); the new thread in [Architecture → Threads](architecture.md#threads) and [The app → Triggers](app.md#triggers).

## Add a setting

1. **Rust** [`settings.rs`](../../src-tauri/src/settings.rs): the field and its default. Parsing is generic, field by field, so there's no parsing code to write; test only a type with its own validation (like `TextSize`).
2. **Decide what existing users get.** A `settings.json` written before the field existed is filled in from `Settings::upgraded()`, a new folder from `Settings::default()`. `upgraded()` copies the defaults unless you set the field there.
3. **Apply it** in `update_settings` only if Rust must react the moment it changes (the window, the look, the hotkeys, the player). A setting read when it's needed needs nothing here.
4. **Use it** where it acts: in the UI, `relay.settings.<field>` in the store action or component; in Rust, read `SettingsStore` and release the lock before any window, tray or hotkey call. Test that behavior, not just the saving.
5. **Frontend**: run `cargo test` to regenerate `Settings.ts`, and `npm run check` points at `DEFAULT_SETTINGS` in [`defaults.ts`](../../src/lib/defaults.ts): give it Rust's default (the browser backend and the fake core copy it). Add the field to the fake core's `SETTINGS` spec (it ignores unlisted fields, so nothing fails if you forget). Then the control, in `SettingsPage` or wherever the user changes it, with a test that checks the saved settings.
6. **E2E** in `settings.e2e.test.mjs` if it changes the native window or survives a restart.
7. **Spec**: [Settings](../spec/settings.md).

## Change the file format

Only when the event model changes, and only for macros that use the new thing:

1. Bump `VERSION` in [`format.rs`](../../crates/relay-core/src/format.rs), and choose the version a macro is written with in `Envelope::new`, so a macro without the new thing keeps the older version.
2. Add an arm to `migrate` for the previous version: `Ok(())` if the new version only adds an event, else a `migrate_vN` working on `serde_json::Value`.
3. Add a test that loads the old format, and a snapshot if a migration rewrites it.
4. Update [File formats](file-formats.md) and [Export and import](../spec/export-import.md).

## Add an OS backend

1. A module beside `windows/` and `stub.rs` in [`relay-platform`](../../crates/relay-platform/src), implementing each trait, with the `mod` and `platform()` `#[cfg]`s narrowed to match (the stub covers every non-Windows target today). See the [table of APIs](platform.md#other-backends).
2. The recorder, key map and grouping stay as they are. Scan codes are Windows' set 1: map the OS's codes to W3C codes in the backend.
3. The app's Windows-only parts (`window_ctl`, `look`, `tray`) and relay-player's Win32 shell need versions of their own.
4. Add the target to CI.

## Every change ends the same way

- [ ] Tests at the lowest layer that can express the behavior; new UI controls checked by the command they send
- [ ] The fake core changed with Rust, and `fake-core.test.ts` pins it
- [ ] `npm run verify:quick` passes, and the regenerated bindings and fixtures are committed
- [ ] `npm run verify` if the app, the UI or IPC changed
- [ ] The spec's rules updated; the user guide if what users do changed; this guide if the design changed
- [ ] A `CHANGELOG.md` entry under *Unreleased* for anything users notice

---

<p align="center"><a href="testing.md">← Testing, CI and releases</a> · <a href="README.md">Engineering guide</a></p>
