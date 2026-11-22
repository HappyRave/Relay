# Editing

How the editor shows a macro's steps and how they are selected, added, changed, deleted, undone and renamed.

Rules are numbered EDIT-n. Cite them in tests and PRs.

- [The steps list](#the-steps-list)
- [Selecting steps](#selecting-steps)
- [Adding steps](#adding-steps)
- [The Step tab](#the-step-tab)
- [Saving fields](#saving-fields)
- [Editing several steps](#editing-several-steps)
- [Moves](#moves)
- [Waits](#waits)
- [Pauses](#pauses)
- [Deleting steps](#deleting-steps)
- [Undo and redo](#undo-and-redo)
- [Renaming a macro](#renaming-a-macro)
- [Edits during a session](#edits-during-a-session)

## The steps list

- **EDIT-1** A macro is edited by editing its steps, in the editor, which opens when a macro is opened from the Library. The step types and how input becomes steps are in [Steps](steps.md).
- **EDIT-2** The steps list fills the left of the editor. Its header shows the number of steps, **Find a step**, **Add** and **Steps options** (⋯).
- **EDIT-3** Each row shows the **time** the step starts, a mark for its kind, the step, and a short note on the right.
- **EDIT-4** The marks: a circle for the mouse, a square for the keyboard, a triangle for waits, pixel checks and Find image steps, a ring for a move.
- **EDIT-5** The note says how long the move to the step takes, how many characters were typed, or how long a pixel check or Find image step waits at most (*≤ 5 s*).
- **EDIT-6** Moves are folded into the step they lead to: a click's row stands for the cursor going there too, and its note says how long that move takes.
- **EDIT-7** **Steps options → Show moves** gives each move a row of its own, such as *Move · 134, 70 → 230, 324* with the note *0.63 s*. Relay remembers the choice.
- **EDIT-8** A move with no step after it always has its own row, with or without **Show moves**. It's often the cursor going to the player when the recording was stopped; deleting it makes the macro end at its last real step.
- **EDIT-9** A dashed line, **+ insert here**, sits just after the step under the playhead. It's where [Add](#adding-steps) puts a step.
- **EDIT-10** A striped line such as **1.4 s pause** above a row marks a [pause](#pauses) longer than a second before that step.
- **EDIT-11** **Find a step** (<kbd>Ctrl</kbd>+<kbd>F</kbd>) shows only the rows whose kind or text contains what is typed, such as *click*, *save* or *Ctrl*.
- **EDIT-12** While the macro plays, each row fills with its color once it has played, the step playing is outlined in red, and the list scrolls to follow along.
- **EDIT-13** A step the last run stopped on (a pixel check that timed out, or a Find image step that didn't find its image) is marked in red, with **Timed out**.
- **EDIT-14** With **Window** coordinates (see [Screen or Window coordinates](playback.md#screen-or-window-coordinates)), the list shows positions relative to the window, like *+120, +48*.

## Selecting steps

- **EDIT-15** Clicking a row selects it: the playhead moves to that step, and the [Step tab](#the-step-tab) shows it.
- **EDIT-16** These keys work once the steps list has the focus:

| Rule | To… | Do |
| --- | --- | --- |
| **EDIT-17** | Select the next or previous step (the playhead follows) | <kbd>↓</kbd> or <kbd>→</kbd>, <kbd>↑</kbd> or <kbd>←</kbd> |
| **EDIT-18** | Select the first or last step | <kbd>Home</kbd>, <kbd>End</kbd> |
| **EDIT-19** | Select a range | <kbd>Shift</kbd>+click, or <kbd>Shift</kbd> with the arrows, <kbd>Home</kbd> or <kbd>End</kbd> |
| **EDIT-20** | Add a step to the selection, or take it out | <kbd>Ctrl</kbd>+click |
| **EDIT-21** | Select every step shown | <kbd>Ctrl</kbd>+<kbd>A</kbd> |
| **EDIT-22** | Go back to one selected step | <kbd>Esc</kbd> |
| **EDIT-23** | Edit the selected step: the focus goes to its first field in the inspector | <kbd>Enter</kbd> |
| **EDIT-24** | Delete the selected steps (with Undo) | <kbd>Del</kbd> |
| **EDIT-25** | Open **Add** at the insertion line | <kbd>Insert</kbd> or <kbd>+</kbd> |

- **EDIT-26** The selected step stays selected when other steps are added or deleted, and after an undo.

## Adding steps

- **EDIT-27** **Add**, or <kbd>Insert</kbd> or <kbd>+</kbd> in the steps list, opens a menu with these items:

| Rule | Item | Adds |
| --- | --- | --- |
| **EDIT-28** | **Wait** | A 0.5-second [wait](#waits) |
| **EDIT-29** | **Text** | An empty [Text step](text-steps.md), selected and opened in the inspector |
| **EDIT-30** | **Wait for pixel** | A [pixel check](pixel-checks.md) on the pixel under the macro's cursor at that moment of the macro, with that pixel's color now |
| **EDIT-31** | **Find image** | A [Find image](find-image.md) step: the image is snipped, or taken with **Paste** or **File…** |

- **EDIT-32** The new step goes at the **+ insert here** line, just after the step under the playhead, and everything after it moves later by the time the new step takes.
- **EDIT-33** A step is never split: if the playhead is in the middle of one, such as between the press and the release of a drag, the new step goes right after it.
- **EDIT-34** Hovering the edge between two rows shows a **+** there. It opens the same menu, and adds right after the row above.

## The Step tab

- **EDIT-35** The inspector's **Step** tab shows the selected step: its kind and number (*Click · step 5*; numbering is in [Steps](steps.md)), what it does, and its fields.
- **EDIT-36** The fields per step:

| Rule | Step | Fields |
| --- | --- | --- |
| **EDIT-37** | Every step | **Pause before**: the idle time before the step, in seconds. See [Pauses](#pauses). |
| **EDIT-38** | Click, drag, wait, pixel check, Find image | **Label**, for example *Save button*. Labels show in the steps list and, for a selected click, in the [preview](view.md). |
| **EDIT-39** | A step with a move folded in | **Moving there**, with **Smooth the path** and **Straighten**. See [Moves](#moves). |
| **EDIT-40** | Move (with **Show moves**) | **Takes**, with **Smooth the path** and **Straighten** |
| **EDIT-41** | Click, scroll, drag, keys | Its **Position** (or **From** and **To**), what it **Scrolls**, or its keys, shown for reference |
| **EDIT-42** | Typed text | What it **Types**, and **Make editable**, which turns it into a [Text step](text-steps.md) |
| **EDIT-43** | Text | The **Text**, a button for each placeholder, what it types now, and how long it **Lasts**. See [Text steps](text-steps.md). |
| **EDIT-44** | Wait | How long it **Waits** |
| **EDIT-45** | Pixel check | **Position** (**X**, **Y**), **Pick from screen**, **Color**, **Tolerance** and **Gives up after**. See [Pixel checks](pixel-checks.md). |
| **EDIT-46** | Find image | The image (clicked where the step should click), **Snip**, **Paste** or **File…** to replace it, **Match**, **Gives up after**, **Test** and **Show**, the button it **Clicks**, and where it **Looks on** with several monitors. See [Find image](find-image.md). |

- **EDIT-47** **Delete step**, at the bottom of the Step tab, deletes the step.
- **EDIT-48** With no step selected, the Step tab says so, and points to selecting a step or to the **Macro** tab (<kbd>Ctrl</kbd>+<kbd>M</kbd>), for how the macro [plays](playback.md) and [when it runs](triggers.md).
- **EDIT-49** While the macro plays, the Step tab follows the step playing (*Now · step 5 of 12*) and, with a [data file](data-files.md), the row (*Row 3 of 20*).
- **EDIT-50** When a pixel check timed out in the last run, its Step tab says what happened, for example *Timed out in the last run: the pixel at 1248, 680 px wasn't #2E7D32 within 5 s, so the macro stopped.*

## Saving fields

- **EDIT-51** A change saves as soon as the field is left or <kbd>Enter</kbd> is pressed. There is no Save button.
- **EDIT-52** In the top bar, **Saved** shows for a moment after each change, and **Couldn't save** stays while a change isn't on disk.
- **EDIT-53** A number field also changes by dragging sideways on it, or with <kbd>↑</kbd> and <kbd>↓</kbd>: one step at a time, ten with <kbd>Shift</kbd>.
- **EDIT-54** After saving, a number field shows the value saved: rounded to the millisecond, or kept within its limits.

## Editing several steps

- **EDIT-55** With several steps selected, the Step tab shows what they share:
  - **Pause before** sets the same pause before each of them.
  - **Label** labels each one that takes a label.
  - **Make editable**, shown when some are typed text, turns each typed text into a Text step.
  - **Delete *N* steps** deletes them.
- **EDIT-56** Each of these is one change: a single Undo puts all the steps back as they were.

## Moves

- **EDIT-57** **Moving there** (or **Takes**, on a move's own row) is how long the move takes, from its first recorded position to its last. Shorter is faster. The path keeps its shape, and everything after the move comes earlier or later by the difference.
- **EDIT-58** **Smooth the path** takes the wobble out of a shaky path and keeps its big turns. Pressing it again smooths more.
- **EDIT-59** **Straighten** makes the path a straight line.
- **EDIT-60** Smooth and Straighten change only where the cursor goes, not when: the move still starts and ends at the same times and places, and keeps the way it sped up and slowed down.
- **EDIT-61** Smooth and Straighten each offer **Undo** in their message.
- **EDIT-62** A cursor stopping for a moment halfway doesn't split a move: the stop is part of it.
- **EDIT-63** A move with a single recorded position is a jump: it has no duration to set and no path to reshape.
- **EDIT-64** Selecting a move, or the step it's folded into, draws its path thicker in the [preview](view.md).

## Waits

- **EDIT-65** **Add → Wait** adds a 0.5-second wait after the step under the playhead, and everything after it moves later by the same amount.
- **EDIT-66** **Waits** sets its length. Making a wait longer or shorter moves everything after it by the difference.

## Pauses

- **EDIT-67** A pause is the recorded time between two steps when nothing happened, the mouse still. Moving the mouse isn't a pause: it's a [move](#moves).
- **EDIT-68** A pause is not a wait: a wait is a step added on purpose, a pause is recorded time between steps.
- **EDIT-69** **Pause before** on a step sets the pause before it. Everything after the step moves earlier or later by the difference.
- **EDIT-70** On a row with a move folded in, **Pause before** is the pause before the move.
- **EDIT-71** **Steps options → Shorten pauses** shortens every pause longer than 1 second to 1 second, and says how many it changed (*Shortened 4 pauses to 1 s*), with **Undo**.
- **EDIT-72** Two clicks on the same spot less than the Windows double-click time apart (usually half a second) are a double click, to Windows as much as to Relay. Shortening the pause between two such clicks that far merges them into one **Double click** step (see [Steps](steps.md)).

## Deleting steps

- **EDIT-73** <kbd>Del</kbd> deletes the selected steps, as does **Delete step** (**Delete *N* steps**) in the Step tab.
- **EDIT-74** The whole step goes: a click's press and release, all the characters of a typed text, the keys of a combination with their modifiers, and the move folded into its row.
- **EDIT-75** Deleting a wait, a Text step, a pixel check or a Find image step also removes its time, so the rest of the macro moves earlier.
- **EDIT-76** Deleting any other step leaves the timing of the rest unchanged. Without its move, the cursor jumps to the next step's position when it plays instead of travelling there.
- **EDIT-77** Nothing stays pressed: deleting a press deletes its release too.
- **EDIT-78** After a delete, a message at the bottom of the window (*Deleted the step*, *Deleted 3 steps*) has an **Undo** button.
- **EDIT-79** That Undo message belongs to its macro: it goes away when another macro is opened or another change is made.

## Undo and redo

- **EDIT-80** Every change to a macro can be undone: deleting, adding, pauses, waits, moves, Text steps, pixel checks, Find image steps, labels and the name.

| Rule | To… | Press |
| --- | --- | --- |
| **EDIT-81** | Undo | **Undo** in the top bar, or <kbd>Ctrl</kbd>+<kbd>Z</kbd> |
| **EDIT-82** | Redo | **Redo** in the top bar, or <kbd>Ctrl</kbd>+<kbd>Y</kbd> or <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>Z</kbd> |

- **EDIT-83** Each macro has its own history of up to 100 changes. It lasts until Relay quits.
- **EDIT-84** A change to several steps at once is one change. Typing a new name is one change, not one per letter.
- **EDIT-85** In a text or number field, <kbd>Ctrl</kbd>+<kbd>Z</kbd> undoes the typing in that field instead. A focused switch or choice doesn't stop <kbd>Ctrl</kbd>+<kbd>Z</kbd> from undoing the macro's last change.
- **EDIT-86** Play options (speed, repeat, Vary timing…) and triggers are settings, not edits: they aren't part of the history.

## Renaming a macro

- **EDIT-87** Clicking the name in the top bar makes it editable. It's saved as it's typed.
- **EDIT-88** A macro needs a name: if the field is cleared, the previous name comes back when the field is left.
- **EDIT-89** Names don't have to be unique.
- **EDIT-90** While recording, the name in the editor's top bar reads *New recording*.
- **EDIT-91** How a new recording is named is in [Recording](recording.md).

## Edits during a session

- **EDIT-92** Steps can only be edited while nothing is recording or playing: the Step tab's fields are off.
- **EDIT-93** During a session, the Step tab says *Stop playback to edit steps. Speed and the other play options can still change.*

---

[Spec index](README.md)
