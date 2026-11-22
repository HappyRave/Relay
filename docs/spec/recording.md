# Recording

How Relay starts, runs and ends a recording, what it captures and what it leaves out. Rules are numbered REC-n. Cite them in tests and PRs.

- [Starting and stopping](#starting-and-stopping)
- [The countdown](#the-countdown)
- [While recording](#while-recording)
- [What is recorded](#what-is-recorded)
- [Recording context](#recording-context)
- [What is left out](#what-is-left-out)
- [Recording options](#recording-options)
- [The input hook](#the-input-hook)
- [The end of a recording](#the-end-of-a-recording)
- [Privacy](#privacy)

## Starting and stopping

- **REC-1** A recording starts with the record key (see [Relay's keys](keys.md)), the red **record** button in the Library, the editor or the player, or **Record** in the tray menu.
- **REC-2** **Record** is off while a macro plays.
- **REC-3** A recording stops, and is kept, with the record key again, <kbd>Esc</kbd> (unless **Esc stops recording** is off, REC-37), the player's **■** or **●**, **Record** in the tray menu, or **Stop** in the tray menu.
- **REC-4** The kill switch also stops a recording (or its countdown), and the recording is kept. See [the kill switch](playback.md#the-kill-switch).
- **REC-5** Every recording is saved to the Library automatically and opens in the editor when it stops, except an empty one (REC-46).
- **REC-6** When a recording starts, Relay's window steps aside and the player shows instead, so Relay never covers what is recorded, unless **Hide the window while recording** is off (REC-12). See [the window and the player](windows.md).

## The countdown

- **REC-7** With **Count down 3, 2, 1** on (the default), the player says *GET READY* and counts down *3, 2, 1*, the count on the right of its screen. Recording starts when the count reaches zero.
- **REC-8** With **Count down 3, 2, 1** off, recording starts the moment the record key is pressed.
- **REC-9** The record key or the player's **■** during the countdown cancels it.

## While recording

- **REC-10** The player shows *REC*, the macro's name as *New recording*, the number of steps recorded so far on the right, and the last step under it (*Last: Click · 812, 402*).
- **REC-11** Neither of Relay's windows takes the focus when clicked during a recording, so keystrokes keep going to the app being recorded.
- **REC-12** With **Settings → Window → Hide the window while recording** off, the window stays where it is and the player doesn't show; the window's red **record** button counts down instead.
- **REC-13** In that case an open editor fills in the steps and draws the mouse path as the recording goes. A move shows in the list once the next step after it starts; the last move shows when the recording stops.
- **REC-14** With **Settings → Sound** on, Relay clicks softly when recording starts, at each new step, and when it stops.
- **REC-15** During a recording, only the record key and the kill switch belong to Relay (and <kbd>Esc</kbd>, per REC-27). Relay's play key, its show the player key and the macros' hotkeys are let go: they reach the app being recorded and are recorded like any other key.

## What is recorded

| Rule | Input | Recorded as |
| --- | --- | --- |
| **REC-16** | Mouse movement | The cursor path, sampled up to about 60 times a second |
| **REC-17** | Mouse buttons | Left, right, middle and the two side buttons, pressed and released at an exact position |
| **REC-18** | Scroll wheel | Vertical and horizontal notches |
| **REC-19** | Keyboard | Every key pressed and released, including modifiers, with the character it typed |
| **REC-20** | Timing | When each of these happened, to the millisecond |

- **REC-21** Positions are real screen pixels across all monitors. Relay is DPI-aware, so 125% or 150% display scaling doesn't distort them.
- **REC-22** How raw input becomes steps is in [Steps](steps.md).

## Recording context

- **REC-23** Relay records the window first clicked in: its program, window class and position. Playback uses it for **Window** coordinates (see [Playback](playback.md)).
- **REC-24** Relay records the monitor layout and the Windows double-click settings, so steps are grouped as they were experienced while recording.
- **REC-25** With **Screenshot** on, Relay takes a screenshot of the screen as the recording starts, for the preview (see [The view](view.md)). Relay's own windows are left out of it.
- **REC-26** The screenshot is kept on this PC only: exported files never include it.

## What is left out

- **REC-27** <kbd>Esc</kbd> is not recorded by default: it stops the recording and is swallowed, so the app being recorded doesn't see it either.
- **REC-28** Clicks and scrolls on Relay's windows (the main window and the player) are not recorded, so pressing the player's **■** doesn't end up in the macro.
- **REC-29** The cursor's trip to the player is still recorded: it's the last move of the recording, which can be deleted. Stopping with the record key leaves no such trip.
- **REC-30** Keys typed while one of Relay's windows is in front are not recorded.
- **REC-31** The record key is not recorded, nor the <kbd>Ctrl</kbd> and <kbd>Shift</kbd> pressed with it. The same key without them is recorded: with <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>F9</kbd> set, <kbd>F9</kbd> alone is.
- **REC-32** The kill switch is not recorded, including the <kbd>Ctrl</kbd> and <kbd>Alt</kbd> pressed for it.
- **REC-33** Relay's own playback is never recorded.
- **REC-34** Input simulated by other programs is not recorded unless **Ignore simulated input** is off (REC-39).
- **REC-35** Remote Desktop, KVM switches and KVM tools such as Synergy or Barrier deliver real input as simulated input, so with **Ignore simulated input** on it is missing from the recording.
- **REC-36** Windows hides input to an app that runs as administrator from normal apps: it is recorded only when Relay runs as administrator too.

## Recording options

These are in **Settings → Recording** and apply to new recordings.

| Rule | Option | Default | Effect on a recording |
| --- | --- | --- | --- |
| **REC-37** | **Esc stops recording** | On | Off, <kbd>Esc</kbd> is recorded like any key, and the recording stops with the record key or the player's **■** instead. <kbd>Esc</kbd> still stops playback either way. |
| **REC-38** | **Capture the mouse path** | On | Records the cursor's movement between clicks. Off records only where clicks happened, and playback jumps straight there. |
| **REC-39** | **Ignore simulated input** | On | Leaves out input other programs generate. Off records it, as Remote Desktop and KVM tools need (REC-35). |
| **REC-40** | **Capture keystrokes** | On | Off records the mouse only. |
| **REC-41** | **Screenshot** | On | Takes the screenshot of REC-25. Off takes none. |
| **REC-42** | **Count down 3, 2, 1** | On | See REC-7 and REC-8. |

**Hide the window while recording** (on by default) is a window setting: see [Settings](settings.md) and REC-12.

## The input hook

- **REC-43** Windows silently removes an input hook from an app it considers too slow, for example under heavy load. If this happens during a recording, Relay notices, installs a new hook, keeps recording and says *"Windows dropped Relay's input hook; it was restarted. Check the last steps."*
- **REC-44** A few events from just before the hook was restarted may be missing from the recording.
- **REC-45** A program moving the mouse by itself, such as a remote-desktop tool, does not cause that message: Relay tells it apart from a dropped hook.

## The end of a recording

- **REC-46** A recording with no click, key or scroll (only mouse movement, however much) is thrown away rather than saved.
- **REC-47** Otherwise, when the recording stops, Relay's window comes back on the new macro with its name selected, ready to type over, and says *Recording stopped, 12 steps. Name this macro.* (with the actual count).
- **REC-48** In player mode, the player shows the new macro instead of the window coming back.
- **REC-49** A new recording is named *Recording 1*, *Recording 2* and so on until it's renamed.

## Privacy

- **REC-50** A recording stores what is typed, passwords included, as plain text in the macro file.
- **REC-51** Logs never contain what was typed: see [Installing, files and logs](install.md).

---

[Spec index](README.md)
