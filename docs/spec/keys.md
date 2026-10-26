# Relay's keys and shortcuts

Relay's four global keys (their defaults, setting them, refusals and warnings) and every keyboard shortcut in its window, the picker and exported programs.

Rules are numbered KEY-n. Cite them in tests and PRs.

- [Relay's keys](#relays-keys)
- [What each key does](#what-each-key-does)
- [Setting a key](#setting-a-key)
- [Refusals](#refusals)
- [Warnings](#warnings)
- [When a key doesn't reach Relay](#when-a-key-doesnt-reach-relay)
- [Anywhere in the window](#anywhere-in-the-window)
- [The editor](#the-editor)
- [The steps list](#the-steps-list)
- [The timeline and the preview](#the-timeline-and-the-preview)
- [The Library](#the-library)
- [Fields, menus and dialogs](#fields-menus-and-dialogs)
- [The picker](#the-picker)
- [Exported programs](#exported-programs)

## Relay's keys

- **KEY-1** Relay has four global keys: **Record**, **Play / pause**, **Show the player** and **Stop everything** (the kill switch). They work from any app, even when Relay's windows are hidden. Each one is a setting in **Settings → Keys**.

| Rule | Key | Does | New install | Upgraded from an older Relay |
| --- | --- | --- | --- | --- |
| **KEY-2** | **Record** | Starts and stops a recording | <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>F9</kbd> | <kbd>F9</kbd> |
| **KEY-3** | **Play / pause** | Plays the macro open in the window, or shown on the player, from the playhead; or pauses it | <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>F10</kbd> | <kbd>F10</kbd> |
| **KEY-4** | **Show the player** | Switches between the window and the player ([player mode](windows.md)) | <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>M</kbd> | <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>M</kbd> |
| **KEY-5** | **Stop everything** | The kill switch: stops any recording or playback and pauses triggers (see [the kill switch](playback.md#the-kill-switch)) | <kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>Backspace</kbd> | <kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>End</kbd> |
| **KEY-6** | **Stop** | Stops a recording or playback | <kbd>Esc</kbd>, fixed | <kbd>Esc</kbd>, fixed |

- **KEY-7** The new-install Record and Play / pause keys are the first run's recommended choice; its keys screen can choose others (see [the first run](settings.md#the-first-run)).
- **KEY-8** Someone upgrading from an older Relay keeps the keys they knew: <kbd>F9</kbd>, <kbd>F10</kbd>, <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>M</kbd> and <kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>End</kbd> (the keys screen shown on an upgrade: see [the first run](settings.md#the-first-run)).
- **KEY-9** Tooltips, the player and the tray menu name Relay's keys as they're set.
- **KEY-10** Relay only holds on to a key while it needs it: the Play / pause key belongs to your apps while you record, and <kbd>Esc</kbd> belongs to your apps while Relay is idle.

## What each key does

What a key does depends on what Relay is doing:

| Rule | Key | Idle | While recording | While playing |
| --- | --- | --- | --- | --- |
| **KEY-11** | **Record** | Starts recording | Stops recording (during the countdown: cancels it) | Stops playback¹ |
| **KEY-12** | **Play / pause** | Plays the open macro from the playhead | Recorded like any key | Pauses or resumes |
| **KEY-13** | **Show the player** | Turns [player mode](windows.md) on or off | Recorded like any key | Stops playback¹ |
| **KEY-14** | **Stop everything** | Pauses triggers | Stops, keeps the recording, pauses triggers | Stops, pauses triggers |
| **KEY-15** | <kbd>Esc</kbd> | Reaches your app | Stops recording² | Stops playback |
| **KEY-16** | A macro's hotkey | Runs that macro | Recorded like any key | Stops playback¹ |

¹ When the macro's **Stop on key** is on (the default), like any key: see [Stop on key press](playback.md#stop-on-key-press). With it off, the key reaches your app.

² Unless **Settings → Recording → Esc stops recording** is off: then <kbd>Esc</kbd> is recorded like any key (see [recording](recording.md)).

- **KEY-17** During a recording, the keys that belong to Relay are the Record key and the kill switch (and <kbd>Esc</kbd>, while **Esc stops recording** is on). During playback, they're the Play / pause key, <kbd>Esc</kbd> and the kill switch.

## Setting a key

- **KEY-18** In **Settings → Keys**, **Change** next to a key makes it say *Press keys…*, and **Change** becomes **Cancel**.
- **KEY-19** The combination pressed next sets the key, and the line under it says so: *Set to Ctrl + Shift + F9.*
- **KEY-20** <kbd>Backspace</kbd> sets the key to **None**: the line says *Set to None: use the buttons instead.* The buttons still do that job, and the key goes to your apps. A key set to **None** does nothing.
- **KEY-21** <kbd>Esc</kbd> cancels.
- **KEY-22** **Stop everything** can't be **None**. <kbd>Backspace</kbd> on it says *Stop everything can't be turned off. Choose another combination.*
- **KEY-23** While Relay listens for a combination, its own keys and the macros' hotkeys let go of the keyboard, so a combination they use reaches the field.

## Refusals

- **KEY-24** A key that breaks a rule isn't set, and the line under it says why.
- **KEY-25** Relay's keys follow the same rules as a [macro's hotkey](triggers.md#hotkey): a key that types needs <kbd>Ctrl</kbd>, <kbd>Alt</kbd> or <kbd>Win</kbd> (<kbd>Shift</kbd> alone isn't enough), function keys work alone, and a combination can't end with a modifier.
- **KEY-26** Two of Relay's keys can't be the same: *F10 is already Relay's “Record” key*, or *Ctrl + Shift + F9 is already Relay's “Record” key*.
- **KEY-27** None of Relay's keys can be a macro's hotkey: *Ctrl + Alt + 1 already runs “Fill weekly timesheet”*.

## Warnings

- **KEY-28** A key that's set can still come with a warning under it, when it costs you something. The key stays set.

| Rule | When | It says, for example |
| --- | --- | --- |
| **KEY-29** | Apps use the key | *F9 recalculates in Excel and updates fields in Word. While Relay runs, they won't get it.* |
| **KEY-30** | A <kbd>Ctrl</kbd>+<kbd>Alt</kbd> combination types a character on your keyboard layout (<kbd>AltGr</kbd>) | *Ctrl + Alt + E types “€” on your keyboard (AltGr + E). You couldn't type it while this is set.* |
| **KEY-31** | Remote Desktop, VMware Horizon, Citrix or Hyper-V take it | *Remote Desktop sends this as Ctrl + Alt + Del. In a remote session, Relay won't get it.* |

- **KEY-32** On many keyboard layouts, <kbd>Ctrl</kbd>+<kbd>Alt</kbd> works as <kbd>AltGr</kbd> (<kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>E</kbd> types *€*). While Relay uses such a combination, that character can't be typed. Combinations with <kbd>Ctrl</kbd>+<kbd>Shift</kbd> don't get this warning.

## When a key doesn't reach Relay

- **KEY-33** When another program already has one of Relay's keys, Relay says so once per run, for example: *Another program uses Ctrl + Alt + Backspace, so Relay's “Stop everything” key doesn't work. Choose another in Settings → Keys.*
- **KEY-34** In a Remote Desktop session, the remote-desktop client takes some combinations for itself: <kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>End</kbd> becomes <kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>Del</kbd> in the remote PC, so it doesn't reach Relay. <kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>Backspace</kbd> works over Remote Desktop.
- **KEY-35** On a laptop, the function keys may need <kbd>Fn</kbd>.

## Anywhere in the window

| Rule | Key | Does |
| --- | --- | --- |
| **KEY-36** | <kbd>Ctrl</kbd>+<kbd>Z</kbd> | In a macro: undoes its last edit (see [undo and redo](editing.md#undo-and-redo)). In a text or number field: undoes the typing in that field instead. A focused switch or choice doesn't get in the way. |
| **KEY-37** | <kbd>Ctrl</kbd>+<kbd>Y</kbd> or <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>Z</kbd> | In a macro: redo |
| **KEY-38** | <kbd>Alt</kbd>+<kbd>←</kbd> | Back to the Library, from Settings, or from a macro unless it records or plays |
| **KEY-39** | <kbd>Tab</kbd> / <kbd>Shift</kbd>+<kbd>Tab</kbd> | Moves between controls |
| **KEY-40** | <kbd>Alt</kbd>+<kbd>F4</kbd> | Same as the window's close button: hides Relay to the tray, or quits with **Close to the tray** off (see [settings](settings.md)) |

## The editor

| Rule | Key | Does |
| --- | --- | --- |
| **KEY-41** | <kbd>Ctrl</kbd>+<kbd>M</kbd> | Opens the inspector's **Macro** tab |
| **KEY-42** | <kbd>Ctrl</kbd>+<kbd>F</kbd> | **Find a step** |
| **KEY-43** | <kbd>←</kbd> <kbd>→</kbd> on the inspector's tabs | Switches between **Step** and **Macro** |
| **KEY-44** | <kbd>Esc</kbd> | In a narrow window, closes the inspector |

## The steps list

- **KEY-45** These keys work once the steps list has the focus.

| Rule | Key | Does |
| --- | --- | --- |
| **KEY-46** | <kbd>↓</kbd> or <kbd>→</kbd>, <kbd>↑</kbd> or <kbd>←</kbd> | Selects the next or previous step (the playhead follows) |
| **KEY-47** | <kbd>Home</kbd>, <kbd>End</kbd> | Selects the first or last step |
| **KEY-48** | <kbd>Shift</kbd> with the above, or <kbd>Shift</kbd>+click | Selects a range |
| **KEY-49** | <kbd>Ctrl</kbd>+click | Adds a step to the selection, or takes it out |
| **KEY-50** | <kbd>Ctrl</kbd>+<kbd>A</kbd> | Selects every step shown |
| **KEY-51** | <kbd>Esc</kbd> | Goes back to one selected step |
| **KEY-52** | <kbd>Enter</kbd> | Moves the focus to the selected step's first field in the inspector |
| **KEY-53** | <kbd>Del</kbd> | Deletes the selected steps, with **Undo** |
| **KEY-54** | <kbd>Insert</kbd> or <kbd>+</kbd> | Opens **Add** at the insertion line |

## The timeline and the preview

| Rule | Where | Key | Does |
| --- | --- | --- | --- |
| **KEY-55** | Timeline | <kbd>←</kbd> or <kbd>↓</kbd>, <kbd>→</kbd> or <kbd>↑</kbd> | Moves the playhead to the previous or next step |
| **KEY-56** | Timeline | <kbd>Shift</kbd>+arrow | Moves the playhead a tenth of a second back or forward |
| **KEY-57** | Timeline | <kbd>Home</kbd>, <kbd>End</kbd> | Moves the playhead to the start or the end |
| **KEY-58** | Timeline | <kbd>Space</kbd> | Plays or pauses |
| **KEY-59** | Preview | <kbd>+</kbd>, <kbd>−</kbd> | Zooms in, out |
| **KEY-60** | Preview | Arrows | Moves around, when zoomed in |
| **KEY-61** | Preview | <kbd>0</kbd> | Shows the whole screen |

## The Library

| Rule | Key | Does |
| --- | --- | --- |
| **KEY-62** | <kbd>Ctrl</kbd>+<kbd>F</kbd> | **Search macros** |
| **KEY-63** | <kbd>↓</kbd>, <kbd>↑</kbd> in a row | Goes to the next or previous macro |
| **KEY-64** | <kbd>Enter</kbd> on a name | Opens the macro in the editor |
| **KEY-65** | <kbd>Del</kbd> in a row | Moves the macro to the trash, with **Undo**. If its row is selected, every selected macro. |
| **KEY-66** | <kbd>Ctrl</kbd>+<kbd>D</kbd> in a row | Duplicates the macro |

## Fields, menus and dialogs

| Rule | Where | Key | Does |
| --- | --- | --- | --- |
| **KEY-67** | Number field | <kbd>↑</kbd>, <kbd>↓</kbd> | One step up or down; with <kbd>Shift</kbd>, ten |
| **KEY-68** | Any field | <kbd>Enter</kbd>, or leaving the field | Saves the change |
| **KEY-69** | Hotkey field | Any combination | Sets the hotkey |
| **KEY-70** | Hotkey field | <kbd>Backspace</kbd> | Clears it |
| **KEY-71** | Hotkey field | <kbd>Esc</kbd> | Cancels |
| **KEY-72** | Menu | <kbd>↑</kbd> <kbd>↓</kbd>, <kbd>Home</kbd> <kbd>End</kbd> | Moves between items |
| **KEY-73** | Menu | <kbd>Esc</kbd> or <kbd>Tab</kbd> | Closes it |
| **KEY-74** | Choices (such as **Speed**) | Arrows | Moves the choice |
| **KEY-75** | The divider between the steps list and the inspector | <kbd>←</kbd> <kbd>→</kbd> (with <kbd>Shift</kbd>, further), <kbd>Home</kbd> <kbd>End</kbd> | Moves it. A double-click puts it back. |
| **KEY-76** | Export dialog | Arrows | Chooses the format |
| **KEY-77** | Export dialog | <kbd>Esc</kbd> | Closes it |

## The picker

- **KEY-78** These keys work in the picker that **Pick from screen** shows (for a [pixel check](pixel-checks.md) or the [pixel trigger](triggers.md)).

| Rule | Key | Does |
| --- | --- | --- |
| **KEY-79** | Arrows | Move the cursor one pixel; with <kbd>Shift</kbd>, ten |
| **KEY-80** | <kbd>Space</kbd> (or **Freeze** in the picker's help bar) | Holds the screen as it is (*FROZEN*); again goes back to the live screen (*LIVE*) |
| **KEY-81** | Click or <kbd>Enter</kbd> | Picks the pixel under the cursor |
| **KEY-82** | <kbd>Esc</kbd> or right-click | Cancels |

- **KEY-83** Other keys reach the app under the picker. The picker never takes the focus, so an open menu stays open.

## Exported programs

- **KEY-84** A macro [exported as a program](export-import.md) doesn't use Relay's keys. It stops on <kbd>Esc</kbd> or <kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>End</kbd>, and on any other key when its macro's **Stop on key** is on.
- **KEY-85** An exported program's kill switch is always <kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>End</kbd>, whatever Relay's own **Stop everything** key is: the program has no settings.

---

[Spec index](README.md)
