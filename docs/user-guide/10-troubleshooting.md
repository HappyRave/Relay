# Troubleshooting and FAQ

Find your problem below. If it isn't here, [open an issue](https://github.com/HappyRave/Relay/issues) and attach that day's [log](09-settings.md#logs).

Error messages stay at the bottom of the window until you close them (×), so you don't miss one; other messages go away after a few seconds. While the window is hidden, the player shows the last error until you dismiss it.

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

- the app's window is where it was when you recorded. Maximize it, or set the macro's [**Clicks land**](04-playback.md#screen-or-window-coordinates) to **Window**.
- your monitor layout and display scaling are the same as when you recorded.
- the app is ready. At 2× or 4×, or when the app is slow today, clicks land before the window is there. Add a [pixel check](05-pixel-checks.md) or a [wait](03-editing.md#waits).

When the thing to click isn't always in the same place, replace the click with a [Find image](06-find-image.md) step: it clicks the button wherever it is.

</details>

<details>
<summary><b>Nothing happens in one particular app</b></summary>

The app probably runs **as administrator**. Relay shows *"The app in front runs as administrator, so Windows blocks Relay's input to it."* Windows doesn't let a normal app control an elevated one. Run Relay as administrator too: quit it from the tray, then right-click Relay → **Run as administrator**.

Some games and anti-cheat software ignore simulated input entirely. Relay can't work around that.

</details>

<details>
<summary><b>The macro types the wrong characters</b></summary>

Relay replays **physical keys**, like your fingers would. If the keyboard layout is different from when you recorded (AZERTY versus QWERTY, or another input language selected in the taskbar), the same keys type different characters. Switch back to the layout you recorded with, or record the macro again. Or select the typed text and press **Make editable**: a [Text step](03-editing.md#text-that-changes-each-run) types characters, whatever the layout.

</details>

<details>
<summary><b>The macro typed into Relay instead of my app</b></summary>

When you press **Play** in Relay, it gives the keyboard back to the app you used just before. If that app was closed or minimized in the meantime, the focus can end up somewhere else. Click in the target app first and start the macro with your Play / pause key, or the player's **▶** (the player never takes the keyboard), or add a click in the app at the start of the macro.

</details>

<details>
<summary><b>Playback stopped by itself</b></summary>

The message at the bottom of the window (or on the player) says why:

- **You pressed a key** or moved to another app with the keyboard, and *Stop on key* is on. Turn it off in the macro's [Play options](04-playback.md#stop-on-key-press) if you need to type during playback.
- *"Pixel check timed out at step N"*. See [When the pixel never matches](05-pixel-checks.md#when-the-pixel-never-matches).
- *"Image not found at step N"*. See [When the image isn't found](06-find-image.md#when-the-image-isnt-found).
- The **kill switch** was pressed. Triggers are now paused too.

</details>

<details>
<summary><b>The macro is slower than it needs to be</b></summary>

It replays the pauses you made while recording. Choose **Shorten pauses** in the steps list's **Steps options** menu (**⋯**) to shorten every pause over a second, or set **Pause before** on a step. See [Pauses](03-editing.md#pauses).

</details>

<details>
<summary><b>I deleted or changed the wrong step</b></summary>

Press **Undo** in the editor's top bar, or <kbd>Ctrl</kbd>+<kbd>Z</kbd>. See [Undo and redo](03-editing.md#undo-and-redo). The history lasts until you quit Relay.

</details>

<details>
<summary><b>Timing feels a little off</b></summary>

*Vary timing* may be on: it shifts steps by up to the time it says (±40 ms, say). Turn it off in the macro's Play options to replay the exact timing. Relay's own timing is precise to about a millisecond.

</details>

## Recording

<details>
<summary><b>Some clicks or keys are missing from the recording</b></summary>

- **Clicks on Relay's player or window** are never recorded. While you record, the window steps aside and only the player shows: move it out of the way if it covers what you click.
- **Your Record and Play / pause keys and the kill switch** are never recorded, and neither is <kbd>Esc</kbd> unless you turn off **Settings → Recording → Esc stops recording**. See [What doesn't get recorded](02-recording.md#what-doesnt-get-recorded).
- **Using Remote Desktop, a KVM switch or software like Synergy or Barrier?** They send your input as *simulated* input, which Relay ignores by default. Turn off **Settings → Recording → Ignore simulated input**.
- **Typing in an app that runs as administrator?** Windows hides that input from normal apps. Run Relay as administrator.

</details>

<details>
<summary><b>"Windows dropped Relay's input hook; it was restarted"</b></summary>

Windows sometimes removes an input hook when the PC is very busy. Relay noticed, put a new one in place and kept recording, but a few events from just before may be missing. Check the last steps, and record again if something's off. (Relay tells this apart from a program moving the mouse by itself, such as a remote-desktop tool, which doesn't cause the message.)

</details>

<details>
<summary><b>My recording disappeared</b></summary>

A recording with nothing in it (no click, key or scroll: only mouse movement, however much) isn't saved. Otherwise, when the recording stops, the window comes back on the new macro and says *Recording stopped, 12 steps. Name this macro.* It's in the [Library](08-library.md) from then on.

</details>

## Hotkeys and triggers

<details>
<summary><b>Relay's keys do nothing</b></summary>

- **Check which keys are set** in [Settings → Keys](09-settings.md#keys). A new install uses <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>F9</kbd> to record and <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>F10</kbd> to play; one set to **None** does nothing.
- **Another program may have the key.** Relay says so once per run: *Another program uses Ctrl + Alt + Backspace, so Relay's “Stop everything” key doesn't work. Choose another in Settings → Keys.*
- **During a recording**, only the Record key and the kill switch belong to Relay. **During playback**, only the Play / pause key, <kbd>Esc</kbd> and the kill switch do.
- On a laptop, you may need <kbd>Fn</kbd> with the function keys.

</details>

<details>
<summary><b>The kill switch does nothing</b></summary>

- **Another program has it.** Relay says *Another program uses …, so Relay's “Stop everything” key doesn't work.* Choose another combination in Settings → Keys.
- **In a Remote Desktop session**, the remote-desktop client takes some combinations for itself: <kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>End</kbd> becomes <kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>Del</kbd> in the remote PC. Settings → Keys warns about these when you set one. <kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>Backspace</kbd>, the new default, works over Remote Desktop.
- **An exported program** always uses <kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>End</kbd>, whatever Relay's kill switch is. See [Exported programs](08-library.md#exported-programs).

<kbd>Esc</kbd> and the player's **■** stop a recording or playback too.

</details>

<details>
<summary><b>Setting a key warns that it types a character</b></summary>

On many keyboard layouts, <kbd>Ctrl</kbd>+<kbd>Alt</kbd> works as <kbd>AltGr</kbd>: <kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>E</kbd> types *€*, for example. While Relay uses such a combination, you can't type that character. Choose a combination Relay doesn't warn about, such as one with <kbd>Ctrl</kbd>+<kbd>Shift</kbd>.

</details>

<details>
<summary><b>A macro hotkey says "taken by another app"</b></summary>

Another running program registered the same combination first. Choose another combination, or close that program and set the hotkey again.

</details>

<details>
<summary><b>A trigger didn't run</b></summary>

Check each of these:

1. **Is Relay running?** Triggers need Relay in the tray. Keep *Close to the tray* on and turn on *Start with Windows*.
2. **Are triggers paused?** After the kill switch, the Library and the macro's **Runs automatically** say *Triggers are paused*. Press **Resume**, or check **Triggers active** in the tray menu.
3. **Was Relay busy?** A trigger doesn't interrupt a recording or another macro. Relay says *Skipped “…”: Relay was busy*, and [Problems](08-library.md#activity) lists it.
4. **Was the screen locked?** Triggers are skipped on the lock screen.
5. **Schedule:** was the PC asleep at that time? Missed runs are skipped, not made up.
6. **App start:** was the program already running? The trigger fires when it starts, not while it runs.
7. **Pixel:** did the pixel *change* to the color? A pixel that already had the color when you turned the trigger on doesn't count until it changes and comes back.
8. **Image:** did the image *appear*? One already on screen when you turned the trigger on doesn't count until it goes away and comes back. Press **Test** to see how well it matches now.

</details>

## The window

<details>
<summary><b>I can't find Relay's window</b></summary>

- **It's in the tray.** Click Relay's tray icon (it may be under the **^** arrow in the taskbar). Starting Relay again also brings it back.
- **It's in player mode**, and only the player shows. Press the player's **Open the window**, or your Show the player key (<kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>M</kbd> by default).
- **It's recording**, so the window stepped aside. Stop the recording with the player's **●**, <kbd>Esc</kbd> or your Record key: the window comes back.
- If it was on a monitor that's gone, it comes back on the nearest one.

</details>

<details>
<summary><b>The window disappears while a macro plays</b></summary>

**Settings → Window → Hide the window while playing** is on: the player stands in for it. Turn it off to keep the window during playback.

</details>

<details>
<summary><b>The window or the text is too small or too big</b></summary>

Resize the window from its edges, and set **Settings → Appearance → Text size** (and **Player size** for the player). Relay shrinks the window to fit small screens, and follows your display scaling (Settings → System → Display → Scale).

</details>

<details>
<summary><b>The player covers what the macro clicks</b></summary>

Drag it by its grip somewhere else: Relay remembers where. Turn off **Settings → Window → Keep the player on top**, or set it to **While working**, if it's in the way the rest of the time.

</details>

## Installing

<details>
<summary><b>"Windows protected your PC"</b></summary>

The installer isn't code-signed yet, so SmartScreen doesn't recognize it. Click **More info → Run anyway**.

</details>

<details>
<summary><b>"Windows protected your PC" when running an exported program</b></summary>

[Exported programs](08-library.md#exported-programs) aren't code-signed, so SmartScreen warns about one that was downloaded or received from another PC. Click **More info → Run anyway**. A program you exported on this PC runs without the warning.

</details>

<details>
<summary><b>My antivirus quarantined an exported program</b></summary>

A program that presses keys and moves the mouse by itself can look suspicious to an antivirus. If you trust where it came from, restore it from quarantine and allow it, or export the macro again. Or share the `.rly` instead, for someone who has Relay.

Some antivirus programs also watch what a program does while it runs, such as Bitdefender's *Advanced Threat Defense*. They're most wary of a program that a script starts from a temporary folder. If one stops an exported program:

- Keep the program in a folder of its own, such as `Documents\Macros`, rather than in a temporary or download folder.
- Add that folder as an exception in the antivirus.
- Check the quarantine for `relay.exe` too: when the antivirus cleans up, it may remove the program that wrote the file as well. Restore Relay from there, or reinstall it.

</details>

## FAQ

<details>
<summary><b>Does Relay send anything over the internet?</b></summary>

No. Relay doesn't connect to the internet at all. Macros, settings and logs stay in `%APPDATA%\Relay`.

</details>

<details>
<summary><b>Can I record passwords?</b></summary>

You can, but you shouldn't: a macro stores what you typed as plain text. Stop recording before typing a password, or delete that *Type* step.

</details>

<details>
<summary><b>Can a macro run while the PC is locked?</b></summary>

No. Windows doesn't accept simulated input on the lock screen, so triggers are skipped while it's locked.

</details>

<details>
<summary><b>Does Relay work on macOS or Linux?</b></summary>

Not yet. Relay is built so other systems can be added later, but for now it's Windows only.

</details>

<details>
<summary><b>Can I edit a macro file by hand?</b></summary>

Yes. `.rly` files are JSON, described in [File formats](../engineering/file-formats.md). Quit Relay first, or import the edited file as a new macro.

</details>

<details>
<summary><b>Can I run a macro on a PC without Relay?</b></summary>

Yes: export it as a **Standalone program**, a small `.exe` that plays it. See [Exported programs](08-library.md#exported-programs).

</details>

<details>
<summary><b>Can I export to AutoHotkey?</b></summary>

Yes: export it as **AutoHotkey v2**, a script that plays it with AutoHotkey, one readable block per step. See [AutoHotkey scripts](08-library.md#autohotkey-scripts).

</details>

---

<p align="center"><a href="09-settings.md">← Settings, tray and window</a> · <a href="README.md">Contents</a> · <a href="keyboard-shortcuts.md">Keyboard shortcuts →</a></p>
