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

Your **record key** is <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>F9</kbd> unless you chose another (<kbd>F9</kbd> if you upgraded from an older Relay). You can change it, or set it to *None*, in **Settings → Keys**.

| To… | Press |
| --- | --- |
| Start recording | Your record key, the red **record** button (in the Library, the editor or the player), or **Record** in the tray menu |
| Stop and keep the recording | Your record key again, <kbd>Esc</kbd> (unless you [record Esc](#recording-esc)), or **■** on the player |
| Cancel the countdown | Your record key again, or **■** on the player |
| Stop everything | The kill switch, <kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>Backspace</kbd> by default (see [The kill switch](04-playback.md#the-kill-switch)) |

Relay's window **steps aside** and the player shows instead, so Relay never covers what you record. With **Count down 3, 2, 1** on (the default), the player says *GET READY* and counts down first. Recording starts when it reaches zero.

<p align="center"><img src="../images/countdown.png" alt="The player counting down before a recording" width="600"></p>

When you stop, Relay's window comes back on the new macro, with its name selected so you can type a better one, and says *Recording stopped, 12 steps. Name this macro.* (In [player mode](README.md#which-one-shows), the player shows the new macro instead.) The macro is named *Recording 1*, *Recording 2* and so on until you rename it.

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
- **A screenshot of your screen** as the recording starts, so the preview can draw the macro over what you saw. Relay leaves its own windows out of it. It's kept **on this PC only**: exported files never include it. Turn it off with **Settings → Recording → Screenshot**.

## What doesn't get recorded

Relay leaves out anything that would break the macro or loop back into itself:

- **Clicks and scrolls on Relay's windows**, the main window and the player. You can press **■** without it ending up in the macro. The cursor's trip to the player is still recorded: it's the last [move](03-editing.md#moves) in the steps, and you can delete it.
- **Keys typed while one of Relay's windows is in front.**
- **Your record key**, which stops the recording, and the <kbd>Ctrl</kbd> and <kbd>Shift</kbd> you pressed with it. The same key without them is recorded: with <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>F9</kbd> set, <kbd>F9</kbd> alone is.
- **<kbd>Esc</kbd>**, by default. It stops the recording and is swallowed, so the app you're recording doesn't see it either. You can [record it instead](#recording-esc).
- **The kill switch**, including the <kbd>Ctrl</kbd> and <kbd>Alt</kbd> you pressed for it.
- **Relay's own playback,** always.
- **Input simulated by other programs,** unless you turn off [Ignore simulated input](#recording-options).

Relay's other keys are let go while you record: your play key, your show the player key and your macros' hotkeys reach the app you're recording, and are recorded like any other key.

### Recording Esc

Need <kbd>Esc</kbd> in your macro, for example to close a dialog? Turn off **Settings → Recording → Esc stops recording**. <kbd>Esc</kbd> is then recorded like any other key, and you stop the recording with your record key or the player's **■**. <kbd>Esc</kbd> still stops playback either way.

## While you record

- The player shows *REC*, the macro as *New recording*, how many steps it has so far, and the last one (*Last: Click · 812, 402*).
- Neither of Relay's windows **takes the focus** when you click it during a session, so your keystrokes keep going to the app you're recording.
- With **Settings → Sound** on, Relay clicks softly when recording starts, at each new step, and when it stops.
- With **Settings → Window → Hide the window while recording** off, the window stays where it is and the player doesn't show. The window's red **record** button counts down instead. An open editor then fills in the steps and draws the mouse path as you go. A move shows up in the list once the next step after it starts; the last one shows up when you stop.

If Windows removes Relay's input hook in the middle of a recording (it does this silently to apps it thinks are too slow, for example under heavy load), Relay notices, installs a new one and tells you: *"Windows dropped Relay's input hook; it was restarted. Check the last steps."*

## Recording options

These are in **Settings → Recording** and apply to new recordings.

| Option | Default | What it does |
| --- | --- | --- |
| **Capture the mouse path** | On | Records the cursor's movement between clicks. Off records only where you clicked, and playback jumps straight there. |
| **Capture keystrokes** | On | Off records the mouse only. |
| **Screenshot** | On | Saves a picture of your screen as recording starts, for the [preview](03-editing.md#the-preview). Kept on this PC only, never in exports. |
| **Count down 3, 2, 1** | On | Off starts recording the moment you press your record key. |
| **Esc stops recording** | On | Off records <kbd>Esc</kbd> like any key. Stop with your record key or the player's **■** instead. |
| **Ignore simulated input** | On | Leaves out input other programs generate. **Turn it off for Remote Desktop and KVM tools** (such as Synergy or Barrier): they deliver your real typing as simulated input, and it would be missing otherwise. |

**Settings → Window → Hide the window while recording** (on) decides whether the window steps aside. See [Settings](09-settings.md#window).

## Tips for recordings that replay well

1. **Start from a known state.** Open the window you'll work in, in the same place and size it will be when the macro runs.
2. **Prefer the keyboard over the mouse** where you can. <kbd>Ctrl</kbd>+<kbd>S</kbd> works wherever the Save button is.
3. **Don't rush, and don't worry about being slow.** The macro replays your timing. If an app needs a moment to open a dialog, give it that moment while recording, or add a [pixel check](05-pixel-checks.md) afterwards. Pauses where you hesitated can be [shortened](03-editing.md#pauses) later, and slow or shaky mouse moves [sped up or smoothed](03-editing.md#moves).
4. **Keep the player out of the way.** Clicks that land on it aren't recorded, so drag it aside before you start if you need to click where it is.
5. **Stop with the key, not the mouse.** Your record key leaves no trip to the player at the end.
6. **Trim afterwards.** Delete stray clicks and extra steps in the [editor](03-editing.md) rather than recording again.

## Privacy

> [!WARNING]
> **A recording stores what you type, passwords included.** It's saved as plain text in the macro file. Stop the recording before typing anything secret, or delete that step afterwards.

Everything stays on your PC, and Relay never connects to the internet. The [diagnostic logs](09-settings.md#logs) record what Relay did (a recording started, a trigger fired), never what you typed.

---

<p align="center"><a href="01-getting-started.md">← Getting started</a> · <a href="README.md">Contents</a> · <a href="03-editing.md">Editing steps →</a></p>
