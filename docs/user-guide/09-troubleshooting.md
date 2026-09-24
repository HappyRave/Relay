# Troubleshooting and FAQ

Find your problem below. If it isn't here, [open an issue](https://github.com/HappyRave/Relay/issues) and attach that day's [log](08-settings.md#logs).

- [Playback](#playback)
- [Recording](#recording)
- [Hotkeys and triggers](#hotkeys-and-triggers)
- [The window](#the-window)
- [Installing](#installing)
- [FAQ](#faq)

## Playback

<details>
<summary><b>The macro clicks in the wrong place</b></summary>

A macro clicks at the screen positions you recorded. Check that:

- the app's window is where it was when you recorded. Maximize it, or switch the macro to [Window coordinates](04-playback.md#screen-or-window-coordinates).
- your monitor layout and display scaling are the same as when you recorded.
- the app is ready. At 2× or 4×, or when the app is slow today, clicks land before the window is there. Add a [pixel check](05-pixel-checks.md) or a [wait](03-editing.md#waits).

</details>

<details>
<summary><b>Nothing happens in one particular app</b></summary>

The app probably runs **as administrator**. Relay shows *"The app in front runs as administrator, so Windows blocks Relay's input to it."* Windows doesn't let a normal app control an elevated one. Run Relay as administrator too: quit it from the tray, then right-click Relay → **Run as administrator**.

Some games and anti-cheat software ignore simulated input entirely. Relay can't work around that.

</details>

<details>
<summary><b>The macro types the wrong characters</b></summary>

Relay replays **physical keys**, like your fingers would. If the keyboard layout is different from when you recorded (AZERTY versus QWERTY, or another input language selected in the taskbar), the same keys type different characters. Switch back to the layout you recorded with, or record the macro again.

</details>

<details>
<summary><b>The macro typed into Relay instead of my app</b></summary>

When you press play in Relay, it gives the keyboard back to the app you used just before. If that app was closed or minimized in the meantime, the focus can end up somewhere else. Click in the target app first and start the macro with <kbd>F10</kbd>, or add a click in the app at the start of the macro.

</details>

<details>
<summary><b>Playback stopped by itself</b></summary>

The message at the bottom of the panel says why:

- **You pressed a key** or moved to another app with the keyboard, and *Stop on key press* is on. Turn it off in [Settings → Playback](08-settings.md#playback-for-the-open-macro) if you need to type during playback.
- *"Pixel check timed out at step N"*. See [When the pixel never matches](05-pixel-checks.md#when-the-pixel-never-matches).
- The **kill switch** was pressed. Triggers are now paused too.

</details>

<details>
<summary><b>The macro is slower than it needs to be</b></summary>

It replays the pauses you made while recording. Press **Trim pauses** in the Steps tab to shorten every pause over a second, or set **Pause before** on a single step. See [Pauses](03-editing.md#pauses).

</details>

<details>
<summary><b>I deleted or changed the wrong step</b></summary>

Press **Undo** in the header or <kbd>Ctrl</kbd>+<kbd>Z</kbd>. See [Undo and redo](03-editing.md#undo-and-redo). The history lasts until you quit Relay.

</details>

<details>
<summary><b>Timing feels a little off</b></summary>

*Humanize* is on by default and shifts steps by up to ±40 ms. Turn it off in Settings → Playback to replay the exact timing. Relay's own timing is precise to about a millisecond.

</details>

## Recording

<details>
<summary><b>Some clicks or keys are missing from the recording</b></summary>

- **Clicks on the Relay widget** are never recorded. Move the widget out of the way before recording.
- **<kbd>F9</kbd> and the kill switch** are never recorded, and neither is <kbd>Esc</kbd> unless you turn off **Settings → Recording → Esc stops recording**. See [What doesn't get recorded](02-recording.md#what-doesnt-get-recorded).
- **Using remote desktop, a KVM switch or software like Synergy or Barrier?** They send your input as *simulated* input, which Relay ignores by default. Turn off **Settings → Recording → Ignore simulated input**.
- **Typing in an app that runs as administrator?** Windows hides that input from normal apps. Run Relay as administrator.

</details>

<details>
<summary><b>"Windows dropped Relay's input hook; it was restarted"</b></summary>

Windows sometimes removes an input hook when the PC is very busy. Relay noticed, put a new one in place and kept recording, but a few events from just before may be missing. Check the last steps, and record again if something's off.

</details>

<details>
<summary><b>My recording disappeared</b></summary>

A recording with nothing in it (no click, key or scroll, and barely any mouse movement) isn't saved. Otherwise, every recording goes to the top of the [Library](07-library.md).

</details>

## Hotkeys and triggers

<details>
<summary><b>F9 or F10 does nothing</b></summary>

- Another program may have taken the key first. Relay reports *"Couldn't register the Record hotkey"* when it starts.
- During a recording, only <kbd>F9</kbd> and the kill switch belong to Relay. During playback, only <kbd>F10</kbd>, <kbd>Esc</kbd> and the kill switch do.
- On a laptop, you may need <kbd>Fn</kbd> with the function keys.

</details>

<details>
<summary><b>A macro hotkey says "taken by another app"</b></summary>

Another running program registered the same combination first. Choose another combination, or close that program and set the hotkey again.

</details>

<details>
<summary><b>A trigger didn't run</b></summary>

Check each of these:

1. **Is Relay running?** Triggers need Relay in the tray. Keep *Close to tray* on and turn on *Start with Windows*.
2. **Are triggers paused?** The Triggers tab shows a banner after the kill switch. Press **Resume**, or check **Triggers active** in the tray menu.
3. **Was Relay busy?** A trigger doesn't interrupt a recording or another macro. Relay says *Skipped "…": Relay was busy*.
4. **Was the screen locked?** Triggers are skipped on the lock screen.
5. **Schedule:** was the PC asleep at that time? Missed runs are skipped, not made up.
6. **App launch:** was the program already running? The trigger fires when it starts, not while it runs.
7. **Pixel:** did the pixel *change* to the color? A pixel that already had the color when you turned the trigger on doesn't count until it changes and comes back.

</details>

## The window

<details>
<summary><b>I can't find the widget</b></summary>

Click Relay's tray icon (it may be under the **^** arrow in the taskbar). If you started Relay again, it brings the existing widget forward. If the widget was on a monitor that's gone, it comes back on the main monitor.

</details>

<details>
<summary><b>The widget is too small or too big</b></summary>

Relay shrinks the editor to fit small screens. It follows your display scaling (Settings → System → Display → Scale), so changing that changes the widget's size.

</details>

## Installing

<details>
<summary><b>"Windows protected your PC"</b></summary>

The installer isn't code-signed yet, so SmartScreen doesn't recognize it. Click **More info → Run anyway**.

</details>

## FAQ

<details>
<summary><b>Does Relay send anything over the internet?</b></summary>

No. Relay doesn't connect to the internet at all. Macros, settings and logs stay in `%APPDATA%\Relay`.

</details>

<details>
<summary><b>Can I record passwords?</b></summary>

You can, but you shouldn't: a macro stores what you typed as plain text. Stop recording before typing a password, or delete that `TYPE` step.

</details>

<details>
<summary><b>Can a macro run while the PC is locked?</b></summary>

No. Windows doesn't accept simulated input on the lock screen, so triggers are skipped while it's locked.

</details>

<details>
<summary><b>Does Relay work on macOS or Linux?</b></summary>

Not yet. Relay is built so other systems can be added later, but v1 is Windows only.

</details>

<details>
<summary><b>Can I edit a macro file by hand?</b></summary>

Yes. `.rly` files are JSON, described in [File formats](../engineering/file-formats.md). Quit Relay first, or import the edited file as a new macro.

</details>

<details>
<summary><b>Can I export to AutoHotkey or a standalone .exe?</b></summary>

Not in v1. Both are planned, and are shown as *Coming later* in the Export dialog.

</details>

---

<p align="center"><a href="08-settings.md">← Settings, tray and window</a> · <a href="README.md">Contents</a> · <a href="keyboard-shortcuts.md">Keyboard shortcuts →</a></p>
