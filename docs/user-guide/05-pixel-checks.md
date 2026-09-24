# Pixel checks

A pixel check makes a macro **wait until something appears on screen**, such as a dialog, a *Done* message or a button turning green, instead of hoping a fixed wait is long enough. In the steps list it's the `IF` step:

> **IF** · Wait for pixel 1248, 680 = `#EC3013` · *timeout 5 s, else stop*

- [How a pixel check works](#how-a-pixel-check-works)
- [Adding a pixel check](#adding-a-pixel-check)
- [Adjusting it](#adjusting-it)
- [When the pixel never matches](#when-the-pixel-never-matches)
- [Choosing a good pixel](#choosing-a-good-pixel)
- [Example: wait for an export to finish](#example-wait-for-an-export-to-finish)

## How a pixel check works

When playback reaches the check:

1. The playhead **stops** and Relay reads the pixel at **X, Y** about 30 times a second.
2. As soon as its color is **close enough** to the target color, playback **continues right away** with the next step.
3. If it doesn't match within the **timeout**, playback **stops**.

"Close enough" means each of red, green and blue is within **Tolerance** of the target. With a tolerance of 8, `#EC3013` matches anything from `#E42805` to `#F4381B`.

On the timeline, the check is an `IF` block in the *Logic* lane. The block's length is only there to make it visible and clickable. When the pixel already matches, the macro doesn't spend that time waiting.

> [!TIP]
> Pausing (<kbd>F10</kbd>) during a check freezes the timeout too. It carries on counting when you resume.

## Adding a pixel check

1. **Click the step that must wait**, for example the click on a button that only appears once loading is done. That moves the playhead to it, and the check goes right before it.
2. **Get the screen ready.** Make the thing you want to wait for visible, for example open the dialog in the app.
3. Press **+ Pixel check**.

Relay inserts an 800 ms `IF` block at the playhead. It points at **where the macro's cursor is at that moment**, and takes **the color that pixel has on your screen right now**. It uses a tolerance of **8** and a timeout of **5 s**.

That's often not the pixel you want. Open the step to point it somewhere better.

## Adjusting it

<p align="center"><img src="../images/step-editor.png" alt="The pixel check editor" width="720"></p>

Click the `IF` row to open its editor:

| Field | Meaning |
| --- | --- |
| **X**, **Y** | The pixel to watch, in screen pixels |
| **Color** | The color to wait for, as `#RRGGBB`. The swatch shows it. |
| **Tolerance** | How far each color channel may be off, 0 to 255. 0 means an exact match. |
| **Timeout s** | How long to wait before giving up, in steps of 0.5 s |
| **Label** | An optional note, such as *Save dialog is open* |
| **Pick** | Points the check at a pixel on your screen (see below) |

### Pick a pixel on screen

1. Press **Pick**. The button counts down: *Point at it… 3, 2, 1*.
2. During those 3 seconds, **move the mouse over the pixel** you want to wait for. You can switch to another window.
3. When the countdown ends, Relay takes the position and the color under the cursor.

With [Window coordinates](04-playback.md#screen-or-window-coordinates), the check follows the window too: if the window moved, Relay watches the pixel at the same place in the window.

## When the pixel never matches

If the timeout runs out, Relay:

- stops playback and releases any held keys and buttons,
- shows *"Pixel check timed out at step 7; playback stopped."*,
- leaves the playhead **on that step** so you can see it highlighted.

A macro that times out has usually hit one of these:

| Cause | Fix |
| --- | --- |
| The app was slower than the timeout | Raise **Timeout** |
| The color is slightly different each time (anti-aliasing, shadows, transparency) | Raise **Tolerance** to 16 to 32, or pick a pixel in a flat area |
| The window is somewhere else | Use **Window** coordinates, or keep the window in the same place |
| Display scaling or theme changed since you picked the color | Pick again |
| HDR or protected video | These can change the colors Relay reads. Pick a pixel outside the video, or turn HDR off. |

## Choosing a good pixel

- ✅ **The middle of a solid area** of something that only appears when you're ready: a dialog's title bar, a colored button, a status icon.
- ✅ **A distinctive color**, not white or light grey, which are everywhere.
- ❌ **Edges of text or icons**, where colors blend with the background.
- ❌ **Gradients, animations and blinking cursors.**
- ❌ **Anything under the Relay widget**, since the widget covers it.

## Example: wait for an export to finish

An app shows a green check mark when an export is done, and the export takes anywhere from 2 to 20 seconds. You recorded it on a good day: click **Export**, wait, click **Close**.

1. In the editor, click the **Close** click step. The check will go right before it.
2. Run an export by hand, so the green check is on screen.
3. Press **+ Pixel check**, open the new `IF` step, press **Pick** and point at the middle of the green check.
4. Set **Timeout** to 30 s.

Now the macro never clicks **Close** too early: on a slow day it waits for the check mark, up to 30 seconds.

> [!NOTE]
> The pause you recorded between **Export** and **Close** still plays before the check, so the macro isn't faster than your recording. v1 can't shorten recorded pauses; to keep them short, record the task when the app is quick.

---

<p align="center"><a href="04-playback.md">← Playing back</a> · <a href="README.md">Contents</a> · <a href="06-triggers.md">Triggers →</a></p>
