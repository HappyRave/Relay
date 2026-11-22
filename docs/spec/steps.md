# Steps

What a step is: the step types, and how Relay groups recorded input into them. Rules are numbered STEP-n. Cite them in tests and PRs.

- [Step types](#step-types)
- [Mouse steps](#mouse-steps)
- [Keyboard steps](#keyboard-steps)
- [Added steps](#added-steps)
- [Moves](#moves)
- [Step numbering](#step-numbering)

## Step types

- **STEP-1** Relay doesn't show the raw mouse and keyboard events it recorded: it groups them into steps that read like instructions. A step reads as its kind and a detail, separated by a middle dot: *Click · Save button*, *Keys · Ctrl + S*, *Type · “invoice_2026”*.

| Rule | Step | For example | Made from |
| --- | --- | --- | --- |
| **STEP-2** | **Click**, **Double click**, **Triple click**, **Right click**… | *Click · Save button*, *Click · 812, 402* | A mouse button pressed and released without moving more than 4 px ([STEP-12](#mouse-steps), [STEP-13](#mouse-steps)) |
| **STEP-3** | **Drag** | *Drag · 640, 300 → 980, 300* | A button pressed, moved more than 4 px while held, then released ([STEP-12](#mouse-steps)) |
| **STEP-4** | **Scroll** | *Scroll · down 3 notches* | Wheel notches in the same direction, less than 300 ms apart ([STEP-16](#mouse-steps)) |
| **STEP-5** | **Keys** | *Keys · Ctrl + S*, *Keys · Enter* | A key pressed with <kbd>Ctrl</kbd>, <kbd>Alt</kbd> or <kbd>Win</kbd>, or a key that doesn't type a character ([STEP-17](#keyboard-steps)) |
| **STEP-6** | **Type** | *Type · “invoice_2026”*, *Type · “Hello from Relay”* | Characters typed less than 500 ms apart ([STEP-18](#keyboard-steps)) |
| **STEP-7** | **Text** | *Text · “Invoice {date}”* | A Text step added in the editor, or typed text made editable: it's filled in when it plays ([STEP-21](#added-steps)) |
| **STEP-8** | **Wait** | *Wait · 0.50 s* | A pause added in the editor |
| **STEP-9** | **Wait for** | *Wait for · pixel 1248, 680* | A pixel check added in the editor. See [Pixel checks](pixel-checks.md). |
| **STEP-10** | **Find** | *Find · OK button* | A Find image step added in the editor: it waits for a picture, then clicks it. See [Find image](find-image.md). |
| **STEP-11** | **Move** | *Move · 134, 70 → 230, 324* | The cursor moving between two other steps, including before the first and after the last ([Moves](#moves)) |

## Mouse steps

- **STEP-12** A button pressed and released without the cursor moving more than 4 px while it's held is a click. A button pressed, moved more than 4 px while held (even if it came back), then released is a **Drag**.
- **STEP-13** Clicks that follow each other quickly enough, and close enough, by the Windows double-click settings recorded with the macro (see [Recording](recording.md)), merge into one **Double click** or **Triple click** step.
- **STEP-14** Two clicks on the same spot less than the double-click time apart (usually half a second) are a double click, to Windows as much as to Relay. Shortening the pause between them that far merges them: see [Pauses](editing.md#pauses).
- **STEP-15** The cursor moving while a button is held is part of that click or drag, not a move of its own.
- **STEP-16** Wheel notches in the same direction, less than 300 ms apart, form one **Scroll** step, counted in notches (*Scroll · down 3 notches*).

## Keyboard steps

- **STEP-17** A key pressed with <kbd>Ctrl</kbd>, <kbd>Alt</kbd> or <kbd>Win</kbd>, or a key that doesn't type a character (arrows, <kbd>Enter</kbd>, <kbd>Tab</kbd>, function keys…), is a **Keys** step.
- **STEP-18** Characters typed less than 500 ms apart form one **Type** step.
- **STEP-19** Shifted characters and <kbd>AltGr</kbd> characters (like `@` or `€` on many European layouts) are part of a Type step's text, not Keys steps.
- **STEP-20** Key names follow the keyboard layout: on an AZERTY keyboard, the key to the right of <kbd>Tab</kbd> shows as **Ctrl + A**, not *Ctrl + Q*. Playback replays the physical keys (see [Playback](playback.md#where-input-goes)).

## Added steps

- **STEP-21** Recording never makes **Text**, **Wait**, **Wait for** or **Find** steps: they're added in the editor (see [Editing](editing.md)). A Text step is also made from a Type step with **Make editable**. See [Text steps](text-steps.md).

## Moves

- **STEP-22** A move is the cursor going from where it was to where it goes next, usually the next click. Every cursor movement with no button held is a move, between two other steps, including before the first step and after the last.
- **STEP-23** A cursor stopping for a moment halfway doesn't split a move: the stop is part of it.
- **STEP-24** A move with a single recorded position is a jump: it has no duration and no path.

## Step numbering

- **STEP-25** Moves don't count in step numbers: *step 8* is the eighth step that isn't a move.
- **STEP-26** A step keeps its number whether moves show as rows of their own or not (**Show moves**, see [Editing](editing.md)).
- **STEP-27** The same numbers are used in the inspector, in messages such as *Pixel check timed out at step 8*, and in Activity.
- **STEP-28** A move goes by the step it leads to: *Move before step 3*. A move after the last step goes by that step: *Move after step 12*.

---

[Spec index](README.md)
