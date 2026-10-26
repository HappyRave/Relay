# The view: preview and timeline

What the editor's view shows under the steps and the inspector: its bar, the preview, the timeline and the measurements table.

Rules are numbered VIEW-n. Cite them in tests and PRs.

- [The view](#the-view)
- [The view bar](#the-view-bar)
- [Folding](#folding)
- [The preview](#the-preview)
- [Screen and Outline](#screen-and-outline)
- [What the preview draws](#what-the-preview-draws)
- [Zooming and moving the preview](#zooming-and-moving-the-preview)
- [The timeline](#the-timeline)
- [Seeking](#seeking)
- [Timeline keys](#timeline-keys)
- [The measurements table](#the-measurements-table)

## The view

- **VIEW-1** The view sits in the editor under the steps list and the inspector. It shows where and when the macro acts.
- **VIEW-2** Open, the view has the [preview](#the-preview), the [timeline](#the-timeline) with the playhead, and the [measurements table](#the-measurements-table).

## The view bar

- **VIEW-3** The view's bar has, from left to right: **View**, the time, **Screen | Outline**, **Previous step** and **Next step**, and, while the preview is zoomed in, the zoom and **Fit**.
- **VIEW-4** **View** folds the view away, or opens it again.
- **VIEW-5** The time shows the playhead's time and the macro's length: *00:06.02 / 10.15*.
- **VIEW-6** **Screen | Outline** chooses what the preview draws the macro over (see [Screen and Outline](#screen-and-outline)).
- **VIEW-7** **Previous step** and **Next step** move the playhead to the previous or next step.
- **VIEW-8** While the preview is zoomed in, the bar shows the zoom and **Fit**: *250% · Fit*. **Fit** shows the whole screen again.

## Folding

- **VIEW-9** Folded, the bar says what's under the playhead: *Step 4 · Double click · Filename field*.
- **VIEW-10** A macro without mouse steps (only keys, text and waits) opens with the view folded, since there's nothing to draw.
- **VIEW-11** Every macro opens with the view folded when the window is short.

## The preview

- **VIEW-12** The preview draws the macro over the whole screen: all the monitors, whatever the size of the pane.
- **VIEW-13** While a recording runs with the window showing (**Settings → Window → Hide the window while recording** off, see [settings](settings.md)), an open editor's preview draws the mouse path as it's recorded.

## Screen and Outline

- **VIEW-14** **Screen** draws the macro over the [screenshot](recording.md) taken when it was recorded, dimmed so the path stays readable.
- **VIEW-15** **Outline** draws the macro over the outlines of the monitors and of the window first clicked in during the recording.
- **VIEW-16** The choice between **Screen** and **Outline** is remembered.
- **VIEW-17** A macro without a screenshot (recorded before Relay 1.3, imported from a file, or recorded with **Settings → Recording → Screenshot** off) always shows the outline, and the **Screen | Outline** choice is off.

## What the preview draws

- **VIEW-18** The **mouse path**: a dashed line for the whole recording, and a solid line for the part already played.
- **VIEW-19** With **Settings → Appearance → Mouse path: Played part**, only the solid part (the part already played) is drawn. See [settings](settings.md).
- **VIEW-20** The path of the selected step's [move](editing.md) is drawn thicker. Selecting a move's own row, or the step it's folded into, both count.
- **VIEW-21** A **numbered circle** beside each click, numbered like its row (see [step numbering](steps.md)).
- **VIEW-22** A click's circle is filled once that click has played, and white when the click is selected.
- **VIEW-23** A selected click shows its label next to its number when **Settings → Appearance → Show click labels** is on.
- **VIEW-24** The **cursor**, at its position for the playhead's current time.
- **VIEW-25** For a selected [pixel check](pixel-checks.md), a small square at its pixel, in the color it waits for.

## Zooming and moving the preview

| Rule | Action | Does |
| --- | --- | --- |
| **VIEW-26** | **Scroll** over the preview, or <kbd>+</kbd> and <kbd>−</kbd> | Zoom in on a part, zoom out |
| **VIEW-27** | **Drag**, or the arrow keys | Move around, when zoomed in |
| **VIEW-28** | **Double-click**, or <kbd>0</kbd> | Show the whole screen again |

## The timeline

- **VIEW-29** The timeline shows the whole macro, with its length above it, in three lanes:

| Rule | Lane | Shows |
| --- | --- | --- |
| **VIEW-30** | **Mouse** (a circle) | A bar for each [move](editing.md) and drag; a tick for each click, scroll and drag, hollow until played; and [pauses](editing.md), striped |
| **VIEW-31** | **Keyboard** (a square) | Key combinations, typed text and [Text steps](text-steps.md), with their text |
| **VIEW-32** | **Logic** (a triangle), the bottom lane | Waits, pixel checks (as *Wait for* blocks) and Find image steps (as *Find* blocks) |

- **VIEW-33** A *Wait for* or *Find* block's length is only there to make it visible and clickable: when the pixel already matches, or the image is already there, the macro doesn't spend that time waiting (see [pixel checks](pixel-checks.md) and [Find image](find-image.md)).
- **VIEW-34** While recording, the Mouse lane follows the live cursor path.

## Seeking

- **VIEW-35** Clicking or dragging anywhere on the timeline moves the playhead there.
- **VIEW-36** When the playhead moves, the preview, the steps list and the time all follow.
- **VIEW-37** Playback starts from the playhead, and seeking also works while a macro plays: see [Playback](playback.md).

## Timeline keys

With the timeline focused:

| Rule | Key | Does |
| --- | --- | --- |
| **VIEW-38** | <kbd>←</kbd> or <kbd>↓</kbd> | The previous step |
| **VIEW-39** | <kbd>→</kbd> or <kbd>↑</kbd> | The next step |
| **VIEW-40** | <kbd>Shift</kbd>+arrow | A tenth of a second back or forward |
| **VIEW-41** | <kbd>Home</kbd>, <kbd>End</kbd> | The start, the end |
| **VIEW-42** | <kbd>Space</kbd> | Play or pause |

## The measurements table

- **VIEW-43** The open view has a small table of the macro's measurements:

| Rule | Row | Shows |
| --- | --- | --- |
| **VIEW-44** | **Steps** | The number of steps |
| **VIEW-45** | **Length** | The macro's length |
| **VIEW-46** | **Screen** | The screen it was recorded on: all the monitors, in pixels |
| **VIEW-47** | **Scale** | The preview's scale |

---

[Spec index](README.md)
