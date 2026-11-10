# Stop a macro

Press <kbd>Esc</kbd> to stop a macro, in any app. Over [Remote Desktop](remote-desktop.md#stop-a-macro-over-remote-desktop), <kbd>Esc</kbd> needs **Ignore simulated input** off.

- [Every way to stop a macro](#every-way-to-stop-a-macro)
- [Stop everything, then turn triggers back on](#stop-everything-then-turn-triggers-back-on)
- [When Esc is used by your game or app](#when-esc-is-used-by-your-game-or-app)
- [Stop a standalone program](#stop-a-standalone-program)
- [Good to know](#good-to-know)

## Every way to stop a macro

| Way | How |
| --- | --- |
| <kbd>Esc</kbd> | Press it. |
| The [player](glossary.md#player) | Select **Stop** (the square). |
| Any key | Press any key, if **Stop on key** is on. |
| The editor | Select **Stop** in the top bar. |
| The [tray](glossary.md#tray) | Select **Stop** in Relay's menu. |
| **Stop everything** (the kill switch) | Press its key. It also pauses [triggers](glossary.md#trigger). |

**Stop on key** is a [play option](play.md#choose-whether-a-key-press-stops-it) of each macro, on by default. <kbd>Shift</kbd>, <kbd>Ctrl</kbd>, <kbd>Alt</kbd> or <kbd>Win</kbd> alone doesn't stop the macro.

On a new install, the **Stop everything** key is <kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>Backspace</kbd>. After an [upgrade from an older Relay](keys.md#you-upgraded-from-an-older-relay), it can be <kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>End</kbd>.

> [!WARNING]
> The **Play / pause** key pauses a macro. It doesn't stop it.

More detail: [Playback: play, pause and stop](../spec/playback.md#play-pause-and-stop)

## Stop everything, then turn triggers back on

**Stop everything** stops any playback or recording, and pauses every trigger. A stopped recording keeps what you recorded.

1. Press the **Stop everything** key.
2. Open the **Library**.
3. Select **Resume**.

If it worked, the Library no longer says *Triggers are paused (the kill switch).*

Or select **Triggers active** in Relay's tray menu.

More detail: [Playback: the kill switch](../spec/playback.md#the-kill-switch)

## When Esc is used by your game or app

While a macro plays, <kbd>Esc</kbd> stops it and doesn't reach your app. While you record, <kbd>Esc</kbd> stops the recording by default. At other times, <kbd>Esc</kbd> goes to your app.

To stop without <kbd>Esc</kbd>, select a **Stop** button. **Stop everything** also works, but it pauses your triggers too.

To record <kbd>Esc</kbd>, read [Record the Esc key](record.md#record-the-esc-key).

## Stop a standalone program

A [standalone program](glossary.md#standalone-program) stops when you:

- press <kbd>Esc</kbd>, or select its **Stop** button
- press <kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>End</kbd>, even if you changed Relay's keys
- press any key, if **Stop on key** was on when you exported it

A program started with `--quiet` has no window. Use one of these keys, or **End task** in Task Manager (<kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>Esc</kbd>).

> [!WARNING]
> Over Remote Desktop, neither key reaches the program. Select its **Stop** button, or **End task** in Task Manager. The command `taskkill /im <name>.exe` also works.

More detail: [Export and import: exported programs](../spec/export-import.md#exported-programs)

## Good to know

- A stop releases every key and button the macro held down.
- After a stop, the [playhead](glossary.md#playhead) goes back to the start of the macro.

Something not working? Read [Problems with keys and triggers](troubleshooting.md#problems-with-keys-and-triggers).

---

Previous: [Record a macro](record.md) · Next: [Edit the steps](edit-steps.md) · [Guide home](README.md)
