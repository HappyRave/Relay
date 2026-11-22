# Where Relay keeps things

Find your macros and logs, or uninstall Relay. Everything stays on this PC. Relay never connects to the internet.

- [Where your macros are](#where-your-macros-are)
- [What each file holds](#what-each-file-holds)
- [Send a log with a bug report](#send-a-log-with-a-bug-report)
- [Install for one user](#install-for-one-user)
- [Use the portable exe](#use-the-portable-exe)
- [Uninstall Relay](#uninstall-relay)

## Where your macros are

Your macros are in the data folder's `macros` folder.

1. Open File Explorer (<kbd>Win</kbd>+<kbd>E</kbd>).
2. Paste this into the address bar:

   ```text
   %APPDATA%\Relay\macros
   ```

3. Press <kbd>Enter</kbd>.

If it worked, File Explorer shows one `.rly` file for each macro.

Or, with a mouse, right-click Relay's icon in the [tray](glossary.md#tray), then select **Open macros folder**.

## What each file holds

All are in the data folder, `%APPDATA%\Relay`.

| File or folder | What it holds |
| --- | --- |
| `macros\` | One `.rly` file per macro. |
| `macros\.trash\` | Deleted macros. Import one to restore it. |
| `screens\` | A screenshot from the start of each recording. |
| `library.json` | Triggers, run counts, and the [data file](glossary.md#data-file) of each macro. |
| `runs.json` | The runs in **Activity**. |
| `settings.json` | Your settings. |
| `window.json` | Window positions, and whether player mode is on. |
| `logs\` | Logs from the last 7 days. |

For developers: [the .rly file format](../engineering/file-formats.md).

More detail: [Installing, files and logs: the data folder](../spec/install.md#the-data-folder)

## Send a log with a bug report

A log lists what Relay did. It never holds what you type or what your macros contain.

1. Open the `logs` folder in the data folder.
2. Find the log for the day of the problem, for example `relay.2026-11-03.log`.
3. Open a [new issue on GitHub](https://github.com/HappyRave/Relay/issues/new).
4. Attach the log file to the issue.

If it worked, the issue shows the log's file name.

## Install for one user

Relay installs for your Windows user only. It needs no administrator rights. It can't install for all users.

## Use the portable exe

The portable exe is Relay without an installer, on the [releases page](https://github.com/HappyRave/Relay/releases).

- It uses the same data folder as an installed Relay.
- It needs WebView2. Windows 11 includes it.
- If you move the file, turn **Start with Windows** off and on again in **Settings**.

## Uninstall Relay

Uninstalling keeps your data folder.

1. In Windows **Settings**, open **Apps**, then **Installed apps**.
2. Next to **Relay**, select **More options** (three dots), then **Uninstall**.
3. To also remove your macros, settings and **Activity**, delete the folder `%APPDATA%\Relay`.

If it worked, **Relay** is no longer in the Start menu.

For the portable exe, turn **Start with Windows** off. Then delete the file.

Before you reset a PC: [Back up your macros, or move to a new PC](library.md#back-up-your-macros-or-move-to-a-new-pc).

---

Previous: [Use Relay over Remote Desktop](remote-desktop.md) · Next: [Fix a problem](troubleshooting.md) · [Guide home](README.md)
