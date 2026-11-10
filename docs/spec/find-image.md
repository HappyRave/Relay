# Find image

How Find image steps search the screen for a picture, click it, and give up. Rules are numbered IMG-n. Cite them in tests and PRs.

- [The search](#the-search)
- [Adding one](#adding-one)
- [The image](#the-image)
- [Fields](#fields)
- [Test and Show](#test-and-show)
- [Another size](#another-size)
- [When the image isn't found](#when-the-image-isnt-found)
- [Files and versions](#files-and-versions)

## The search

- **IMG-1** When playback reaches a Find image step, the playhead stops and Relay looks for the image on the screen about 4 times a second.
- **IMG-2** As soon as Relay finds the image, it clicks it at the step's click point, and playback continues right away.
- **IMG-3** If the image isn't found within the step's **Gives up after** time, playback stops (see [When the image isn't found](#when-the-image-isnt-found)).
- **IMG-4** Found means at least as similar as **Match** (85% by default).
- **IMG-5** Relay compares shades, not exact colors, so the image is still found when it's a little lighter or darker (a hovered button, say).
- **IMG-6** Relay looks for the image anywhere on the screen. With several monitors, the step's **Looks on** limits the search to one screen; searching one screen is faster than **All screens**.
- **IMG-7** When several places match, Relay clicks the best match, which may not be the one meant.
- **IMG-8** Relay never looks inside its own windows, so the copy of the image in the inspector is never the one found.
- **IMG-9** Relay doesn't see what's behind its player or its window (a click couldn't reach it). A window in front of Relay's, like a dialog opening over it, is seen. See [**Hide the window while playing**](settings.md).
- **IMG-10** The step's length on the timeline is only there to make it visible and clickable: when the image is already there, the macro doesn't spend that time waiting.
- **IMG-11** A smaller picture is found faster.
- **IMG-12** Searching takes processor time 4 times a second, and only while a Find image step is waiting.
- **IMG-13** The image trigger (**When an image appears**) uses the same kind of search; its rules are in [Triggers](triggers.md#when-an-image-appears).

## Adding one

- **IMG-14** **Add** → **Find image** opens Windows' snipping overlay; the rectangle dragged becomes the image. The step goes where [inserts go](editing.md).
- **IMG-15** While the snip waits, the steps list says *Snip the image to find, or* with **Paste**, for the picture on the clipboard, and **File…**, for a PNG or JPEG file.
- **IMG-16** <kbd>Esc</kbd> closes the snipping overlay; **Cancel** then abandons adding the step.
- **IMG-17** The new step lasts 800 ms, left-clicks the middle of the image, needs an 85% match and gives up after 5 s.

## The image

- **IMG-18** Relay keeps pictures up to 512 pixels on their longest side; larger ones are shrunk.
- **IMG-19** Relay refuses a picture smaller than 8 pixels, or one so plain (one flat color) that it could be anywhere.
- **IMG-20** The click point is set by clicking the image in the Step tab; a red ring shows it. It defaults to the image's middle.
- **IMG-21** **Snip**, **Paste** and **File…** in the Step tab replace the image, and the click point goes back to its middle.

## Fields

The Step tab of a Find image step has these fields besides **Label** and **Pause before** ([Editing](editing.md)):

| Rule | Field | Meaning |
| --- | --- | --- |
| **IMG-22** | **The image** | What to look for; click it to set the click point ([IMG-20](#the-image)). |
| **IMG-23** | **Snip**, **Paste**, **File…** | Replace the image ([IMG-21](#the-image)). |
| **IMG-24** | **Match** | How similar the screen must be, 50 to 100%. Lower finds more, and risks the wrong thing. Default 85%. |
| **IMG-25** | **Gives up after** | How long to look before stopping: at least 0.5 s, in steps of 0.5 s. Default 5 s. |
| **IMG-26** | **Test**, **Show** | See [Test and Show](#test-and-show). |
| **IMG-27** | **Clicks** | **Left**, **Right** or **Middle**. Default **Left**. |
| **IMG-28** | **Looks on** | With several monitors: **All screens**, or one (**Screen 1**, **Screen 2**…). |

## Test and Show

- **IMG-29** **Test** looks for the image once, now, and says where it found it, as *Found at 812, 440 (97%)*, or how close it came, as *Not found: the best match is 61%*.
- **IMG-30** After a Test, **Show** marks that match on the screen itself for 3 seconds: a red outline around it and a red dot where the step would click.
- **IMG-31** **Show** also works after a poor match, marking what Relay took for the image.

## Another size

- **IMG-32** Relay finds the picture from half to twice its size on screen. That covers a screenshot taken on a monitor with other display scaling (100% and 150%, say), a zoomed-in screenshot, or a page zoomed in the browser.
- **IMG-33** The click point grows or shrinks with the size found.
- **IMG-34** The image still has to look the same, with the same shape: it isn't found rotated, stretched one way only, or restyled (a new theme, dark mode).

## When the image isn't found

- **IMG-35** When **Gives up after** runs out, Relay stops playback and releases any held keys and buttons.
- **IMG-36** It shows *"Image not found at step 5; playback stopped."* (with the step's number).
- **IMG-37** The playhead stays on that step, and its row in the steps list says **Timed out**. How the playhead and player behave after it: see [Playback](playback.md).
- **IMG-38** An image isn't found when the app was slower than the timeout, when it looks different now (hovered, selected, another theme), when it's covered by another window or by Relay's player or window, when it's much bigger or smaller than the picture, or when Match is stricter than what **Test** reports.
- **IMG-39** Activity records each search and a run that stopped on one: see [Library and Activity](library.md).

## Files and versions

- **IMG-40** A macro with a Find image step needs Relay 1.4 or later to open; older versions say it was saved by a newer Relay ([Import](export-import.md)).
- **IMG-41** The image is part of the macro: exports carry it (what each format does with it: [Export and import](export-import.md)).

---

[Spec index](README.md)
