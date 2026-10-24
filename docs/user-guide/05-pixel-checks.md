# Pixel checks

A pixel check makes a macro **wait until something appears on screen**, such as a dialog, a *Done* message or a button turning green, instead of hoping a fixed wait is long enough. In the steps list it's a *Wait for* step:

> **Wait for** · pixel 1248, 680 · Save dialog is open &nbsp;&nbsp; *≤ 5 s*

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
3. If it doesn't match within the time it **gives up after**, playback **stops**.

"Close enough" means each of red, green and blue is within **Tolerance** of the target. With a tolerance of 8, `#EC3013` matches anything from `#E42805` to `#F4381B`.

On the timeline, the check is a *Wait for* block in the bottom lane, with the other logic steps. The block's length is only there to make it visible and clickable. When the pixel already matches, the macro doesn't spend that time waiting.

> [!TIP]
> Pausing during a check (**Pause**, or your [Play / pause key](keyboard-shortcuts.md)) freezes the timeout too. It carries on counting when you resume.

## Adding a pixel check

1. **Select the step the macro should wait after**, usually the click that starts something slow. The insertion line (*+ insert here*) shows where the check goes: right after it.
2. **Get the screen ready.** Make the thing you want to wait for visible, for example open the dialog in the app.
3. Press **Add** in the steps list (or <kbd>Insert</kbd>) and choose **Wait for pixel**. The **+** that shows between two rows opens the same menu, to add the check right there.

Relay inserts an 800 ms check after that step. It points at **where the macro's cursor is at that moment**, and takes **the color that pixel has on your screen right now**. It uses a tolerance of **8** and gives up after **5 s**. If Relay can't read the screen there (the screen is locked, say), it inserts nothing and tells you: *Couldn't read the screen at x, y*.

That's often not the pixel you want: press **Pick from screen** to point it somewhere better.

## Adjusting it

<p align="center"><img src="../images/step-editor.png" alt="The inspector's Step tab on a pixel check" width="720"></p>

Select the *Wait for* row: the inspector's **Step** tab shows its fields.

| Field | Meaning |
| --- | --- |
| **Label** | An optional note, such as *Save dialog is open*. The steps list shows it. |
| **Pause before** | The time between the step before and this one |
| **Position** | **X** and **Y**: the pixel to watch, in screen pixels |
| **Pick from screen** | Points the check at a pixel on your screen (see below) |
| **Color** | The color to wait for, as `#RRGGBB`. The swatch shows it. |
| **Tolerance** | How far each color channel may be off, 0 to 255. 0 means an exact match. |
| **Gives up after** | How long to wait before stopping, at least 0.5 s, in steps of 0.5 s |

Each change saves at once. **Delete step** at the bottom removes the check. While a macro plays, the fields are off.

### Pick from screen

Press **Pick from screen**. Relay's windows step aside, and the picker shows over the live screen:

- a **loupe** follows the cursor: the 11 × 11 pixels around it, magnified, with the coordinates and the color under them (*1248, 680 · #EC3013*);
- a bar at the top of the screen says what to do, and shows **LIVE**.

| To | Do |
| --- | --- |
| Move to the pixel | Move the mouse, or press the arrow keys: one pixel at a time, ten with <kbd>Shift</kbd> |
| Hold the screen still | Press <kbd>Space</kbd> or the bar's **Freeze**. The bar shows **FROZEN** and the time, and each monitor gets corner marks. <kbd>Space</kbd> again goes back to the live screen. |
| Pick | Click, or press <kbd>Enter</kbd> |
| Cancel | <kbd>Esc</kbd>, or a right-click |

Relay comes back and says what it picked: *Picked 1248, 680 · #EC3013*, followed by *from the frozen screen* if it was frozen. The check keeps its tolerance and timeout.

**Freeze** is for what blinks, moves, or only shows while the mouse is over something. The picker never takes the focus from the app under it, so an open menu or tooltip stays open: open it, press **Pick from screen**, point at it, press <kbd>Space</kbd>, and pick at your ease.

There's no countdown: the picker waits until you pick or cancel. It's there only while Relay isn't recording or playing.

With [**Clicks land: Window**](04-playback.md#screen-or-window-coordinates), the check follows the window too: if the window moved, Relay watches the pixel at the same place in the window.

## When the pixel never matches

If the timeout runs out, Relay:

- stops playback and releases any held keys and buttons,
- shows *"Pixel check timed out at step 7; playback stopped."*,
- leaves the playhead **on that step**. Its row says **Timed out**, and the Step tab says what happened: *Timed out in the last run: the pixel at 1248, 680 px wasn't #EC3013 within 5 s, so the macro stopped.*

[Activity](08-library.md#activity) shows how long each check waited in the runs before, which helps pick a timeout. A run that timed out has **Open at step N**, which opens the macro on that step.

A macro that times out has usually hit one of these:

| Cause | Fix |
| --- | --- |
| The app was slower than the timeout | Raise **Gives up after** |
| The color is slightly different each time (anti-aliasing, shadows, transparency) | Raise **Tolerance** to 16 to 32, or pick a pixel in a flat area |
| The window is somewhere else | Use **Clicks land: Window**, or keep the window in the same place |
| Display scaling or theme changed since you picked the color | Pick again |
| HDR or protected video | These can change the colors Relay reads. Pick a pixel outside the video, or turn HDR off. |

## Choosing a good pixel

> [!TIP]
> Waiting for something whose position changes, or that isn't one flat color? A [Find image](06-find-image.md) step waits for a picture anywhere on screen, then clicks it.

- ✅ **The middle of a solid area** of something that only appears when you're ready: a dialog's title bar, a colored button, a status icon.
- ✅ **A distinctive color**, not white or light grey, which are everywhere.
- ❌ **Edges of text or icons**, where colors blend with the background.
- ❌ **Gradients, animations and blinking cursors.**
- ❌ **Anything under Relay's player or window** while the macro plays, since they cover it.

## Example: wait for an export to finish

An app shows a green check mark when an export is done, and the export takes anywhere from 2 to 20 seconds. You recorded it on a slow day: click **Export**, wait 20 seconds, click **Close**.

1. In the editor, select the **Export** click step. The check will go right after it.
2. Run an export by hand, so the green check is on screen.
3. Press **Add** → **Wait for pixel**, then **Pick from screen**, and click the middle of the green check.
4. Set **Gives up after** to 30 s.
5. Select the **Close** click step and set **Pause before** to 0.3 s. The 20 seconds you waited while recording are gone.

Now the macro clicks **Close** as soon as the export is done: after 2 seconds on a good day, after 20 on a bad one, and never too early.

---

<p align="center"><a href="04-playback.md">← Playing back</a> · <a href="README.md">Contents</a> · <a href="06-find-image.md">Find image →</a></p>
