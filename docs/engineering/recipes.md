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
3. **Undo** needs nothing new unless the op is special (a `Batch`, a rename): [`history.rs`](../../src-tauri/src/history.rs) records any op's splice.
4. **Fake core** [`fake-edit.ts`](../../src/test/fake-edit.ts): the same semantics, event renumbering included; pin it in `fake-core.test.ts`.
5. **Store** [`relay.svelte.ts`](../../src/lib/state/relay.svelte.ts): an action that calls `edit(op)`. It's guarded by `apply` and `canEdit` for free.
6. **UI**: the control, and a component test that checks the exact op sent.
7. **Spec**: a rule in [Editing](../spec/editing.md).

## Add a step type

A new kind of inserted event, like Wait, Text or Find image. Example: the `m20-text-placeholders` merge (`bd267cb`).

1. **Model** [`model.rs`](../../crates/relay-core/src/model.rs): an `Event` variant. If it has a duration, `Event::end()` must include it, so `normalize` keeps other events out of it.
2. **Grouping** [`steps.rs`](../../crates/relay-core/src/steps.rs): a `StepKind`, one step per event.
3. **Edits** [`edit.rs`](../../crates/relay-core/src/edit.rs): an `Insert…` op (snapping past the step at the playhead) and an `Update…` op; add both to the property tests.
4. **Format** [`format.rs`](../../crates/relay-core/src/format.rs): the new event needs a [new version](#change-the-file-format), written only for macros that have one.
5. **Playback** [`relay-playback/src/engine.rs`](../../crates/relay-playback/src/engine.rs): what `advance` does with it, tested with the fake clock and recording injector. Anything it presses is released on every path.
6. **Exports** [`ahk.rs`](../../crates/relay-core/src/ahk.rs) (a block for it, snapshot updated); exported programs come free with the engine.
7. **The app**: any command the inspector needs (see [Add a command](#add-a-command)).
8. **Frontend**: the fake core (`fake-edit.ts`), the row text ([`rows.ts`](../../src/lib/editor/rows.ts)), its category ([`categories.ts`](../../src/lib/categories.ts)), the timeline lane, the inspector's fields in `StepPane`, the **Add** menu. Component tests for each.
9. **E2E** in `editing.e2e.test.mjs`: insert, edit, save, reopen. Don't play it if it would act on the desktop.
10. **Spec**: [Steps](../spec/steps.md) and a page of its own if it has its own rules.

## Add a command

1. **Rust** [`commands.rs`](../../src-tauri/src/commands.rs): a `#[tauri::command]` returning `Result<T>`. Make it `async` if it writes files. Refuse with `busy` if it edits while a session runs.
2. **Register it** in `generate_handler![…]` in [`lib.rs`](../../src-tauri/src/lib.rs).
3. **Types**: derive `Serialize` and `TS` with `#[ts(export)]` on new types, and run `cargo test` to generate the bindings.
4. **Backend** [`backend.ts`](../../src/lib/ipc/backend.ts): the method on `Backend`, the Tauri implementation, and a browser one (a simulation, or a rejection with `code: "unavailable"`). `backend.test.ts` checks the argument names Rust expects.
5. **Fake core**: answer it exactly as Rust does, errors included, and pin that in `fake-core.test.ts`.
6. **Store**: call it from an action, never from a component. Guard the answer by macro id (and a sequence number if answers can race).

## Add a trigger kind

Example: `96e2b02`, the interval trigger (*Every…*), which touched 20 files.

1. **Model** [`relay-core/src/triggers.rs`](../../crates/relay-core/src/triggers.rs): a field on `MacroTriggers` with a `#[serde(default)]` (older `library.json` files must load), its `TriggerOn` value and `also_on`. Add the `RunSource` in [`runlog.rs`](../../crates/relay-core/src/runlog.rs).
2. **The watch** [`src-tauri/src/triggers.rs`](../../src-tauri/src/triggers.rs): a pure `…Watch` type with `tick(now, …) → Vec<Uuid>`, unit-tested with a fixed clock (on time, late, after sleep, busy, changed). Then a thread that sleeps, calls `tick` and `fire`s; start it in `spawn`.
3. **Admission** [`coordinator.rs`](../../src-tauri/src/coordinator.rs): `admit` already handles paused, deleted, busy and locked. Change it only if the new kind needs different treatment, and test that.
4. **Frontend**: the card in `MacroPane.svelte`, the text in [`editor/triggers.ts`](../../src/lib/editor/triggers.ts) and [`runs.ts`](../../src/lib/runs.ts), the fake core's defaults. Component tests for the card's every field.
5. **E2E** in `triggers.e2e.test.mjs`, firing on a real event, never on synthesized input.
6. **Spec**: new rules in [Triggers](../spec/triggers.md).

## Add a setting

1. **Rust** [`settings.rs`](../../src-tauri/src/settings.rs): the field and its default. It's read field by field, so an invalid value resets only itself; add a test for that. If it differs for an upgrade, set it in `Settings::upgraded` too.
2. **Apply it** in `update_settings` if it changes something outside the page (the window, the hotkeys, the player).
3. **Frontend**: the default in [`defaults.ts`](../../src/lib/defaults.ts) (matching Rust's), the fake core's defaults, the control in `SettingsPage`, and a test that checks the saved settings.
4. **E2E** in `settings.e2e.test.mjs` if it changes the native window or survives a restart.
5. **Spec**: [Settings](../spec/settings.md).

## Change the file format

Only when the event model changes, and only for macros that use the new thing:

1. Bump `VERSION` in [`format.rs`](../../crates/relay-core/src/format.rs), and write it only when the macro needs it.
2. Add `migrate_vN` from the previous version, working on `serde_json::Value`.
3. Add a fixture of the old format and snapshot its migration.
4. Update [File formats](file-formats.md) and [Export and import](../spec/export-import.md).

## Add an OS backend

1. A module beside `windows/` in [`relay-platform`](../../crates/relay-platform/src), implementing each trait, chosen with `#[cfg]` in `platform()`. See the [table of APIs](platform.md#other-backends).
2. The recorder, key map, grouping and everything above stay as they are. Scan codes are Windows' set 1: map the OS's codes to W3C codes in the backend.
3. Add the target to CI's `quick` job.

## Every change ends the same way

- [ ] Tests at the lowest layer that can express the behavior; new UI controls checked by the command they send
- [ ] The fake core changed with Rust, and `fake-core.test.ts` pins it
- [ ] `npm run verify:quick` passes, and the regenerated bindings and fixtures are committed
- [ ] `npm run verify` if the app, the UI or IPC changed
- [ ] The spec's rules updated; the user guide if what users do changed; this guide if the design changed
- [ ] A `CHANGELOG.md` entry under *Unreleased* for anything users notice

---

<p align="center"><a href="testing.md">← Testing, CI and releases</a> · <a href="README.md">Engineering guide</a></p>
