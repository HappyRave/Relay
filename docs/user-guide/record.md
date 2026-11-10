# Record a macro

Record a task so that it plays back right the first time.

Find it: **Record** in the Library, or the record key (<kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>F9</kbd> on a new install).

> [!WARNING]
> Relay doesn't record clicks on its own windows, such as the player (Relay's small bar). Move the player away from where you click first. Relay remembers where you put it. Read [Move the player](player.md#move-the-player).

- [Start and stop a recording](#start-and-stop-a-recording)
- [What Relay records, and what it doesn't](#what-relay-records-and-what-it-doesnt)
- [Record the Esc key](#record-the-esc-key)
- [Good to know](#good-to-know)
- [Not possible yet](#not-possible-yet)

## Start and stop a recording

To record, press the record key, do the task, then press <kbd>Esc</kbd>.

1. Open the app to record.
2. Select **Record**, or press the record key. Relay's window hides and the player shows.
3. The player counts 3, 2, 1. When it shows *REC*, do the task.
4. Press <kbd>Esc</kbd> or the record key, or select **Stop** (a square) on the player.
5. Type a name. It replaces the default name, such as *Recording 1*.

If it worked, Relay says *Recording stopped, … steps. Name this macro.*

With **Sounds** on, a click sound marks the start and stop. Screen readers say *Recording*.

More detail: [Recording: starting and stopping](../spec/recording.md#starting-and-stopping)

## What Relay records, and what it doesn't

Relay records clicks, keys, scrolls, mouse [moves](glossary.md#move) and the time between them.

Relay doesn't record:

- the record key and the **Stop everything** key
- <kbd>Esc</kbd>, which stops the recording
- an app that runs as administrator, unless Relay does too
- simulated input while **Ignore simulated input** is on. Read [Record over Remote Desktop](remote-desktop.md#record-over-remote-desktop).

More detail: [Recording: what is left out](../spec/recording.md#what-is-left-out)

## Record the Esc key

To record <kbd>Esc</kbd>, stop it from ending the recording.

1. In the Library, select **Settings**.
2. Under **Recording**, turn off **Esc stops recording**. <kbd>Esc</kbd> still stops playback.
3. Record the task. Press <kbd>Esc</kbd> where the task needs it.
4. Stop with the record key, or **Stop** on the player.

If it worked, the new macro has a step for <kbd>Esc</kbd>.

## Good to know

- Relay keeps what you type as plain text. Don't record a password.
- Relay doesn't save a recording with no click, key or scroll.
- With **Screenshot** on, Relay keeps a picture of the screen at the start, on this PC only.
- Relay also records your screen reader's keys. Read [Remove screen-reader keys from a recording](accessibility.md#remove-screen-reader-keys-from-a-recording).

More detail: [Recording: privacy](../spec/recording.md#privacy)

## Not possible yet

- Recording more onto an existing macro. Record a new macro, or add steps with **Add**.

Something not working? Read [Problems while recording](troubleshooting.md#problems-while-recording).

---

Previous: [Get started](getting-started.md) · Next: [Stop a macro](stop-a-macro.md) · [Guide home](README.md)
