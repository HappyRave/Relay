# Show targets

This page covers *Show targets*: the marks Relay draws on the screen where a macro clicks, drags, scrolls and types, live as it plays, or every target at once from the editor.

Rules are numbered TGT-n. Cite them in tests and PRs.

- [Live marks](#live-marks)
- [Every target at once](#every-target-at-once)
- [What marks never do](#what-marks-never-do)

## Live marks

- **TGT-1** With **Show targets** on (**Settings → Window**, or the target key on the [player](windows.md#the-player)), each step marks the screen as it starts playing. It's off by default.
- **TGT-2** Turning it on or off while a macro plays applies from the next step.

| Rule | Step | Mark |
| --- | --- | --- |
| **TGT-3** | Click | A red dot on the exact pixel, and a ring rippling out from it. A double click ripples twice; a right click ripples as a rounded square. It lasts about a second. |
| **TGT-4** | Drag | A red line drawn along the path as the drag plays, from a dot where it starts to an arrow where it ends. It stays a moment after the drag. |
| **TGT-5** | Mouse move | A faint dotted red trail along the path. |
| **TGT-6** | Scroll | A red dot with three chevrons pointing the way it scrolls. |
| **TGT-7** | Keys | Keycaps in a dark caption by the cursor: <kbd>Ctrl</kbd> <kbd>S</kbd>. |
| **TGT-8** | Typing and Text steps | The text in quotes in a dark caption by the cursor, appearing as it's typed, cut to 22 characters with an ellipsis. A [Text step](text-steps.md) shows its text filled in. |

- **TGT-9** Waits, pixel checks and Find image steps have no mark.
- **TGT-10** A caption goes below and to the right of the cursor, flipped to the left or above near the edge of its monitor. A new caption replaces the one before.
- **TGT-11** A drag's line draws at the speed the macro plays.
- **TGT-12** With [**Clicks land: Window**](playback.md#screen-or-window-coordinates), the marks move with the clicks.
- **TGT-13** Marks are sized for each monitor's display scaling.

## Every target at once

- **TGT-14** **Show targets** in the editor's top bar shows every target of the open macro on the screen at once. Relay's windows step aside while it shows.
- **TGT-15** Each target is a red dot, with its step number in a red disc above and to its right (to its left near the right edge of the monitor, below it near the top). A right click's number is in ink.
- **TGT-16** Steps that land on the same spot share one dot, their numbers side by side.
- **TGT-17** A dotted ink route joins the targets in step order. A drag is a solid red line with an arrow.
- **TGT-18** Keys and typed text are tags under the spot where the cursor was: the step number, then the keys or the quoted text.
- **TGT-19** A faint wash covers the screen, and a bar at the bottom of the monitor with the cursor says the macro's name, its number of steps and *Esc or click to go back*.
- **TGT-20** <kbd>Esc</kbd> or a click anywhere goes back: the click doesn't reach the app under it, and Relay's windows come back.
- **TGT-21** With **Clicks land: Window**, the targets show where the clicks would land now. If Relay can't find the window, it says so and shows the recorded screen positions.
- **TGT-22** It's refused while Relay records or plays (the button is off). A macro with nothing to show says *This macro doesn't click, scroll or type anywhere.*

## What marks never do

- **TGT-23** Marks let clicks through and never take the focus, so the macro plays exactly as it would without them.
- **TGT-24** Marks are left out of what Relay reads from the screen: pixel checks, Find image steps and the pixel and image triggers never see them. Screenshots and screen recordings don't show them either.
- **TGT-25** [Exported programs](export-import.md) don't show marks.

---

[Spec index](README.md)
