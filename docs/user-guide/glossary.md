# Words used in Relay

What each word means and where you see it.

## Words for the window

### Steps list

The editor's list of a macro's steps, in play order.

### Inspector

The part of the editor with 2 tabs. **Step** shows the selected steps. **Macro** shows the whole macro.

### View

The part of the editor with the preview and the timeline.

### Timeline

The macro's steps on a time scale, in the view.

### Preview

A picture of your screen with the macro's mouse path, in the view.

### Playhead

The point where play starts and new steps go. It is the selected step, and a line on the timeline.

### Player

Relay's small bar on top of your apps. It shows in player mode, while Relay records or plays with its window hidden, and while a trigger runs a macro. Read [Use the player](player.md).

### Player mode

Only the player shows, not Relay's window. To turn it on, press <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>M</kbd> (**Show the player**), or select **Switch to the player** in the top bar.

### Tray

The icons by the clock, maybe under the **^** arrow.

## Words for macros

### Step

One instruction, such as a click: a row in the steps list.

### Move

A step that moves the mouse to the next step, or after the last one. Unless **Show moves** is on, the steps list folds it into the next step's row. A move after the last step always has its own row.

### Pause

Recorded time with no input before a step, at **Pause before** on the **Step** tab. It is not a [wait](#wait).

### Wait

A step you add with **Add**, then **Wait**. Relay never records one.

### Trigger

Something that runs a macro automatically, such as a hotkey. Set it on the **Macro** tab.

### Data file

A file of rows, set at **Data file** on the **Macro** tab. The macro plays once per row.

### Standalone program

A program that plays a macro without Relay. Make one with **More**, then **Export…**, then **Standalone program**.

### Stop everything

A key that stops any recording or playback and pauses triggers. It is <kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>Backspace</kbd> by default. After an upgrade from an older Relay, and in every standalone program, it is <kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>End</kbd>. Change it in **Settings**, at **Keys**.

---

Previous: [Can Relay…?](faq.md) · [Guide home](README.md)
