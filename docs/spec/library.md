# Library and Activity

This page covers the Library (its lists, search, sort, rows, menus and keys), acting on several macros at once, Activity and Problems, duplicating, and deleting macros to the trash.

Rules are numbered LIB-n. Cite them in tests and PRs.

- [The Library page](#the-library-page)
- [Search and sort](#search-and-sort)
- [Rows](#rows)
- [The row menu](#the-row-menu)
- [Row keys](#row-keys)
- [An empty Library](#an-empty-library)
- [Several macros at once](#several-macros-at-once)
- [Activity and Problems](#activity-and-problems)
- [Run history limits](#run-history-limits)
- [Duplicate](#duplicate)
- [Delete and the trash](#delete-and-the-trash)
- [Undo a delete](#undo-a-delete)

## The Library page

- **LIB-1** The Library is a page of Relay's main window, and Relay's home. Going back to it is described in [places and navigation](windows.md#places-and-navigation).
- **LIB-2** The Library's top bar has **Record**, **Switch to the player** and **Settings**.
- **LIB-3** The side bar chooses what the Library lists:

  | Rule | Item | Lists |
  | --- | --- | --- |
  | **LIB-3a** | **All** | Every macro |
  | **LIB-3b** | **With triggers** | Macros with a hotkey, or another [trigger](triggers.md) that's on |
  | **LIB-3c** | **Recently run** | Macros that ran in the last 7 days |
  | **LIB-3d** | **All runs** | [Activity](#activity-and-problems): every run |
  | **LIB-3e** | **Problems** | Activity: the runs that didn't end as planned |
  | **LIB-3f** | **Import…** | [Imports](export-import.md#import) macro files. It's at the bottom of the side bar. |

- **LIB-4** Each list in the side bar shows how many macros it has.
- **LIB-5** **Problems** has a red badge saying how many problems it lists.
- **LIB-6** When the [kill switch](playback.md#the-kill-switch) has paused the triggers, the Library says *Triggers are paused (the kill switch).*, with **Resume** (see [pausing triggers](triggers.md#pausing-triggers)).

## Search and sort

- **LIB-7** **Search macros** filters the listed macros by name. <kbd>Ctrl</kbd>+<kbd>F</kbd> in the Library goes to it.
- **LIB-8** **Sort by** orders the macros by **Name**, **Newest** (when they were made) or **Last run**.

## Rows

- **LIB-9** Each row has four columns: **Name**, **Hotkey**, **Also runs** and **Last run**.
- **LIB-10** **Name** shows the macro's name, and its length, steps and runs: *10.9 s · 12 steps · 5 runs*.
- **LIB-11** A run counts in the run count when the macro plays to the end.
- **LIB-12** The current macro (the one open in the editor) has a red bar on its row.
- **LIB-13** **Hotkey** shows the macro's [hotkey](triggers.md), if one is on.
- **LIB-14** **Also runs** shows the macro's other triggers that are on, in words: *Mon–Fri at 09:00*, *When EXCEL.EXE starts*, *When a pixel changes*, *When an image appears*. Several are joined with ·: *Mon–Fri at 09:00 · When EXCEL.EXE starts*.
- **LIB-15** Clicking **Also runs** opens the macro on its triggers.
- **LIB-16** **Last run** shows when the macro last ran: *Today, 09:12*, *Fri, 17:40*, *Sep 12* or *Never*.
- **LIB-17** **Last run** has a check mark, or a red mark when that run didn't end as planned. Hovering it says how it ended.
- **LIB-18** Clicking a macro's name, or <kbd>Enter</kbd> on it, opens the macro in the editor.

## The row menu

- **LIB-19** The **⋯** button at the end of a row (its accessible name is *More for* followed by the macro's name) opens the row's menu:

  | Rule | Item | Does |
  | --- | --- | --- |
  | **LIB-19a** | **Open** | Opens the macro in the editor |
  | **LIB-19b** | **Play** | Plays it from the start, without leaving the Library |
  | **LIB-19c** | **Duplicate** | See [Duplicate](#duplicate) |
  | **LIB-19d** | **Export…** | See [Export and import](export-import.md) |
  | **LIB-19e** | **Runs** | Opens [Activity](#activity-and-problems) on its runs |
  | **LIB-19f** | **Delete** | See [Delete and the trash](#delete-and-the-trash) |

- **LIB-20** **Duplicate**, **Export…** and **Delete** are off while Relay records or plays.
- **LIB-21** The editor's **More** (**⋯**) menu, in its top bar, has **Export…**, **Duplicate**, **Runs** and **Delete** for the open macro.

## Row keys

These keys work in a row of the Library.

| Rule | Key | Does |
| --- | --- | --- |
| **LIB-22** | <kbd>↓</kbd>, <kbd>↑</kbd> | Go to the next or previous macro |
| **LIB-23** | <kbd>Enter</kbd> on a name | Opens the macro |
| **LIB-24** | <kbd>Del</kbd> | Moves the macro to the trash, with **Undo**. If its row is selected, it moves every selected macro (see [LIB-30](#several-macros-at-once)). |
| **LIB-25** | <kbd>Ctrl</kbd>+<kbd>D</kbd> | Duplicates the macro |

## An empty Library

- **LIB-26** A new Library is empty. It says *Record what you do once. Relay plays it back.*, with **Record**, until there's a macro.
- **LIB-27** Once there's a macro, Relay opens on the Library, or straight on the macro while it's the only one.

## Several macros at once

- **LIB-28** The box at the start of a row selects that macro. The box in the header selects every macro listed.
- **LIB-29** While macros are selected, a bar at the bottom says how many are selected, and has **Export…**, **Delete** and **Clear**.
- **LIB-30** <kbd>Del</kbd> on a selected row deletes every selected macro.
- **LIB-31** **Export…** asks for a folder and saves each selected macro there as a `.rly` file named after it, then says *Exported 3 macros to Backup*.
- **LIB-32** A file of the same name already in that folder is replaced.
- **LIB-33** **Delete** moves all the selected macros to the trash, with one **Undo**: *Moved 3 macros to the trash*.
- **LIB-34** **Clear** unselects them.

## Activity and Problems

- **LIB-35** Activity is shown by **All runs** and **Problems** in the Library's side bar: what ran and when.
- **LIB-36** **Show the runs of** chooses **Every macro** or one macro.
- **LIB-37** A macro's **Runs** menu item (in its row menu or the editor's **⋯** menu) opens Activity on that macro's runs.
- **LIB-38** Each run shows the macro's name as it was at the time of the run, and:

  | Rule | Column | Shows |
  | --- | --- | --- |
  | **LIB-38a** | **What started it** | *Play* (the Play button or the Play / pause key), *Hotkey*, *Schedule*, *App launch*, *Pixel trigger* or *Image trigger* |
  | **LIB-38b** | **When** | *Today, 09:12*, *Fri, 17:40*, *Sep 12, 14:05* |
  | **LIB-38c** | **Rows or loops** | *12 rows* for a run from a [data file](data-files.md), or *3 loops* when it played more than one loop |
  | **LIB-38d** | **How it ended** | *Completed*, *Stopped*, *Stopped by a key*, *Kill switch*, *Pixel check timed out*, *Image not found* or *Failed* |
  | **LIB-38e** | **Duration** | From start to end, pauses included |

- **LIB-39** Clicking a run shows what its [pixel checks](pixel-checks.md) and [Find image](find-image.md) steps did: *Step 4 · Pixel check matched after 1.2 s*, *Row 2 of 12 · Step 6 · Image found at 812, 344 (93 %) after 0.4 s*. For a pixel check, that's how long it waited; for a Find image step, where and how quickly the image was found.
- **LIB-40** A run's details also show its play options when they weren't the usual ones: *From 00:03.20 · 2× speed · Humanized*.
- **LIB-41** A run that stopped on a pixel check or a Find image step has **Open at step N**, which opens the macro on that step, marked *Timed out*.
- **LIB-42** Activity lists each triggered run, and each trigger that fired but didn't run its macro, with the trigger that fired it.
- **LIB-43** A trigger that didn't run its macro shows as one of: *Skipped: Relay was busy*; *Skipped: screen locked*; *Skipped: PC was asleep*, for a scheduled run the PC slept through; *Skipped: data file*, when the macro's [data file](data-files.md) couldn't be played, with what was wrong.
- **LIB-44** **Problems** lists only the runs that didn't end as planned: every run that ended other than *Completed* or *Stopped*, and every skip. They're marked in red.

## Run history limits

- **LIB-45** Relay keeps the last 200 runs, in `runs.json` (see [the data folder](install.md)).
- **LIB-46** A run keeps its last 50 checks.
- **LIB-47** A run still going when Relay quits isn't recorded.

## Duplicate

- **LIB-48** **Duplicate** makes a copy named *“… (copy)”*, then *“… (copy) 2”*, and so on.
- **LIB-49** Duplicating from the editor's **⋯** menu opens the copy in the editor.
- **LIB-50** The copy starts with no triggers, so the two macros don't compete for the same hotkey or schedule.
- **LIB-51** The copy's run count starts at zero.
- **LIB-52** The copy keeps the original's data file (see [DATA-26](data-files.md#copies-and-exports)).

## Delete and the trash

- **LIB-53** **Delete** moves the macro to the trash, the `macros\.trash` folder in Relay's data folder (see [where macros are stored](export-import.md#where-macros-are-stored)).
- **LIB-54** Deleting the macro open in the editor goes back to the Library, with the next macro in the list as the current one.
- **LIB-55** After the Undo message is gone, the macro's file is still in `macros\.trash`. Importing the file from there gets the macro back.
- **LIB-56** Relay never empties the trash by itself. Deleting files from that folder frees the space.

## Undo a delete

- **LIB-57** After a delete, a message at the bottom of the window says *Moved “…” to the trash*, with an **Undo** button, for 8 seconds.
- **LIB-58** Undo brings the macro back, with its run count and triggers.
- **LIB-59** If another macro took the restored macro's hotkey in the meantime, the macro comes back with that hotkey off, and a message says so: *“Export invoice to PDF” is back; its hotkey Ctrl + Alt + 1 is now used by “Fill weekly timesheet”, so it's off*.

---

[Spec index](README.md)
