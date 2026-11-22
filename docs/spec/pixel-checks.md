# Pixel checks

This page covers *Wait for* steps (pixel checks): how they match, their tolerance and timeout, adding one, the screen picker, and what makes a good pixel.

Rules are numbered PIX-n. Cite them in tests and PRs.

- [What a pixel check is](#what-a-pixel-check-is)
- [How matching works](#how-matching-works)
- [When the timeout runs out](#when-the-timeout-runs-out)
- [Adding a pixel check](#adding-a-pixel-check)
- [Its fields](#its-fields)
- [Pick from screen](#pick-from-screen)
- [Choosing a good pixel](#choosing-a-good-pixel)

## What a pixel check is

- **PIX-1** A pixel check makes a macro wait until a pixel on screen has a given color, then continue. In the steps list it's a **Wait for** step showing its pixel, its label and how long it waits at most: **Wait for** · pixel 1248, 680 · Save dialog is open *≤ 5 s*.
- **PIX-2** On the timeline it's a *Wait for* block in the bottom lane, with the other logic steps (see [the timeline](view.md#the-timeline)). The block's length is only there to make it visible and clickable: when the pixel already matches, the macro doesn't spend that time waiting.

## How matching works

- **PIX-3** When playback reaches the check, the playhead stops and Relay reads the pixel at **X, Y** about 30 times a second.
- **PIX-4** As soon as the pixel's color is close enough to the target color, playback continues right away with the next step.
- **PIX-5** "Close enough" means each of red, green and blue is within **Tolerance** of the target's. With a tolerance of 8, `#EC3013` matches anything from `#E42805` to `#F4381B`.
- **PIX-6** Pausing during a check (**Pause**, or the [Play / pause key](keys.md)) freezes the timeout too. It carries on counting on resume.
- **PIX-7** With [**Clicks land: Window**](playback.md#screen-or-window-coordinates), the check follows the window: if the window moved, Relay watches the pixel at the same place in the window.
- **PIX-8** HDR and protected video can change the colors Relay reads.
- **PIX-9** A change of display scaling or theme since the color was picked can change the pixel's color.

## When the timeout runs out

- **PIX-10** If the pixel doesn't match within **Gives up after**, Relay stops playback and releases any held keys and buttons.
- **PIX-11** It shows *"Pixel check timed out at step 7; playback stopped."* (with the check's [step number](steps.md)).
- **PIX-12** The playhead stays on that step, and its row is marked **Timed out** (see [the steps list](editing.md#the-steps-list); the playhead after a playback ends is in [Playback](playback.md)).
- **PIX-13** The check's Step tab says what happened: *Timed out in the last run: the pixel at 1248, 680 px wasn't #EC3013 within 5 s, so the macro stopped.*
- **PIX-14** Activity shows how long each check waited in earlier runs, and a run that timed out has **Open at step N**, which opens the macro on that step (see [Activity](library.md#activity-and-problems)).

## Adding a pixel check

- **PIX-15** **Add** (or <kbd>Insert</kbd>) → **Wait for pixel** in the steps list adds a pixel check at the insertion line (*+ insert here*), right after the step under the playhead (see [adding steps](editing.md#adding-steps)). The **+** that shows between two rows opens the same menu, to add it there.
- **PIX-16** The new check lasts 800 ms. It points at where the macro's cursor is at that moment of the macro, and takes the color that pixel has on the screen right now.
- **PIX-17** A new check uses a **Tolerance** of 8 and **Gives up after** 5 s.
- **PIX-18** If Relay can't read the screen at that point (the screen is locked, say), it inserts nothing and says *Couldn't read the screen at x, y*.

## Its fields

The inspector's Step tab on a *Wait for* step (saving fields and number fields: see [Editing](editing.md#the-step-tab)):

| Rule | Field | Meaning |
| --- | --- | --- |
| **PIX-19** | **Label** | An optional note, such as *Save dialog is open*. The steps list shows it. |
| **PIX-20** | **Pause before** | The time between the step before and this one |
| **PIX-21** | **Position** | **X** and **Y**: the pixel to watch, in screen pixels |
| **PIX-22** | **Pick from screen** | Points the check at a pixel on the screen (see [Pick from screen](#pick-from-screen)) |
| **PIX-23** | **Color** | The color to wait for, as `#RRGGBB`. A swatch shows it. |
| **PIX-24** | **Tolerance** | How far each color channel may be off, 0 to 255. 0 means an exact match. |
| **PIX-25** | **Gives up after** | How long to wait before stopping: at least 0.5 s, in steps of 0.5 s |

## Pick from screen

- **PIX-26** **Pick from screen** sets aside Relay's windows and shows the picker over the live screen. The same picker serves the [pixel trigger](triggers.md#when-a-pixel-changes).
- **PIX-27** A loupe follows the cursor: the 11 × 11 pixels around it, magnified, with the coordinates and the color under them (*1248, 680 · #EC3013*).
- **PIX-28** A bar at the top of the screen says what to do, and shows **LIVE**.
- **PIX-29** There's no countdown: the picker is ready at once and waits until a pick or a cancel.
- **PIX-30** The picker is only available while Relay isn't recording or playing.
- **PIX-31** The picker never takes the focus from the app under it, so an open menu or tooltip stays open. Keys the picker doesn't use reach that app.

| Rule | To | Do |
| --- | --- | --- |
| **PIX-32** | Move to the pixel | Move the mouse, or press the arrow keys: one pixel at a time, ten with <kbd>Shift</kbd> |
| **PIX-33** | Hold the screen still | <kbd>Space</kbd> or the bar's **Freeze**. The bar shows **FROZEN** and the time, and each monitor gets corner marks. <kbd>Space</kbd> again goes back to the live screen (**LIVE**). |
| **PIX-34** | Pick the pixel under the cursor | Click, or <kbd>Enter</kbd> |
| **PIX-35** | Cancel | <kbd>Esc</kbd>, or a right-click |

- **PIX-36** Freeze holds what blinks, moves, or only shows while the mouse is over something.
- **PIX-37** After a pick, Relay comes back and says what it picked: *Picked 1248, 680 · #EC3013*, followed by *from the frozen screen* if the screen was frozen.
- **PIX-38** A pick sets the check's position and color; it keeps its **Tolerance** and **Gives up after**.

## Choosing a good pixel

- **PIX-39** Relay reads the screen as it shows: a pixel under Relay's player or window while the macro plays reads their color, since they cover it.
- **PIX-40** The colors at the edges of text and icons blend with the background, and anti-aliasing, shadows and transparency can make a color slightly different each time. A tolerance of 16 to 32, or a pixel in a flat area, absorbs this.
- **PIX-41** Gradients, animations and blinking cursors change color on their own; white and light grey are common everywhere on screen. The middle of a solid, distinctive-colored area that only appears when the app is ready (a dialog's title bar, a colored button, a status icon) is a reliable target.
- **PIX-42** A pixel check watches one fixed position. For something whose position changes, or that isn't one flat color, a [Find image](find-image.md) step waits for a picture anywhere on screen, then clicks it.

---

[Spec index](README.md)
