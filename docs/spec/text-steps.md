# Text steps

What a Text step is, how it's made, what its placeholders type, and how it plays.

Rules are numbered TEXT-n. Cite them in tests and PRs.

- [Making a Text step](#making-a-text-step)
- [Placeholders](#placeholders)
- [Braces](#braces)
- [The Types now line](#the-types-now-line)
- [Typing during playback](#typing-during-playback)
- [Duration](#duration)
- [Version compatibility](#version-compatibility)

## Making a Text step

- **TEXT-1** A **Text** step types text that's filled in when the macro plays: an invoice number, today's date, what was copied.
- **TEXT-2** **Add → Text** adds an empty Text step after the step under the playhead (see [adding steps](editing.md#adding-steps)), selects it and opens it in the inspector.
- **TEXT-3** **Make editable**, in the Step tab of a typed text, turns it into a Text step that types the same text, over the same time. For several selected steps, see [editing several steps](editing.md#editing-several-steps).
- **TEXT-4** A Text step's Step tab has the **Text** field, a button for each placeholder, the line saying what it types now, and how long it **Lasts**.

## Placeholders

- **TEXT-5** These placeholders in the **Text** field are filled in each time the step plays:

| Rule | Placeholder | Types | For example |
| --- | --- | --- | --- |
| **TEXT-6** | `{date}` | Today's date | `2026-10-01` |
| **TEXT-7** | `{time}` | The time, to the second | `14:05:09` |
| **TEXT-8** | `{clipboard}` | The text on the clipboard, or nothing if there's none | `ACME Ltd` |
| **TEXT-9** | `{n}` | Which repeat this is, from 1 | `3` |
| **TEXT-10** | `{col:Customer}` | The *Customer* column of the [data file](data-files.md), in this run's row | `Globex` |

- **TEXT-11** With a data file, `{n}` is the row number.
- **TEXT-12** The buttons under the **Text** field put a placeholder where the cursor is. There's one button for each column of the macro's data file.

## Braces

- **TEXT-13** To type a brace itself, double it: `{{` types `{` and `}}` types `}`.

## The Types now line

- **TEXT-14** The line under the placeholder buttons shows what the text would type right now, as the first repeat (with the data file's first row): *Types now: “Invoice 2026-10-01”*.
- **TEXT-15** A mistake, like `{name}` or a `{` that isn't closed, shows on that line instead, and the text isn't saved until it's fixed.
- **TEXT-16** A column the macro has no data file for, or that its file doesn't have, also shows on that line, but the text is saved: Relay won't play the macro until the file has the column (see [data files](data-files.md)).

## Typing during playback

- **TEXT-17** When it plays, the text is typed as characters, one about every 10 ms, rather than as key presses, so it comes out the same whatever the keyboard layout.
- **TEXT-18** A new line in the text presses <kbd>Enter</kbd>, and a tab presses <kbd>Tab</kbd>.

## Duration

- **TEXT-19** A Text step lasts at least as long as typing it takes: about 10 ms a character, counting `{date}` as 10 characters, `{time}` as 8, and `{clipboard}` and columns as none.
- **TEXT-20** **Lasts** makes the step longer, and everything after it moves later, as with a [wait](editing.md#waits).
- **TEXT-21** If the text turns out longer when it plays (a long clipboard), what follows waits until it's typed.

## Version compatibility

- **TEXT-22** A macro with a Text step needs Relay 1.5 or later to open. Macros without one still open in older versions.
- **TEXT-23** An older version says the macro was saved by a newer Relay (see [import](export-import.md)).

---

[Spec index](README.md)
