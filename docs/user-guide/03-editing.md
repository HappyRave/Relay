# Editing steps

Relay doesn't show you a thousand raw mouse events. It groups them into **steps** that read like instructions: *Click · Save button*, *Ctrl + S*, *"invoice_2026"*. You edit a macro by editing its steps.

- [The step types](#the-step-types)
- [The steps list](#the-steps-list)
- [The step editor](#the-step-editor)
- [Waits](#waits)
- [Pauses](#pauses)
- [Deleting steps](#deleting-steps)
- [Undo and redo](#undo-and-redo)
- [The preview](#the-preview)
- [The timeline](#the-timeline)
- [Renaming a macro](#renaming-a-macro)

## The step types

| Tag | Step | Made from |
| --- | --- | --- |
| `CLICK` | **Click**, **Double click**, **Triple click**, **Right click**… | A button pressed and released without moving more than 4 px. Clicks that follow each other quickly enough, and close enough, by your Windows double-click settings, merge into a double or triple click. |
| `DRAG` | **Drag** from one point to another | A button pressed, moved more than 4 px, then released. |
| `SCROLL` | **Scroll up / down / left / right**, *N notches* | Wheel notches in the same direction, less than 300 ms apart. |
| `KEYS` | A key combination such as **Ctrl + S**, **Alt + Tab**, **Enter**, **F5** | A key pressed with <kbd>Ctrl</kbd>, <kbd>Alt</kbd> or <kbd>Win</kbd>, or a key that doesn't type a character (arrows, <kbd>Enter</kbd>, <kbd>Tab</kbd>, function keys…). |
| `TYPE` | Typed text, such as **"invoice_2026"** | Characters typed less than 500 ms apart. Shifted characters and <kbd>AltGr</kbd> characters (like `@` or `€` on many European layouts) are part of the text. |
| `WAIT` | **Wait 0.5 s** | A pause you inserted. |
| `IF` | **Wait for pixel 1248, 680 = #EC3013** | A [pixel check](05-pixel-checks.md) you inserted. |

Mouse movement between steps isn't a step: it's the path the cursor follows to get to the next one, and it plays back as recorded.

> [!NOTE]
> Key names follow **your keyboard layout**. On an AZERTY keyboard, the key to the right of <kbd>Tab</kbd> shows as **Ctrl + A**, not *Ctrl + Q*.

## The steps list

<p align="center"><img src="../images/editor.png" alt="The Steps tab next to the preview" width="720"></p>

Each row shows the **time** the step starts, its **type** tag, what it does and a detail line (the position of a click, how many characters were typed…).

- The row under the playhead is **highlighted**, and rows not yet reached are dimmed. During playback, the list scrolls to follow along.
- **Click a row** to move the playhead there and open the step editor. Click the row again to close it.
- A striped line such as **1.4 s pause** above a row marks a long [pause](#pauses) before that step.
- The bar above the list shows the step count, with **+ Wait**, **+ Pixel check** and **Trim pauses**.

You can only edit while nothing is recording or playing.

## The step editor

<p align="center"><img src="../images/step-editor.png" alt="The step editor open on a pixel check" width="720"></p>

| Step | What you can change |
| --- | --- |
| Every step | **Pause before**: the idle time before the step, in seconds. See [Pauses](#pauses). |
| Click, drag | **Label**, for example *Save button*. Labels appear in the list and, if **Click labels** is on, in the preview. |
| Wait | **Duration** in seconds, and a **label**. |
| Pixel check | **X**, **Y**, **Color**, **Tolerance**, **Timeout** and a **label**, or **Pick** a pixel on screen. See [Pixel checks](05-pixel-checks.md). |

Changes save as soon as you leave a field or press <kbd>Enter</kbd>. There's no Save button.

## Waits

**+ Wait** inserts a 0.5-second wait **after the step under the playhead**. Everything after it moves later by the same amount.

So to put a wait in the right place, **click the step it should come after**, then press **+ Wait**. You can also drag the playhead into the gap between two steps: the wait goes exactly there. A step is never split: if the playhead is in the middle of one, such as between the press and the release of a drag, the wait goes right after it.

Then open the wait to set its length. Making a wait longer or shorter moves everything after it too.

> [!TIP]
> A wait is fine when an app always takes about the same time. When the delay varies (a page loading, a file exporting), use a [pixel check](05-pixel-checks.md) instead: it waits exactly as long as needed.

## Pauses

A **pause** is the time you spent between two steps while recording: reading the screen, waiting for a window, reaching for the mouse. Only the cursor moves during a pause. Pauses of a second or more are marked in the steps list, like **1.4 s pause**.

- **Change one pause**: open the step after it and set **Pause before**. The mouse movement during the pause is sped up or slowed down to fit, so the cursor still follows the same path. Everything after the step moves earlier or later.
- **Shorten them all**: **Trim pauses** shortens every pause longer than 1 second to 1 second. Relay tells you how many it changed, with an **Undo** button.

A pause is different from a [wait](#waits): a wait is a step you added on purpose, a pause is the recorded time between steps.

> [!NOTE]
> Two clicks on the same spot less than your double-click time apart (usually half a second) are a double click, to Windows as much as to Relay. If you shorten the pause between two such clicks that far, they merge into one **Double click** step.

## Deleting steps

Hover a row and click its **×**. The whole step goes: a click's press and release, all the characters of a typed text, the keys of a combination with their modifiers.

- Deleting a **wait** or a **pixel check** also removes its time, so the rest of the macro moves earlier.
- Deleting any other step leaves the timing of the rest alone.
- Relay makes sure nothing stays pressed. If you delete a press, its release goes too.

After you delete a step, the message at the bottom of the panel has an **Undo** button.

## Undo and redo

Every change you make to a macro (deleting, inserting, pauses, waits, pixel checks, labels, the name) can be undone:

| To… | Press |
| --- | --- |
| Undo | **Undo** in the header, or <kbd>Ctrl</kbd>+<kbd>Z</kbd> |
| Redo | **Redo** in the header, or <kbd>Ctrl</kbd>+<kbd>Y</kbd> (or <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>Z</kbd>) |

- Each macro has its own history of up to 100 changes. It lasts until you quit Relay.
- Typing a new name counts as one change, not one per letter.
- In a text field, <kbd>Ctrl</kbd>+<kbd>Z</kbd> undoes your typing in that field instead.
- Playback options (speed, repeat, Humanize…) and triggers are settings, not edits, and aren't part of the history.

## The preview

The left side of the editor draws the macro over a sketch of your monitors:

- The **mouse path**: a dashed grey line for the whole recording, and a solid red line for the part already played. With **Settings → Preview → Mouse path: Trail only**, only the red part is drawn, which is easier to read on long macros.
- **Numbered squares** for clicks, with their labels if **Click labels** is on.
- The **cursor**, at its position for the current time.
- A **key overlay** in the bottom-left corner, showing the key combination or the text being typed at that moment.

## The timeline

<p align="center"><img src="../images/playing.png" alt="The timeline during playback" width="720"></p>

The timeline at the bottom shows the whole macro in four lanes:

| Lane | Shows |
| --- | --- |
| **Mouse** | When the mouse is moving |
| **Clicks** | A tick for each click |
| **Keys** | Key combinations and typed text, with their labels |
| **Logic** | Waits and pixel checks |

**Click or drag anywhere** on the timeline to move the playhead. The preview, the steps list and the clock all follow. The **◀ ▶** buttons in the transport jump to the previous and next step.

Playback starts from the playhead, so this is also how you replay just the end of a macro.

## Renaming a macro

Click the name in the header and type. It's saved as you type. Names don't have to be unique, but distinct names make the Library and notifications easier to follow.

---

<p align="center"><a href="02-recording.md">← Recording</a> · <a href="README.md">Contents</a> · <a href="04-playback.md">Playing back →</a></p>
