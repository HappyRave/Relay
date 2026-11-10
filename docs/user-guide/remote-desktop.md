# Use Relay over Remote Desktop

Record, stop and play macros over Remote Desktop, or on a PC with different display scaling.

- [Record over Remote Desktop](#record-over-remote-desktop)
- [Stop a macro over Remote Desktop](#stop-a-macro-over-remote-desktop)
- [Standalone programs over Remote Desktop](#standalone-programs-over-remote-desktop)
- [Play on a PC with other display scaling](#play-on-a-pc-with-other-display-scaling)

## Record over Remote Desktop

Remote Desktop sends your typing to Relay as simulated input. KVM tools such as Synergy do the same. By default, Relay doesn't record simulated input.

1. Open **Settings**, then **Recording**.
2. Turn off **Ignore simulated input**.
3. [Record the macro](record.md#start-and-stop-a-recording).

If it worked, the new macro has **Keys** and **Type** steps.

While **Ignore simulated input** is off, <kbd>Esc</kbd> also stops playback over Remote Desktop. Turn it on again afterwards.

More detail: [Recording: what is left out](../spec/recording.md#what-is-left-out)

## Stop a macro over Remote Desktop

Over Remote Desktop, <kbd>Esc</kbd> stops a macro only while **Ignore simulated input** is off. [**Stop everything**](stop-a-macro.md#every-way-to-stop-a-macro) (<kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>Backspace</kbd>) works too, unless Relay said at start that another program uses it. Then choose another key in **Settings**, then **Keys**.

If you [upgraded from an older Relay](keys.md#you-upgraded-from-an-older-relay), your **Stop everything** key may be <kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>End</kbd>. Remote Desktop changes that key to <kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>Delete</kbd>, so the key doesn't reach Relay. To change it:

1. Open **Settings**, then **Keys**.
2. Next to **Stop everything**, select **Change**.
3. Press <kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>Backspace</kbd>.

If it worked, the line under the key says *Set to Ctrl + Alt + Backspace.*

More detail: [Shortcuts: when a key doesn't reach Relay](../spec/keys.md#when-a-key-doesnt-reach-relay)

## Standalone programs over Remote Desktop

> [!WARNING]
> Over Remote Desktop, neither <kbd>Esc</kbd> nor <kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>End</kbd> reaches a standalone program. Select the program's **Stop** button.

A program started with `--quiet` has no window. To stop it, end it in [Task Manager](stop-a-macro.md#stop-a-standalone-program).

On a locked screen, the program plays nothing and exits with [code 6](scripts.md#exit-codes).

## Play on a PC with other display scaling

Different display scaling changes where recorded clicks land. Do one of these:

- Use a [**Find image**](find-image.md#add-a-find-image-step) step. It finds its picture at half to twice the picture's size.
- Record the macro again on that PC.

Setting **Clicks land** to **Window** doesn't help here. It only [follows a window that moved](play.md#make-clicks-follow-a-moved-window).

Scaling can also change a pixel's color. If a pixel check fails, [pick the pixel again](wait.md#wait-until-a-window-or-page-is-ready).

More detail: [Playback: Screen or Window coordinates](../spec/playback.md#screen-or-window-coordinates)

---

Previous: [Accessibility](accessibility.md) · Next: [Where Relay keeps things](files.md) · [Guide home](README.md)
