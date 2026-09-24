# Recording

A recording captures your mouse and keyboard until you stop it. Relay saves it to the Library automatically and opens it in the editor.

- [Start and stop](#start-and-stop)
- [What gets recorded](#what-gets-recorded)
- [What doesn't get recorded](#what-doesnt-get-recorded)
- [While you record](#while-you-record)
- [Recording options](#recording-options)
- [Tips for recordings that replay well](#tips-for-recordings-that-replay-well)
- [Privacy](#privacy)

## Start and stop

| To… | Press |
| --- | --- |
| Start recording | <kbd>F9</kbd>, the red **record** button, or **Record** in the tray menu |
| Stop and keep the recording | <kbd>F9</kbd> again, <kbd>Esc</kbd>, or the **stop** button |
| Cancel the countdown | <kbd>F9</kbd> again or the **stop** button |
| Stop everything | <kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>End</kbd> (the [kill switch](04-playback.md#the-kill-switch)) |

With the **3-second countdown** on (the default), a big number counts down over the preview first. Recording starts when it reaches zero.

<p align="center"><img src="../images/countdown.png" alt="The countdown before a recording" width="600"></p>

When you stop, the new macro is named *Recording 1*, *Recording 2* and so on, and goes to the top of the Library.

> [!NOTE]
> A recording with nothing in it (no click, key or scroll, just a twitch of the mouse) is thrown away rather than saved.

## What gets recorded

| Input | Recorded as |
| --- | --- |
| Mouse movement | The cursor path, sampled up to about 60 times a second |
| Mouse buttons | Left, right, middle and the two side buttons, pressed and released at an exact position |
| Scroll wheel | Vertical and horizontal notches |
| Keyboard | Every key pressed and released, including modifiers, with the character it typed |
| Timing | When each of these happened, to the millisecond |

Positions are real screen pixels across all your monitors. Relay is DPI-aware, so 125% or 150% display scaling doesn't distort them.

Relay also remembers a little about the recording's context:

- **The window you first clicked in** (the program, its window class and position). Playback uses it for [Window coordinates](04-playback.md#screen-or-window-coordinates).
- **Your monitor layout and double-click settings**, so steps are grouped the same way you experienced them.

## What doesn't get recorded

Relay leaves out anything that would break the macro or loop back into itself:

- **Clicks and scrolls on the Relay widget.** You can press stop or look at the steps without them ending up in the macro.
- **Keys typed while Relay's own window is focused.**
- **<kbd>F9</kbd>**, the key that stops the recording.
- **<kbd>Esc</kbd>.** It stops the recording and is swallowed, so the app you're recording doesn't see it either.
- **The kill switch** (<kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>End</kbd>), including the <kbd>Ctrl</kbd> and <kbd>Alt</kbd> you pressed for it.
- **Relay's own playback,** always.
- **Input simulated by other programs,** unless you turn off [Ignore simulated input](#recording-options).

> [!TIP]
> Need to close a dialog in your macro? Since <kbd>Esc</kbd> can't be recorded, click the dialog's **Cancel** or **×** button instead. This is a known limitation of v1.

## While you record

- The steps list fills in as you go, and the preview draws your mouse path live.
- The clock shows how long you've been recording, and the transport reads *of recording*.
- The widget **doesn't take the focus** when you click it during a session, so your keystrokes keep going to the app you're recording.
- Relay's other hotkeys are released while you record, so <kbd>F10</kbd> and <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>M</kbd> reach the app you're recording like any other key.

If Windows removes Relay's input hook in the middle of a recording (it does this silently to apps it thinks are too slow, for example under heavy load), Relay notices, installs a new one and tells you: *"Windows dropped Relay's input hook; it was restarted. Check the last steps."*

## Recording options

These are in **Settings → Recording** and apply to new recordings.

| Option | Default | What it does |
| --- | --- | --- |
| **Capture mouse path** | On | Records the cursor's movement between clicks. Off records only where you clicked, and playback jumps straight there. |
| **Capture keystrokes** | On | Off records the mouse only. |
| **3-second countdown** | On | Off starts recording the moment you press <kbd>F9</kbd>. |
| **Ignore simulated input** | On | Leaves out input other programs generate. **Turn it off for remote-desktop and KVM tools** (such as Synergy, Barrier or some RDP setups): they deliver your real typing as simulated input, and it would be missing otherwise. |

## Tips for recordings that replay well

1. **Start from a known state.** Open the window you'll work in, in the same place and size it will be when the macro runs.
2. **Prefer the keyboard over the mouse** where you can. <kbd>Ctrl</kbd>+<kbd>S</kbd> works wherever the Save button is.
3. **Don't rush.** The macro replays your timing. If an app needs a moment to open a dialog, give it that moment while recording, or add a [wait](03-editing.md#waits) or a [pixel check](05-pixel-checks.md) afterwards.
4. **Keep the widget out of the way.** Clicks that land on the widget aren't recorded, so drag it aside before you start if you need to click where it is.
5. **Trim afterwards.** Delete stray clicks and extra steps in the [editor](03-editing.md) rather than recording again.

## Privacy

> [!WARNING]
> **A recording stores what you type, passwords included.** It's saved as plain text in the macro file. Stop the recording before typing anything secret, or delete that step afterwards.

Everything stays on your PC, and Relay never connects to the internet. The [diagnostic logs](08-settings.md#logs) record what Relay did (a recording started, a trigger fired), never what you typed.

---

<p align="center"><a href="01-getting-started.md">← Getting started</a> · <a href="README.md">Contents</a> · <a href="03-editing.md">Editing steps →</a></p>
