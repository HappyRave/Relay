# Export and import

What Relay writes when it exports a macro (a `.rly` file, JSON, a standalone program or an AutoHotkey v2 script), how those behave, how files come back in, and where macros are kept.

Rules are numbered XP-n. Cite them in tests and PRs.

- [The Export dialog](#the-export-dialog)
- [What an export contains](#what-an-export-contains)
- [Exporting several macros](#exporting-several-macros)
- [Exported programs](#exported-programs)
- [Program options](#program-options)
- [Exit codes](#exit-codes)
- [Signing and antivirus](#signing-and-antivirus)
- [AutoHotkey scripts](#autohotkey-scripts)
- [Import](#import)
- [Where macros are stored](#where-macros-are-stored)
- [Backing up and moving to another PC](#backing-up-and-moving-to-another-pc)

## The Export dialog

- **XP-1** **Export…** is in the macro's **⋯** menu, both in the editor's top bar (**More**) and on its Library row. It's off while Relay records or plays.
- **XP-2** The dialog is titled *Export “<name>”* and offers four formats. A click chooses one, and so do the arrow keys.

  | Format | File | What it's for |
  | --- | --- | --- |
  | **Relay macro** | `.rly` | Sharing or backing up a macro. Importing it back into Relay restores all its steps, timing and play options. |
  | **Standalone program** | `.exe` | Playing the macro on any Windows PC, even one without Relay. See [Exported programs](#exported-programs). |
  | **AutoHotkey v2** | `.ahk` | Reading, changing or running the macro as a script with AutoHotkey v2. See [AutoHotkey scripts](#autohotkey-scripts). |
  | **JSON events** | `.json` | Reading or processing the macro in other tools: the same data as `.rly`, pretty-printed, plus the list of steps. Relay can import it too. |

- **XP-3** **Save…** asks where to save. The file name defaults to the macro's name, like `export-invoice-to-pdf.rly`, and the dialog shows it.
- **XP-4** A program is never named `relay.exe`: a macro named *Relay* is exported as `relay-macro.exe`.
- **XP-5** **Cancel**, <kbd>Esc</kbd> or a click outside the dialog closes it.

## What an export contains

- **XP-6** Every export contains the macro's events, its play options, and a little about the PC it was recorded on: the monitor layout and the window it was anchored to.
- **XP-7** An export includes the pictures its [Find image](find-image.md) steps look for.
- **XP-8** An export never contains the macro's triggers, its run count or its last run: those stay on the PC.
- **XP-9** An export never contains the screenshot taken as the recording started (see [recording context](recording.md)).
- **XP-10** A `.rly` export doesn't carry the macro's [data file](data-files.md); a standalone program and an AutoHotkey script carry its rows (XP-23, XP-57).
- **XP-11** A macro, and any file or program exported from it, contains everything that was typed while recording it, as text in its *Type* and *Text* steps.

## Exporting several macros

- **XP-12** With several macros selected in the Library, **Export…** in the selection bar asks for a folder and saves each one there as a `.rly` file named after it, then says *Exported 3 macros to Backup* (the folder's name). A file of the same name in that folder is replaced.

## Exported programs

A macro exported as a **Standalone program** is an `.exe` that plays it without Relay.

- **XP-13** An exported program runs on any Windows 10 or 11 PC without Relay and without installing anything.
- **XP-14** Its window looks like Relay's player without the record key: **Pause** and **Stop**, then a dark screen with the state, the macro's name, the time, and a progress bar of lights with the clicks in red.
- **XP-15** The window opens in a corner of the main screen: the bottom right, or another corner the macro doesn't click in.
- **XP-16** It counts down first: *GET READY*, **3, 2, 1**.
- **XP-17** The window never takes the focus.
- **XP-18** While the macro plays it says *PLAYING*, or *WAITING* during a pixel check or Find image step.
- **XP-19** It plays with the play options saved with the macro: speed, repeats, *Vary timing*, *Stop on key* and *Clicks land*.
- **XP-20** When the macro repeats, the time shows the loop too: *2/3 · 00:12 / 00:45*.
- **XP-21** [Text steps](text-steps.md) fill in the date, the time and the clipboard of the PC the program plays on.
- **XP-22** The program has no settings.
- **XP-23** A macro with a data file plays once per row of the file as it was when the macro was exported, and shows the row (*Row 3 / 12*). Changes to the file after exporting don't reach the program.
- **XP-24** Relay won't export a macro whose data file it can't play (missing, or without a column a Text step types), and says why.
- **XP-25** When the macro has played to the end, the window says *DONE ✓* and closes by itself.
- **XP-26** **Pause** pauses it (*PAUSED*); pressing it again plays on.
- **XP-27** **Stop**, <kbd>Esc</kbd> and <kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>End</kbd> stop it, and so does any key when the macro's **Stop on key** is on. The window then says *STOPPED* and closes.
- **XP-28** An exported program's kill switch is always <kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>End</kbd>, whatever Relay's own **Stop everything** key is on that PC. It doesn't use any of Relay's keys.
- **XP-29** Anything the program was holding down is released when it stops.
- **XP-30** Dragging the window anywhere but its keys moves it.
- **XP-31** When something goes wrong, the window stays open and says what on its second line: *TIMED OUT* with the step a pixel check or Find image step stopped at, *NOT PLAYED* when the screen was locked (*The screen is locked, so nothing can be played.*), or *FAILED*.
- **XP-32** When an app runs as administrator, the window adds a note: Windows won't let the program send input to it. Running the program as administrator lets it automate that app.
- **XP-33** After an error, **Stop** turns into **×**, which closes the window.
- **XP-34** If the macro clicks in every corner of the screen, the window lets clicks go through it to the app below, so its keys can't be clicked; <kbd>Esc</kbd> still stops it.

## Program options

Options given on a command line, a shortcut or a scheduled task change how the program plays.

| Rule | Option | Does |
| --- | --- | --- |
| **XP-35** | `--repeat N` or `--repeat forever` | Plays it *N* times, or until stopped, instead of the saved repeat. A macro with a data file plays once per row whatever this says. |
| **XP-36** | `--speed X` | Plays *X* times faster (`0.5` is half speed), from 0.01 to 100 |
| **XP-37** | `--no-countdown` | Starts right away |
| **XP-38** | `--quiet` | Shows no window at all. <kbd>Esc</kbd> or the kill switch stops it. |
| **XP-39** | `--help` | Lists the options |

- **XP-40** Options combine, for example `export-invoice.exe --repeat 3 --no-countdown`.

## Exit codes

The program's exit code says how it ended, for scripts and for Task Scheduler's *Last Run Result*.

| Rule | Code | Meaning |
| --- | --- | --- |
| **XP-41** | 0 | Completed |
| **XP-42** | 1 | Error: the macro couldn't be read, or playback failed |
| **XP-43** | 2 | Bad options (the message says which) |
| **XP-44** | 3 | Stopped: the **Stop** button, <kbd>Esc</kbd>, or a key press |
| **XP-45** | 4 | Stopped by the kill switch |
| **XP-46** | 5 | A pixel check or Find image step timed out |
| **XP-47** | 6 | The screen was locked, so nothing was played |

- **XP-48** A script gets the code by waiting for the program to end: `start /wait export-invoice.exe --quiet` in `cmd` (then `%ERRORLEVEL%`), or `(Start-Process .\export-invoice.exe -ArgumentList '--quiet' -Wait -PassThru).ExitCode` in PowerShell.

## Signing and antivirus

- **XP-49** Exported programs aren't code-signed, and can't be. On a PC that downloaded or received one, Windows SmartScreen may say *Windows protected your PC*; **More info → Run anyway** runs it. A program exported on the same PC runs without the warning.
- **XP-50** An antivirus may flag an exported program, since it presses keys and clicks like a person would. A quarantined program can be restored from the quarantine and allowed, or the macro exported again; someone who has Relay can be sent the `.rly` instead.
- **XP-51** Antivirus programs that watch what a program does while it runs (such as Bitdefender's *Advanced Threat Defense*) are most wary of a program that a script starts from a temporary folder. A program kept in a folder of its own (such as `Documents\Macros`), or in a folder added as an exception, is less likely to be stopped.
- **XP-52** When such an antivirus cleans up after stopping an exported program, it may also quarantine `relay.exe`, the program that wrote the file. Relay can then be restored from the quarantine, or reinstalled.

## AutoHotkey scripts

A macro exported as **AutoHotkey v2** is a script that plays it with [AutoHotkey](https://www.autohotkey.com/) v2, without Relay.

- **XP-53** Each step is a short block, numbered like the steps list; a move goes by the step it leads to (*Move before step 1*). For example:

  ```autohotkey
  Play(N) {
      ; Move before step 1
      Pause(0, false)
      MouseMove 840, 412
      ; 1. Click · Save
      Pause(100)
      Click "840 412"
      ; 2. Keys · Ctrl + S
      Pause(300)
      Send "^s"
      ; 3. Text
      Pause(300)
      SendText "Invoice " . FormatTime(, "yyyy-MM-dd") . " #" . N
  }
  ```

- **XP-54** The script runs with AutoHotkey v2 installed (double-clicking the `.ahk`). A tooltip counts down 3 seconds, then the macro plays and the script exits.
- **XP-55** <kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>End</kbd> stops it, and <kbd>Esc</kbd> does too if the macro stops on a key press. Keys it was holding are released.
- **XP-56** Its options are variables at the top: `Speed`, `Repeat` (0 loops until stopped), `Jitter` (*Vary timing*, in ms; 0 plays exactly) and `Countdown`.
- **XP-57** A macro with a data file has its rows in `Rows` and plays once per row. Relay won't export it if the file can't be played, as for a program (XP-24).
- **XP-58** Each step starts when it did, relative to the step before; waits are those pauses.
- **XP-59** Pixel checks and Find image steps wait for what they look for, up to their timeout, then stop the script with a message.
- **XP-60** Clicks, drags and scrolls are at the screen coordinates they were recorded at, with any key held over them (a Shift-click).
- **XP-61** A mouse path becomes a jump to where it ends.
- **XP-62** The script ignores *Clicks land: Window*.
- **XP-63** Typed text and Text steps are sent as characters. `{date}`, `{time}`, `{clipboard}`, `{n}` and `{col:…}` are filled in when the script plays.
- **XP-64** Find image steps carry their picture inside the script. AutoHotkey's `ImageSearch` compares pixels rather than shapes: it finds the picture only at the size it was taken, and less forgivingly than Relay.
- **XP-65** A Find image step's **Match** becomes its color variation in the script: 85 % allows 30 levels.
- **XP-66** Relay's tests check that AutoHotkey accepts the scripts Relay writes, but don't run them.

## Import

- **XP-67** **Import…** is at the bottom of the Library's side bar. It's off while recording or playing.
- **XP-68** It takes one or more `.rly` or `.json` files, or programs exported by Relay (`.exe`): Relay reads the macro back out of a program.
- **XP-69** Relay reads files from any version of Relay up to its own. A file that needs a newer Relay is refused with *this macro was saved by a newer Relay (format version N)*; which steps need which version is in [Find image](find-image.md) and [Text steps](text-steps.md).
- **XP-70** After an import Relay says how many macros it imported (*Imported 3 macros*), and the first one becomes the current macro (with the red bar).
- **XP-71** A file that didn't work is named in the message, with what went wrong: *Imported 2 macros. old.exe: not a program exported by Relay*. The reasons are:

  | Message | Meaning |
  | --- | --- |
  | *not a Relay macro* | The file isn't a Relay export |
  | *not a program exported by Relay* | The `.exe` is another program |
  | *this macro was saved by a newer Relay (format version 2)* | It needs a newer Relay |
  | *invalid macro file: …* | The file is damaged, or was edited by hand incorrectly |

- **XP-72** If a macro with the same name exists, the imported one gets a number (*Daily report 2*), or the next one if its name already ends in a number (*Daily report 2* becomes *Daily report 3*).
- **XP-73** Importing a macro that's already in the Library adds a copy rather than replacing it.
- **XP-74** Imported macros have no triggers.
- **XP-75** A macro replays screen positions and physical keys. On a PC with a different monitor layout, display scaling or keyboard layout, its clicks can land in the wrong place and its keys can type different characters. *Clicks land: Window* (see [Playback](playback.md)) or recording the macro again on that PC avoids this.

## Where macros are stored

- **XP-76** Relay keeps everything in `%APPDATA%\Relay` (usually `C:\Users\<you>\AppData\Roaming\Relay`). The full list of its files is in [Installing, files and logs](install.md).
- **XP-77** Each macro is one file, `macros\<id>.rly`; deleted macros are in `macros\.trash`. **Open macros folder** in the tray menu opens the `macros` folder.
- **XP-78** Run counts, triggers and data files are kept in `library.json`, on that PC only, not in the macro's file.
- **XP-79** `.rly` files are plain JSON, described in the engineering guide's [File formats](../engineering/file-formats.md).
- **XP-80** A `.rly` file can be edited by hand while Relay isn't running, or edited and then imported as a new macro.

## Backing up and moving to another PC

- **XP-81** Copying the whole `%APPDATA%\Relay` folder while Relay isn't running (tray → **Quit Relay**) backs up everything; copying it back restores it, triggers included.
- **XP-82** Exporting macros and importing the files on another PC moves those macros without their triggers, which have to be set up again there.

---

[Spec index](README.md)
