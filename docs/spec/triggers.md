# Triggers

What runs a macro without pressing play: the five triggers, when each fires, what happens when one fires, pausing them, and keeping Relay running for them.

Rules are numbered TRIG-n. Cite them in tests and PRs.

- [A macro's triggers](#a-macros-triggers)
- [Hotkey](#hotkey)
- [Schedule](#schedule)
- [When an app starts](#when-an-app-starts)
- [When a pixel changes](#when-a-pixel-changes)
- [When an image appears](#when-an-image-appears)
- [When a trigger fires](#when-a-trigger-fires)
- [Pausing triggers](#pausing-triggers)
- [Keeping Relay running](#keeping-relay-running)

## A macro's triggers

- **TRIG-1** There are five triggers: **Hotkey** (a key combination, from any app), **Schedule** (chosen days at a set time), **When an app starts** (a few seconds after a program starts), **When a pixel changes** (a pixel on screen turns a given color) and **When an image appears** (a picture shows up anywhere on screen).
- **TRIG-2** Each macro has its own triggers, in the inspector's **Macro** tab (<kbd>Ctrl</kbd>+<kbd>M</kbd>), under **Runs automatically**. A macro can have several triggers, at most one of each kind.
- **TRIG-3** Closed, **Runs automatically** sums up the triggers that are on, such as *Ctrl + Alt + 1 · Mon–Fri at 09:00*, or *Only when you play it* when none is.
- **TRIG-4** **Add** offers only the triggers the macro doesn't have yet.
- **TRIG-5** Each trigger is a card: its name, a line under it saying whether it's on or when it runs next, a switch, a sentence saying what it does, and its fields. **Remove** takes it off the macro.
- **TRIG-6** A macro with no trigger says *It runs only when you play it. Add a hotkey, a schedule, or something to watch for.*
- **TRIG-7** If the triggers couldn't be loaded, the section says *Couldn't load the triggers.* with a **Retry** button.
- **TRIG-8** Triggers are saved on this PC only; an exported macro doesn't include them (see [Export and import](export-import.md)).

## Hotkey

- **TRIG-9** The key field shows **Set…** when empty. Clicking it changes it to *Press keys…*; the next combination pressed becomes the hotkey.
- **TRIG-10** While the field waits for keys, <kbd>Backspace</kbd> clears the hotkey and <kbd>Esc</kbd> cancels.
- **TRIG-11** While the field waits for keys, Relay's keys and the other macros' hotkeys let go of the keyboard, so a combination they use reaches the field (and is then refused, TRIG-15).
- **TRIG-12** Until a hotkey is set, the switch is off and can't be turned on. Setting a hotkey turns the switch on by itself.
- **TRIG-13** A hotkey needs <kbd>Ctrl</kbd>, <kbd>Alt</kbd>, <kbd>Shift</kbd> or <kbd>Win</kbd>, so the key still types normally. Function keys <kbd>F1</kbd>–<kbd>F24</kbd> work on their own. A combination can't end with a modifier.
- **TRIG-14** <kbd>Shift</kbd> alone isn't enough with a letter, digit, punctuation, <kbd>Space</kbd> or <kbd>Enter</kbd> (that's ordinary typing: capital letters). It's enough with function keys, arrows, <kbd>Home</kbd>, <kbd>Tab</kbd> and the like.
- **TRIG-15** A hotkey can't be one of [Relay's own keys](keys.md) as set in Settings → Keys (Record, Play / pause, Show the player, Stop everything), and can't already run another macro.
- **TRIG-16** A hotkey that breaks a rule isn't saved, and Relay says why at the bottom of the window:

  | Rule | Message | When |
  | --- | --- | --- |
  | **TRIG-16a** | *Add Ctrl, Alt, Shift or Win, so the key still types normally* | A plain key |
  | **TRIG-16b** | *Add Ctrl, Alt or Win: Shift + A is ordinary typing* | Shift alone with a key that types |
  | **TRIG-16c** | *Ctrl + Alt + 7 already runs “Fill weekly timesheet”* | Another macro has it |
  | **TRIG-16d** | *Ctrl + Shift + F9 is Relay's “Record” key* | It's one of Relay's keys |

- **TRIG-17** A key Relay can't use in a hotkey makes the field say *Use a letter, digit, F-key or arrow*.
- **TRIG-18** When it works, the line under **Hotkey** says *Runs from any app*.
- **TRIG-19** A hotkey that's saved but that another program registered first doesn't work, and the line under **Hotkey** says so instead: *Ctrl + Alt + 1 is taken by another app*.
- **TRIG-20** Macro hotkeys only run their macro while Relay is idle. During a recording they reach the app and are recorded like any key. During playback they reach other apps, except that with **Stop on key** on, a macro hotkey stops the playback and is swallowed like any key (see [Playback](playback.md)).

## Schedule

- **TRIG-21** A new schedule starts on, as weekdays at 09:00.
- **TRIG-22** Its fields are the days, **M T W T F S S** (highlighted days are on), and the time after **at**.
- **TRIG-23** The line under **Schedule** shows the next run, for example *Next run: Tomorrow 09:00*; *Off* while the switch is off; *Pick a day* when no day is selected.
- **TRIG-24** The sentence says it in short: *Mon–Fri at 09:00*.
- **TRIG-25** Times are in the PC's local time, and follow daylight saving changes.
- **TRIG-26** A run whose time passes while the PC is asleep or off is skipped, not made up later. A run Relay notices up to 2 minutes late (the PC woke just after) still runs; later than that, it's skipped and [Activity](library.md) lists it as skipped.
- **TRIG-27** If the clock jumps back (corrected after waking up, say), a run that already happened doesn't happen again.
- **TRIG-28** A schedule checks the clock every few seconds, so a run can start up to 5 seconds after the minute.

## When an app starts

- **TRIG-29** **Program** takes the program's file name, for example `EXCEL.EXE`. The field suggests the programs running right now.
- **TRIG-30** Without a program, the switch is off and can't be turned on. It turns on with the first program typed.
- **TRIG-31** **Delay** is in seconds, 2 s by default: the time between the program starting and the macro starting.
- **TRIG-32** The sentence reads, for example, *2 s after EXCEL.EXE starts*.
- **TRIG-33** The macro runs each time the program starts, counting from the moment the trigger is turned on: a program opened right after counts. A program already running then, or when Relay starts, doesn't count until it's closed and opened again.
- **TRIG-34** Relay looks for new programs every 2 seconds, so a program closed and reopened within 2 seconds may not be noticed.
- **TRIG-35** Opening a second window of a program that's already running doesn't count, since the program didn't start.
- **TRIG-36** If the trigger is turned off during the delay, the macro doesn't run.

## When a pixel changes

- **TRIG-37** The pixel is set with **Pick from screen** (the same picker as a pixel check's, see [Pixel checks](pixel-checks.md)), which takes the pixel's position and its color at that moment, or by typing **X**, **Y** and the **Color** (`#RRGGBB`).
- **TRIG-38** The switch is turned on by hand.
- **TRIG-39** The sentence reads, for example, *When 1248, 680 turns #EC3013*.
- **TRIG-40** Relay checks the pixel 4 times a second, with a tolerance of 8 per color channel.
- **TRIG-41** The trigger fires when the pixel becomes the color: it has to be a different color first, then match twice in a row (about half a second).
- **TRIG-42** A pixel that stays that color runs the macro once, not over and over. It fires again only after the pixel shows another color twice in a row (so the mouse passing over it doesn't count), then comes back.
- **TRIG-43** A pixel that already has the color when the trigger is turned on doesn't fire until it changes and comes back.
- **TRIG-44** Changing the trigger's position or color starts over: the pixel has to change again.

## When an image appears

- **TRIG-45** The image comes from **Snip** (drag a rectangle around it on screen), **Paste** (a picture on the clipboard) or **File…** (a PNG or JPEG).
- **TRIG-46** Until there's an image, the switch is off and can't be turned on. It turns on by itself with the first image.
- **TRIG-47** A thumbnail shows the image. The sentence states the match it needs, for example *When the image is on screen, at a 85% match*.
- **TRIG-48** **Match** goes from 50 to 100%. With several monitors, **Looks on** chooses which screen it looks on.
- **TRIG-49** **Test** looks for the image once, now, and says where it found it or how close it came. **Show** then marks that spot on the screen for 3 seconds: a red outline, with a dot in the middle.
- **TRIG-50** The image may be anywhere on screen, and from half to twice its size (see [Find image](find-image.md)).
- **TRIG-51** Relay looks twice a second, and only while an image trigger is on. It never looks inside its own windows (the thumbnail isn't the image appearing), nor behind them.
- **TRIG-52** The trigger fires when the image appears: it has to be away twice in a row first, then be seen twice in a row (about a second). An image that stays on screen runs the macro once.
- **TRIG-53** An image already on screen when the trigger is turned on doesn't start the macro until it goes away and comes back.
- **TRIG-54** Changing the image or the match starts over.
- **TRIG-55** The trigger only starts the macro; it passes no position for the macro to click.

## When a trigger fires

- **TRIG-56** A triggered macro plays from the start, with its own play options (speed, repeat and the others). Its run count and last run go up as usual.
- **TRIG-57** If the window is open, it stays as it is, on the macro that was open, and says at the bottom what runs, for example *Running “Export invoice to PDF”, started by its schedule*, with **Stop**.
- **TRIG-58** If the window is open, the player comes up in a corner of that screen, on top, clear of the window and of where the macro clicks: the top right first, then the top left, the bottom right and the bottom left.
- **TRIG-59** If Relay is in the tray or in player mode, the player shows where it was last left and plays the macro.
- **TRIG-60** A trigger doesn't run the macro when:

  | Rule | Situation | What happens |
  | --- | --- | --- |
  | **TRIG-60a** | Relay is already recording or playing | Skipped, with the message *Skipped “Export invoice to PDF”: Relay was busy*. A trigger never interrupts a recording or another macro. |
  | **TRIG-60b** | The screen is locked, or a UAC prompt is up | Skipped silently: Windows doesn't accept simulated input there |
  | **TRIG-60c** | Triggers are paused | Nothing happens |
  | **TRIG-60d** | Relay isn't running | Nothing happens |
  | **TRIG-60e** | The macro's data file can't be played | Skipped (see [Data files](data-files.md)) |

- **TRIG-61** [Activity](library.md) lists each triggered run and each skipped one, with the trigger that fired it.
- **TRIG-62** A triggered macro stops like any other: **Stop**, <kbd>Esc</kbd>, any key (with **Stop on key**), or the [kill switch](playback.md).

## Pausing triggers

- **TRIG-63** The kill switch (Relay's **Stop everything** key, <kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>Backspace</kbd> by default) pauses all triggers, whether Relay is idle, recording or playing, so a looping or scheduled macro can't start again right after it was stopped.
- **TRIG-64** On pausing them, Relay says *Triggers are paused. Resume them from the tray or the Library.*
- **TRIG-65** While triggers are paused, the Library and every macro's **Runs automatically** show *Triggers are paused (the kill switch).* with **Resume**.
- **TRIG-66** **Resume**, or checking **Triggers active** in the tray menu, resumes them.
- **TRIG-67** **Triggers active** in the tray menu is checked while triggers can run. Unchecking it pauses every trigger at once, without turning any off.

## Keeping Relay running

- **TRIG-68** Triggers and macro hotkeys only work while Relay runs, including while it's hidden in the tray. **Quit Relay** stops them.
- **TRIG-69** With **Close to the tray** on (the default), closing the window hides Relay to the tray, and triggers keep running. Off, closing quits Relay. See [Settings](settings.md).
- **TRIG-70** With **Start with Windows** on, Relay starts hidden in the tray at sign-in, with hotkeys and triggers ready. See [Settings](settings.md).

---

[Spec index](README.md)
