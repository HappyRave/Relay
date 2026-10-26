# Playback

How Relay plays a macro: the controls, the playhead, the play options, where its input goes, the kill switch and how a playback ends.

Rules are numbered PLAY-n. Cite them in tests and PRs.

- [Play, pause and stop](#play-pause-and-stop)
- [The playhead](#the-playhead)
- [The play options](#the-play-options)
- [The playback summary](#the-playback-summary)
- [Repeat and speed](#repeat-and-speed)
- [Vary timing](#vary-timing)
- [Stop on key press](#stop-on-key-press)
- [Screen or Window coordinates](#screen-or-window-coordinates)
- [Where input goes](#where-input-goes)
- [The kill switch](#the-kill-switch)
- [When playback ends](#when-playback-ends)

## Play, pause and stop

- **PLAY-1** Playback replays a macro's clicks, keys and mouse path with the timing recorded, or faster, slower, looped or slightly varied, as its [play options](#the-play-options) say.
- **PLAY-2** Playback starts from the playhead with any of: the **Play / pause** key ([Relay's keys](keys.md)), **Play** in the editor's top bar, the player's **▶**, or <kbd>Space</kbd> on the focused timeline.
- **PLAY-3** The **Play / pause** key plays the macro open in the window, or the one shown on the player.
- **PLAY-4** The same controls pause a playing macro, and resume a paused one.
- **PLAY-5** Playback stops with <kbd>Esc</kbd>, **Stop** in the editor's top bar, the player's **■**, or **Stop** in the tray menu.
- **PLAY-6** <kbd>Esc</kbd> stops playback whatever **Settings → Recording → Esc stops recording** is set to.
- **PLAY-7** **Play** in a Library row's **More** menu plays that macro from the start, and the window stays in the Library.
- **PLAY-8** While a macro plays in the editor, **Play** becomes **Pause**, with **Stop** beside it. How the steps list and the Step tab follow the playback is in [Editing](editing.md).
- **PLAY-9** **Record** is off while a macro plays. **Play** is off while recording, and while a trigger plays another macro.

## The playhead

- **PLAY-10** Playback starts wherever the playhead is. If the playhead is at the end, playback starts from the beginning.
- **PLAY-11** Selecting a step or clicking the timeline moves the playhead there, so the next playback plays from that point.
- **PLAY-12** Clicking the timeline or a step while a macro plays jumps there, and playback continues from that point.
- **PLAY-13** The **Play / pause** key uses the same playhead the editor shows. After **Stop**, the playhead is back at the start. After a complete run, it's at the end, so the key plays again from the beginning. After a [pixel check](pixel-checks.md) or a [Find image](find-image.md) step timed out, it's on that step.
- **PLAY-14** Relay never leaves a key or mouse button stuck: anything the macro holds down is released when playback stops, at the end of each loop, and when the playhead jumps to another point.
- **PLAY-15** Relay's playback timing is precise to about a millisecond.

## The play options

- **PLAY-16** Each macro has its own play options, saved with it, in its file. They're in the inspector's **Macro** tab, under **Play options**, which opens with <kbd>Ctrl</kbd>+<kbd>M</kbd> or a click on the [playback summary](#the-playback-summary).

| Rule | Option | Default | Effect |
| --- | --- | --- | --- |
| **PLAY-17** | **Repeat** | 1 time | How many times the macro runs in a row. See [Repeat and speed](#repeat-and-speed). |
| **PLAY-18** | **Speed** | 1× | How fast it plays |
| **PLAY-19** | **Data file** | None | A CSV file: the macro plays once per row. See [Data files](data-files.md). |
| **PLAY-20** | **Vary timing** | On, ±40 ms | Shifts each step by a small random amount. See [Vary timing](#vary-timing). |
| **PLAY-21** | **Stop on key** | On | Any key pressed stops playback. See [Stop on key press](#stop-on-key-press). |
| **PLAY-22** | **Clicks land** | Screen | At the recorded screen pixels, or relative to the window. See [Screen or Window coordinates](#screen-or-window-coordinates). |

- **PLAY-23** Speed and the other play options can change while the macro plays.

## The playback summary

- **PLAY-24** Next to **Play** in the editor's top bar, a summary shows how the macro plays, for example *×3 · 1× · ±40 ms*: the repeat (*×3*, *∞*, or *Each row (20)* with a [data file](data-files.md)), the speed, the timing variation when **Vary timing** is on, and *keys don't stop it* when **Stop on key** is off.
- **PLAY-25** Clicking the summary opens the **Macro** tab's **Play options**.

## Repeat and speed

- **PLAY-26** **Repeat** takes a count from 1 to 9999: typed, changed by dragging the field sideways, or with <kbd>↑</kbd> and <kbd>↓</kbd>.
- **PLAY-27** **Forever** loops the macro until it's stopped. Pressing **Forever** again goes back to the count the macro had.
- **PLAY-28** With a data file, the macro plays once per row instead of its repeat: see [Data files](data-files.md).
- **PLAY-29** **Speed** is one of **0.5×**, **1×**, **2×** and **4×**.
- **PLAY-30** At 2× or 4×, the apps the macro works in get half or a quarter of the time they had during the recording; an app that can't keep up gets clicks before its window is ready.

## Vary timing

- **PLAY-31** **Vary timing** shifts each step by a small random amount. It's on by default; off, the recording replays with its exact timing.
- **PLAY-32** **By up to** sets the range, from **±0** to **±200 ms** (default **±40 ms**). The field can be dragged sideways.
- **PLAY-33** A step moves as a whole: a click's press and release move together, and the time between them stays exactly as recorded.
- **PLAY-34** Steps never swap order.
- **PLAY-35** Playing from a step always plays that whole step, even if the variation moves it slightly earlier.
- **PLAY-36** Each loop gets a new random pattern.

## Stop on key press

- **PLAY-37** With **Stop on key** on (the default), any key pressed stops playback the moment it's pressed. The option says *Any key stops playback*.
- **PLAY-38** The key that stops playback is swallowed: it doesn't reach the app the macro is typing into.
- **PLAY-39** <kbd>Shift</kbd>, <kbd>Ctrl</kbd>, <kbd>Alt</kbd> and <kbd>Win</kbd> pressed alone don't stop playback.
- **PLAY-40** The **Play / pause** key still pauses rather than stops, and the [kill switch](#the-kill-switch) still works. The Play / pause key's combination without its modifiers stops playback like any other key: with <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>F10</kbd> set, <kbd>F10</kbd> alone stops it.
- **PLAY-41** The **Record** key, the **Show the player** key and macro hotkeys pressed during playback are keys like any other: with **Stop on key** on they stop playback and are swallowed; with it off they reach the app.
- **PLAY-42** Moving to another app with the keyboard stops playback when **Stop on key** is on.
- **PLAY-43** Moving the mouse doesn't stop playback, but the mouse movement does interfere with the macro's.
- **PLAY-44** With **Stop on key** off, the option says *Keys don't stop it: Esc and Stop still do*, and the [summary](#the-playback-summary) says *keys don't stop it*.

## Screen or Window coordinates

**Clicks land** decides where clicks go.

| Rule | Choice | Clicks land… |
| --- | --- | --- |
| **PLAY-45** | **Screen** (default) | At the exact screen pixels recorded |
| **PLAY-46** | **Window** | At the same place relative to the window first clicked in during the recording |

- **PLAY-47** With **Window**, Relay finds the recorded app's window (the same program and window type) when playback starts, and shifts every click and movement by how far that window has moved.
- **PLAY-48** **Window** follows a window that moved, not one that was resized: a button that moves when the window gets bigger is missed.
- **PLAY-49** If Relay can't find the window, it says so and plays at screen coordinates instead.

## Where input goes

- **PLAY-50** Relay injects input the same way the mouse and keyboard would: it goes to whatever is under the cursor and to whichever window has the focus.
- **PLAY-51** With **Screen** coordinates, clicks land at the recorded screen positions, so a different window position, monitor layout or display scaling from the recording's moves where they land.
- **PLAY-52** Relay replays physical keys. With another keyboard layout than the recording's (AZERTY versus QWERTY, or another input language), the same keys type different characters. [Text steps](text-steps.md) type characters instead.
- **PLAY-53** A playback started with Relay's own **Play** button first hands the focus back to the app used before Relay, so the macro types into that app. If that app was closed or minimized meanwhile, the focus can end up somewhere else. The **Play / pause** key and the player's **▶** leave the focus where it is.
- **PLAY-54** Relay's windows don't take the focus while a macro plays, even when clicked.
- **PLAY-55** If the macro clicks where the player is, the player lets clicks pass through it for the whole playback.
- **PLAY-56** Relay's window stays where it is during playback, behind the app that was in use, unless **Hide the window while playing** is on (see [Settings](settings.md)).
- **PLAY-57** Windows doesn't let a normal app send input to one running as administrator. When the app in front runs as administrator, the input is blocked and Relay says *The app in front runs as administrator, so Windows blocks Relay's input to it.* Running Relay as administrator too lifts this.
- **PLAY-58** Nothing can be sent while the screen is locked or a UAC prompt is up.
- **PLAY-59** Games with anti-cheat software may ignore simulated input or flag it.

## The kill switch

- **PLAY-60** The kill switch is Relay's **Stop everything** key. Its default, how to set it, and its warnings are in [Relay's keys](keys.md).
- **PLAY-61** It works from anywhere, at any time, even while a macro is typing.
- **PLAY-62** It stops whatever Relay is doing: a countdown, a recording (which is kept) or a playback.
- **PLAY-63** It pauses all triggers, so a scheduled or looping macro can't restart right away. With nothing running, that's all it does. Resuming them is in [Triggers](triggers.md#pausing-triggers).

## When playback ends

| Rule | How it ended | Then |
| --- | --- | --- |
| **PLAY-64** | It finished | The playhead stays at the end, the player says *DONE ✓*, and the run counts in the Library's *Last run* and in [Activity](library.md). |
| **PLAY-65** | Stopped: <kbd>Esc</kbd>, **Stop**, a key press, or the kill switch | The playhead goes back to the start. |
| **PLAY-66** | A pixel check timed out, or a Find image step didn't find its image | The playhead stays on that step, and the player says *STOPPED* with the message until it's dismissed. See [Pixel checks](pixel-checks.md) and [Find image](find-image.md). |

- **PLAY-67** When playback stops by itself, a message at the bottom of the window, or on the player, says why: a key press with **Stop on key** on, a pixel check that timed out, an image not found, or the kill switch (which also paused the triggers).

---

[Spec index](README.md)
