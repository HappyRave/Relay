# Fix a problem

Find your symptom or Relay's message, with its fix under it.

- [Problems installing and starting](#problems-installing-and-starting)
- [Problems while recording](#problems-while-recording)
- [Problems while playing](#problems-while-playing)
- [Problems with waits and pictures](#problems-with-waits-and-pictures)
- [Problems with keys and triggers](#problems-with-keys-and-triggers)
- [Problems with standalone programs](#problems-with-standalone-programs)
- [Report a bug](#report-a-bug)

## Problems installing and starting

### Windows protected your PC

Relay isn't code-signed. Select **More info**, then **Run anyway**. Read [Is "Windows protected your PC" safe?](getting-started.md#is-windows-protected-your-pc-safe).

### Relay doesn't open: WebView2 is missing

The portable exe needs WebView2. Only the installer adds it. Read [Use the portable exe](files.md#use-the-portable-exe).

### I closed Relay and it disappeared

Relay is in the tray, by the clock: select its icon. With **Close to the tray** off, closing quits Relay. Read [Where Relay goes when you close it](getting-started.md#where-relay-goes-when-you-close-it).

## Problems while recording

### A click was not recorded

Relay doesn't record clicks on its window or the player. Move the player, then record again. Remote Desktop and administrator apps can also cause this. Read [What Relay records, and what it doesn't](record.md#what-relay-records-and-what-it-doesnt).

### Keystrokes are missing over Remote Desktop

Turn off **Ignore simulated input** in **Settings**, then record again. Read [Record over Remote Desktop](remote-desktop.md#record-over-remote-desktop).

### The last step is a move to the player

Your mouse moved to **Stop**: delete this step. Next time, stop with <kbd>Esc</kbd>. Read [Delete a step](edit-steps.md#delete-a-step).

### An accented letter shows without its accent

The accent shows as a separate step. Playback types the accented letter.

## Problems while playing

### Clicks land in the wrong place

Set **Clicks land** to **Window** only if the app's window moved. For new monitors or scaling, use Find image or record again. Read [Make clicks follow a moved window](play.md#make-clicks-follow-a-moved-window).

### The macro types into the wrong window

Keys go to the window in front. Select your app, then press the **Play / pause** key. Read [Relay's four keys](keys.md#relays-four-keys).

### The macro types the wrong characters

The PC uses another keyboard layout. Use a Text step instead. Read [Turn typed text into a Text step](text.md#turn-typed-text-into-a-text-step).

### The app in front runs as administrator…

Windows blocks Relay's input to that app. Quit Relay, find it in the Start menu, and press <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>Enter</kbd> to start it as administrator. Read [Playback: where input goes](../spec/playback.md#where-input-goes).

### Relay won't play a macro with its data file

The line under **Data file** says why. Fix the file, or select **Change…**. Read [Fill in a form from a spreadsheet: Good to know](spreadsheet.md#good-to-know).

### Couldn't save the change…

Relay keeps the change until you quit. A failed export wrote nothing. Free disk space, or close the program locking the file. Read [Where your macros are](files.md#where-your-macros-are).

## Problems with waits and pictures

### Pixel check timed out at step …

Increase **Gives up after**, or use another pixel. Read [When the wait gives up](wait.md#when-the-wait-gives-up).

### The pixel check passes too early

Use another pixel. Read [Choose a pixel that works](wait.md#choose-a-pixel-that-works).

### Image not found at step …

Select the step's **Test**. Increase **Gives up after**, or select **Snip** again. Read [When the picture isn't found](find-image.md#when-the-picture-isnt-found).

## Problems with keys and triggers

### Another program uses…

Set another key in **Settings**. Read [When a key does nothing](keys.md#when-a-key-does-nothing).

### … is taken by another app

Relay saves the hotkey, but it does nothing. Set another, or close the other program. Read [When a key does nothing](keys.md#when-a-key-does-nothing).

### The schedule didn't run

A schedule runs only if Relay runs, triggers aren't paused, and its switch is on. **Activity** says why a run was skipped. Read [If the PC is asleep or locked](run-automatically.md#if-the-pc-is-asleep-or-locked).

### Skipped “…”: Relay was busy

Relay was recording or playing. A trigger never waits. Read [When a trigger is skipped](run-automatically.md#when-a-trigger-is-skipped).

### Triggers are paused

Select **Resume** in the Library, or **Triggers active** in the tray menu. Read [Stop everything, then turn triggers back on](stop-a-macro.md#stop-everything-then-turn-triggers-back-on).

## Problems with standalone programs

### An antivirus removed the program

In the antivirus, restore the program and allow it. Read [SmartScreen, antivirus and signing](scripts.md#smartscreen-antivirus-and-signing).

### The program ended with exit code 5

A pixel check or Find image step timed out. Without `--quiet`, its window stays open and names the step. Increase the step's **Gives up after**, then export again. Read [Exit codes](scripts.md#exit-codes).

### The program ended with exit code 6

The screen was locked, so nothing played. Read [Exit codes](scripts.md#exit-codes).

### Ctrl+Alt+End doesn't stop the program

Over Remote Desktop, neither this key nor <kbd>Esc</kbd> reaches the program. Select its **Stop**, or end it in Task Manager. Read [Stop a standalone program](stop-a-macro.md#stop-a-standalone-program).

## Report a bug

Send a log with your report. Read [Send a log with a bug report](files.md#send-a-log-with-a-bug-report).

---

Previous: [Where Relay keeps things](files.md) · Next: [Can Relay…?](faq.md) · [Guide home](README.md)
