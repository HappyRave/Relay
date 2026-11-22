# The window, the player and the tray

What Relay's main window and its player show, when each one shows, how they move and take (or don't take) the focus, its messages, the tray icon, closing to the tray and starting with Windows.

Rules are numbered WIN-n. Cite them in tests and PRs.

- [The two windows](#the-two-windows)
- [The main window](#the-main-window)
- [Places and navigation](#places-and-navigation)
- [The editor's layout](#the-editors-layout)
- [Focus](#focus)
- [The player](#the-player)
- [Moving the player and keeping it on top](#moving-the-player-and-keeping-it-on-top)
- [Which window shows when](#which-window-shows-when)
- [Player mode](#player-mode)
- [Messages](#messages)
- [The tray icon](#the-tray-icon)
- [Closing to the tray](#closing-to-the-tray)
- [Start with Windows](#start-with-windows)

## The two windows

- **WIN-1** Relay has two windows: the **main window**, where macros are found, edited and set up, and the **player**, a small bar that floats on top of other apps while Relay records or plays.

## The main window

- **WIN-2** The main window is a normal window: it moves by dragging its top bar, resizes from any edge or corner, and minimizes with **Minimize**.
- **WIN-3** The main window is never on top of other windows.
- **WIN-4** On the first launch, the window opens near the bottom of the main screen, 944 × 612, at most 80% of the screen.
- **WIN-5** The window is never smaller than 760 × 520, and grows with **Text size** (see [Settings](settings.md#appearance)).
- **WIN-6** When the window doesn't fit (a small laptop screen, or high display scaling), Relay shrinks it to fit, down to 40%. It follows Windows' display scaling.
- **WIN-7** Relay remembers the window's size and place, even after a restart.
- **WIN-8** The window never goes off screen or under the taskbar, and keeps 16 pixels from the screen's edges.
- **WIN-9** If the monitor the window was on is gone, the window comes back on the nearest monitor, as close as it can to where it was.
- **WIN-10** The window can be dragged between monitors with different resolutions or display scaling.
- **WIN-11** Only one Relay runs at a time: starting Relay again while it runs brings it back to the front, including out of the tray.

## Places and navigation

- **WIN-12** The main window has three places:

  | Rule | Place | What it's for |
  | --- | --- | --- |
  | **WIN-12a** | **Library** | Relay's home: every macro, with **Record**, **Settings**, **Import…** and **Activity**. See [Library and Activity](library.md). |
  | **WIN-12b** | **A macro** | The editor, on a macro opened from the Library. |
  | **WIN-12c** | **Settings** | Relay itself: recording, its keys, the window, the look and the sounds. See [Settings](settings.md). |

- **WIN-13** The Library's top bar has **Record**, **Switch to the player** and **Settings**. **Settings** opens Settings.
- **WIN-14** **Library**, at the top left of a macro or of Settings, goes back to the Library.
- **WIN-15** <kbd>Alt</kbd>+<kbd>←</kbd> goes back to the Library from Settings, and from a macro unless it records or plays.

## The editor's layout

- **WIN-16** The editor's top bar has, from the left: **Library**; the macro's **name** (click it to [rename](editing.md#renaming-a-macro)); **Undo** and **Redo**; the red **record** button; **Play** (and **Stop** while it plays); the summary of how the macro plays (*×3 · 1× · ±40 ms*), which opens its [play options](playback.md); **More** (⋯); and last, the buttons that switch to the player, minimize and close.
- **WIN-17** **Saved** shows in the top bar for a moment after each change. **Couldn't save** stays there while a change isn't on disk.
- **WIN-18** The top bar's **More** (⋯) has **Export…**, **Duplicate**, **Runs** and **Delete**, for the open macro (see [Library and Activity](library.md)).
- **WIN-19** Under the top bar: the **steps** list on the left, and the **inspector** beside it, with two tabs. **Step** edits the selected steps; **Macro** has the macro's **Play options** and **Runs automatically**, its [triggers](triggers.md).
- **WIN-20** Under the steps and the inspector is the [view](view.md): the preview, the timeline and the measurements. **View** folds it away.
- **WIN-21** Dragging the divider between the steps list and the inspector sets their widths. Double-clicking it puts it back. Its keys are in [Relay's keys and shortcuts](keys.md).
- **WIN-22** In a narrow window, the inspector slides over the steps list when a step is clicked, and <kbd>Esc</kbd> closes it.

## Focus

- **WIN-23** During a recording or a playback, clicking either of Relay's windows doesn't take the keyboard from the app in use: keystrokes keep going to that app.
- **WIN-24** The player never takes the keyboard, at any time: clicking it leaves the app in use ready to type.

## The player

- **WIN-25** The player is 600 × 64, or 760 × 88 with **Player size: Large** (see [Settings](settings.md#appearance)). It grows with the text size, up to 150%.
- **WIN-26** The player has, from left to right:

  | Rule | Part | Does |
  | --- | --- | --- |
  | **WIN-26a** | The **dots** (the grip) | Drag them to move the player |
  | **WIN-26b** | **●** | Record, or stop recording |
  | **WIN-26c** | **▶** / **‖** | Play the macro on its screen, or pause it |
  | **WIN-26d** | **■** | Stop (<kbd>Esc</kbd>) |
  | **WIN-26e** | The screen | The state, the macro's name, the time, and a bar of lights for the progress, with a red light for each click |
  | **WIN-26f** | The macro's name | A menu to switch to another macro, each with its hotkey if it has one. Off while something runs. |
  | **WIN-26g** | **Open the window** (the last button) | Opens the window on the player's macro |

- **WIN-27** The player's screen shows one of these states:

  | Rule | State | When |
  | --- | --- | --- |
  | **WIN-27a** | *READY* | Nothing runs |
  | **WIN-27b** | *GET READY* | Counting down before a recording, with the count (3, 2, 1) on the right |
  | **WIN-27c** | *REC* | Recording: the macro's name reads *New recording*, the number of steps so far is on the right, and the last step under it (*Last: Click · 812, 402*) |
  | **WIN-27d** | *PLAYING*, *PAUSED* | Playing, with the time (*00:04 / 00:10*) or, with a [data file](data-files.md), the row (*Row 37 / 200*) |
  | **WIN-27e** | *DONE ✓* | For a moment after a macro played to the end |
  | **WIN-27f** | *STOPPED* | The last run ended on an error, shown under it until it's dismissed (×) |

## Moving the player and keeping it on top

- **WIN-28** Dragging the grip at the player's left end moves it. Relay remembers where it was put.
- **WIN-29** Until it's moved, the player sits at the bottom center of the screen.
- **WIN-30** **Settings → Window → Keep the player on top** keeps the player above other windows **Always** (the default), only **While working** (recording or playing), or **Never**.
- **WIN-31** When a trigger starts a macro while the window is open, the player comes up on top in a corner of that screen, clear of the window and of where the macro clicks: the top right first, then the top left, the bottom right and the bottom left.
- **WIN-32** When a trigger starts a macro while Relay is in the tray or in player mode, the player shows where it was last left.

## Which window shows when

- **WIN-33** The window and the player show as follows:

  | Rule | When | The window | The player |
  | --- | --- | --- | --- |
  | **WIN-33a** | Nothing runs | Shows | Hidden |
  | **WIN-33b** | Recording | Steps aside (with **Hide the window while recording** on, the default) | Shows when the window doesn't, with the countdown |
  | **WIN-33c** | Playing from the window | Stays, behind the other apps (unless **Hide the window while playing** is on) | Shows when the window doesn't |
  | **WIN-33d** | A trigger starts a macro while the window is open | Stays, and says what runs, with **Stop** | Shows in a free corner of the screen (WIN-31) |
  | **WIN-33e** | **Player mode** | Hidden | Shows |
  | **WIN-33f** | In the tray | Hidden | Shows only while something records or plays |

- **WIN-34** With **Settings → Window → Hide the window while recording** on (the default), the window steps aside while recording and the player shows instead, so Relay never covers what's recorded. The window comes back when the recording stops, on the new macro (see [naming new recordings](recording.md)). In player mode, the player shows the new macro instead.
- **WIN-35** With **Hide the window while recording** off, the window stays where it is during a recording and the player doesn't show. The window's red **record** button counts down instead.
- **WIN-36** With **Settings → Window → Hide the window while playing** on (off by default), the window steps aside while a macro plays, the player shows instead, and the window comes back when the playback ends. Off, the window stays where it is, behind the other apps, so the steps can be watched.
- **WIN-37** When a trigger starts a macro while the window is open, the window stays as it is, on the macro that was open, and says at the bottom what runs, for example *Running “Export invoice to PDF”, started by its schedule*, with **Stop**.

## Player mode

- **WIN-38** In player mode, only the player shows: it takes the window's place.
- **WIN-39** Player mode turns on with the **Show the player** key (<kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>M</kbd> by default; see [Relay's keys](keys.md)) or the **Switch to the player** button in the Library's or the editor's top bar.
- **WIN-40** Player mode turns off with the same key, or the player's **Open the window**.
- **WIN-41** Relay remembers player mode, even after a restart.

## Messages

- **WIN-42** Messages show at the bottom of the window: what just happened, with **Undo** when it can be undone (see [undo and redo](editing.md#undo-and-redo)), and errors.
- **WIN-43** Error messages stay until they're closed (×). Other messages go away after a few seconds.
- **WIN-44** While the window is hidden, the player shows the last error until it's dismissed.

## The tray icon

- **WIN-45** Relay's icon sits in the notification area of the taskbar. Hovering it says what Relay is doing, for example *Relay — recording (Ctrl + Shift + F9 to stop)* or *Relay — playing (Esc to stop)*.
- **WIN-46** Left-clicking the icon puts Relay away in the tray, or brings it back: the window, or the player in player mode.
- **WIN-47** Right-clicking the icon opens its menu:

  | Rule | Item | Does |
  | --- | --- | --- |
  | **WIN-47a** | **Show / hide Relay** | The same as a left-click |
  | **WIN-47b** | **Record** | Starts or stops a recording. It shows the Record key as set. |
  | **WIN-47c** | **Stop** <kbd>Esc</kbd> | Stops the recording or playback |
  | **WIN-47d** | **Triggers active** | Checked while triggers can run. Unchecking it pauses them all; checking it resumes them. See [pausing triggers](triggers.md). |
  | **WIN-47e** | **Open macros folder** | Opens `%APPDATA%\Relay\macros` in Explorer |
  | **WIN-47f** | **Quit Relay** | Quits for real. Hotkeys and triggers stop. |

- **WIN-48** While Relay is in the tray, a recording or a playback still shows the player, so **■** is always at hand.
- **WIN-49** Windows may hide new tray icons under the **^** arrow. Dragging Relay's icon onto the taskbar keeps it visible, as does turning it on in Windows' **Settings → Personalization → Taskbar → Other system tray icons**.

## Closing to the tray

- **WIN-50** With **Settings → Window → Close to the tray** on (the default), the window's close button (**Hide to the tray**) and <kbd>Alt</kbd>+<kbd>F4</kbd> hide Relay to the tray, and hotkeys and triggers keep running.
- **WIN-51** With **Close to the tray** off, the close button (**Quit**) and <kbd>Alt</kbd>+<kbd>F4</kbd> quit Relay. Triggers only work while Relay runs (see [keeping Relay running](triggers.md#keeping-relay-running)).

## Start with Windows

- **WIN-52** With **Settings → Window → Start with Windows** on (off by default), Relay starts hidden in the tray each time the user signs in. Nothing pops up, but hotkeys and triggers are ready; clicking the tray icon opens the window.
- **WIN-53** Start with Windows is a per-user setting, in the *Run* list of the Windows account. It can also be turned off in Task Manager → **Startup apps**.
- **WIN-54** Start with Windows remembers where the Relay program file is: after moving the portable .exe, it has to be turned off and on again.

---

[Spec index](README.md)
