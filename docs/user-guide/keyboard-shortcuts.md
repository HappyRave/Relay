# Keyboard shortcuts

- [Relay's keys](#relays-keys)
  - [Setting them](#setting-them)
- [In the window](#in-the-window)
  - [Anywhere](#anywhere)
  - [The editor](#the-editor)
  - [The steps list](#the-steps-list)
  - [The timeline and the preview](#the-timeline-and-the-preview)
  - [The Library](#the-library)
  - [Fields, menus and dialogs](#fields-menus-and-dialogs)
- [The picker](#the-picker)
- [Exported programs](#exported-programs)

## Relay's keys

These work from any app, even when Relay's windows are hidden. Each one is a setting (**Settings → Keys**), so yours may differ:

| Key | New install | Upgraded from an older Relay |
| --- | --- | --- |
| **Record** | <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>F9</kbd> | <kbd>F9</kbd> |
| **Play / pause** | <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>F10</kbd> | <kbd>F10</kbd> |
| **Show the player** | <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>M</kbd> | <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>M</kbd> |
| **Stop everything** (the kill switch) | <kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>Backspace</kbd> | <kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>End</kbd> |

What each does depends on what Relay is doing:

| Key | Idle | While recording | While playing |
| --- | --- | --- | --- |
| **Record** | Start recording | Stop recording (during the countdown: cancel it) | *Stops playback¹* |
| **Play / pause** | Play the open macro from the playhead | *(recorded like any key)* | Pause / resume |
| **Show the player** | [Player mode](04-playback.md#player-mode) on or off | *(recorded like any key)* | *Stops playback¹* |
| **Stop everything** | Pause triggers | Stop, keep the recording, pause triggers | Stop, pause triggers |
| <kbd>Esc</kbd> | *(reaches your app)* | Stop recording² | Stop playback |
| Your macro hotkeys | Run that macro | *(recorded like any key)* | *Stops playback¹* |

¹ With **Stop on key** on (the default), **any** key stops playback and is swallowed, except your play key, the kill switch, and <kbd>Shift</kbd>, <kbd>Ctrl</kbd>, <kbd>Alt</kbd> or <kbd>Win</kbd> on their own. With it off, these keys reach your app.

² Unless **Settings → Recording → Esc stops recording** is off: then <kbd>Esc</kbd> is recorded like any key.

Relay only holds on to a key while it needs it. For example, your play key belongs to your apps while you record, and <kbd>Esc</kbd> belongs to your apps when Relay is idle.

### Setting them

In **Settings → Keys**, press **Change** next to a key, then the combination. <kbd>Backspace</kbd> sets it to *None* (the buttons still work, and the key goes to your apps), and <kbd>Esc</kbd> cancels. **Stop everything** can't be *None*. While Relay listens for a combination, its own keys and your macros' hotkeys let go of the keyboard, so even one they use reaches the field.

The same rules apply as to a macro's [hotkey](07-triggers.md#hotkey): a key that types needs <kbd>Ctrl</kbd>, <kbd>Alt</kbd> or <kbd>Win</kbd> (<kbd>Shift</kbd> alone isn't enough), function keys work alone, and a combination can't end with a modifier. Two of Relay's keys can't be the same, and none can be a macro's hotkey: Relay says why under the key, such as *Ctrl + Shift + F9 is already Relay's “Record” key* or *Ctrl + Alt + 1 already runs “Daily report”*.

Relay also warns when a key you set costs you something:

| When | It says, for example |
| --- | --- |
| Apps use the key | *F9 recalculates in Excel and updates fields in Word. While Relay runs, they won't get it.* |
| A <kbd>Ctrl</kbd>+<kbd>Alt</kbd> combination types a character on your keyboard (<kbd>AltGr</kbd>) | *Ctrl + Alt + E types “€” on your keyboard (AltGr + E). You couldn't type it while this is set.* |
| A remote-desktop or virtual-machine app takes it | *Remote Desktop sends this as Ctrl + Alt + Del. In a remote session, Relay won't get it.* |

The tooltips, the player and the tray menu name your keys as set.

## In the window

### Anywhere

| Key | Does |
| --- | --- |
| <kbd>Ctrl</kbd>+<kbd>Z</kbd> | In a macro: undo its last edit (in a text or number field: undo your typing) |
| <kbd>Ctrl</kbd>+<kbd>Y</kbd> or <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>Z</kbd> | In a macro: redo |
| <kbd>Alt</kbd>+<kbd>←</kbd> | Back to the Library, from Settings, or from a macro unless it records or plays |
| <kbd>Tab</kbd> / <kbd>Shift</kbd>+<kbd>Tab</kbd> | Move between controls |

### The editor

| Key | Does |
| --- | --- |
| <kbd>Ctrl</kbd>+<kbd>M</kbd> | The inspector's **Macro** tab |
| <kbd>Ctrl</kbd>+<kbd>F</kbd> | **Find a step** |
| <kbd>←</kbd> <kbd>→</kbd> on the inspector's tabs | Switch between **Step** and **Macro** |
| <kbd>Esc</kbd> | In a narrow window, close the inspector |

### The steps list

| Key | Does |
| --- | --- |
| <kbd>↓</kbd> or <kbd>→</kbd>, <kbd>↑</kbd> or <kbd>←</kbd> | Select the next or previous step (the playhead follows) |
| <kbd>Home</kbd>, <kbd>End</kbd> | Select the first or last step |
| <kbd>Shift</kbd> with the above, or <kbd>Shift</kbd>+click | Select a range |
| <kbd>Ctrl</kbd>+click | Add a step to the selection, or take it out |
| <kbd>Ctrl</kbd>+<kbd>A</kbd> | Select every step shown |
| <kbd>Esc</kbd> | Back to one selected step |
| <kbd>Enter</kbd> | Go to the selected step's first field in the inspector |
| <kbd>Del</kbd> | Delete the selected steps (with Undo) |
| <kbd>Insert</kbd> or <kbd>+</kbd> | Open **Add** at the insertion line |

### The timeline and the preview

| Where | Key | Does |
| --- | --- | --- |
| Timeline | <kbd>←</kbd> or <kbd>↓</kbd>, <kbd>→</kbd> or <kbd>↑</kbd> | Previous or next step |
| Timeline | <kbd>Shift</kbd>+arrow | A tenth of a second back or forward |
| Timeline | <kbd>Home</kbd>, <kbd>End</kbd> | The start or the end |
| Timeline | <kbd>Space</kbd> | Play or pause |
| Preview | <kbd>+</kbd>, <kbd>−</kbd> | Zoom in, out |
| Preview | Arrows | Move around, when zoomed in |
| Preview | <kbd>0</kbd> | Show the whole screen |

### The Library

| Key | Does |
| --- | --- |
| <kbd>Ctrl</kbd>+<kbd>F</kbd> | **Search macros** |
| <kbd>↓</kbd>, <kbd>↑</kbd> in a row | The next or previous macro |
| <kbd>Enter</kbd> on a name | Open the macro |
| <kbd>Del</kbd> in a row | Move the macro to the trash (with Undo); if its row is selected, all the selected ones |
| <kbd>Ctrl</kbd>+<kbd>D</kbd> in a row | Duplicate the macro |

### Fields, menus and dialogs

| Where | Key | Does |
| --- | --- | --- |
| Number field | <kbd>↑</kbd>, <kbd>↓</kbd> | One step up or down; with <kbd>Shift</kbd>, ten |
| Any field | <kbd>Enter</kbd> or leaving it | Saves the change |
| Hotkey field | Any combination | Sets the hotkey |
| Hotkey field | <kbd>Backspace</kbd> | Clears it |
| Hotkey field | <kbd>Esc</kbd> | Cancels |
| Menu | <kbd>↑</kbd> <kbd>↓</kbd>, <kbd>Home</kbd> <kbd>End</kbd> | Move between items |
| Menu | <kbd>Esc</kbd> or <kbd>Tab</kbd> | Close it |
| Choices (such as Speed) | Arrows | Move the choice |
| Divider between the steps and the inspector | <kbd>←</kbd> <kbd>→</kbd> (with <kbd>Shift</kbd>, further), <kbd>Home</kbd> <kbd>End</kbd> | Move it; double-click puts it back |
| Export dialog | Arrows | Choose the format |
| Export dialog | <kbd>Esc</kbd> | Close it |

## The picker

**Pick from screen** (a pixel check, the [pixel trigger](07-triggers.md#when-a-pixel-changes)) puts Relay's windows aside and shows a loupe over the live screen. No countdown: it's ready at once.

| Key | Does |
| --- | --- |
| Arrows | Move the cursor one pixel; with <kbd>Shift</kbd>, ten |
| <kbd>Space</kbd> (or **Freeze** in the picker's help bar) | Hold the screen as it is (*FROZEN*), for what blinks or only shows on hover; again to go back to the live screen (*LIVE*) |
| Click or <kbd>Enter</kbd> | Pick the pixel under the cursor |
| <kbd>Esc</kbd> or right-click | Cancel |

Other keys reach the app under the picker, which never takes the focus, so an open menu stays open. See [Pick a pixel on screen](05-pixel-checks.md#pick-from-screen).

## Exported programs

A macro [exported as a program](08-library.md#exported-programs) doesn't use Relay's keys. It stops on <kbd>Esc</kbd> or <kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>End</kbd>, and on any other key when its macro's **Stop on key** is on.

---

<p align="center"><a href="10-troubleshooting.md">← Troubleshooting and FAQ</a> · <a href="README.md">Contents</a> · <a href="../engineering/README.md">Engineering guide →</a></p>
