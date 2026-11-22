# Behavior spec

Every rule of how Relay behaves, one page per feature. Each rule has a number with its page's prefix: **PLAY-12**, **TRIG-3**.

The spec is the reference that the code, the tests and the user guide must agree with. When they disagree, one of them is a bug. The [user guide](../user-guide/README.md) explains how to use Relay; the [engineering guide](../engineering/README.md) explains how it's built.

- [Using the spec](#using-the-spec)
- [Pages](#pages)

## Using the spec

- Cite rules by their ID in tests, pull requests and issues.
- A behavior change updates its rules in the same branch.
- A new rule takes the next free number on its page. A removed rule's number isn't reused.

## Pages

| Page | Prefix | What it covers |
| --- | --- | --- |
| [Recording](recording.md) | REC | Starting and stopping, the countdown, what's recorded and left out, the input hook |
| [Steps](steps.md) | STEP | The step types, how recorded input groups into steps, step numbers |
| [Editing](editing.md) | EDIT | The steps list, selecting, adding, changing and deleting steps, undo, renaming |
| [Text steps](text-steps.md) | TEXT | Text steps, their placeholders, and how they type |
| [Pixel checks](pixel-checks.md) | PIX | *Wait for* steps: matching, tolerance, timeout, the screen picker |
| [Find image](find-image.md) | IMG | Find image steps: the search, the click, other sizes, giving up |
| [Data files](data-files.md) | DATA | A CSV that plays the macro once per row, and when it can't |
| [The view](view.md) | VIEW | The preview, the timeline and the measurements table |
| [Playback](playback.md) | PLAY | The controls, the playhead, the play options, the kill switch, how a playback ends |
| [Triggers](triggers.md) | TRIG | Hotkey, schedule, app, pixel and image triggers, pausing them |
| [Keys](keys.md) | KEY | Relay's four global keys, and every shortcut in its windows |
| [Library](library.md) | LIB | The Library, Activity and Problems, duplicating, the trash |
| [Export and import](export-import.md) | XP | `.rly`, JSON, exported programs, AutoHotkey scripts, import, where macros are kept |
| [Windows](windows.md) | WIN | The main window, the player, focus, messages, the tray, starting with Windows |
| [Settings](settings.md) | SET | Every setting and its default, a damaged settings file, the first run |
| [Installing](install.md) | INST | Requirements, installer and portable exe, the data folder, logs, privacy |

---

[Documentation home](../README.md)
