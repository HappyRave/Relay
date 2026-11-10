# Installing, files and logs

What Relay needs to run, how it's installed, updated and removed, the files it keeps, its logs, and what it never sends anywhere.
Rules are numbered INST-n. Cite them in tests and PRs.

- [Requirements](#requirements)
- [The installer](#the-installer)
- [The portable exe](#the-portable-exe)
- [SmartScreen and antivirus](#smartscreen-and-antivirus)
- [Updating](#updating)
- [The data folder](#the-data-folder)
- [Logs](#logs)
- [Uninstalling](#uninstalling)
- [Privacy](#privacy)

## Requirements

- **INST-1** Relay runs on Windows 10 or 11, 64-bit.
- **INST-2** Relay needs the Microsoft WebView2 runtime. Windows 11 includes it, and most Windows 10 PCs get it with Microsoft Edge.
- **INST-3** Relay is Windows only: there is no macOS or Linux version yet. It's built so other systems can be added later.

## The installer

- **INST-4** The installer is **`Relay_<version>_x64-setup.exe`**, on the [releases page](https://github.com/HappyRave/Relay/releases).
- **INST-5** The installer installs Relay for the current Windows user only, so it doesn't ask for administrator rights.
- **INST-6** The installer adds the WebView2 runtime when it's missing.
- **INST-7** The installed Relay starts from the Start menu, as **Relay**.

## The portable exe

- **INST-8** The portable exe is **`Relay_<version>_x64-portable.exe`**, on the same releases page. It's Relay on its own: no installer, no Start menu entry. It runs from any folder with a double-click.
- **INST-9** The portable exe can't add the WebView2 runtime: it needs it already installed (see INST-2).
- **INST-10** The portable exe keeps macros and settings in the same data folder as the installed Relay (see INST-14), so the two share the same Library.

## SmartScreen and antivirus

- **INST-11** Relay isn't code-signed, so Windows SmartScreen may show *"Windows protected your PC"* for the installer or the portable exe. **More info → Run anyway** runs it. The source and the build scripts are all in the repository.
- **INST-12** An antivirus that removes an exported program may remove `relay.exe` too, as the program that wrote the file. It's then in the antivirus's quarantine, to restore from there, or Relay is reinstalled. (Exported programs and the antivirus: see [exported programs](export-import.md).)

## Updating

- **INST-13** The portable exe is updated by replacing the file with a newer one. **Start with Windows** remembers where the file is, so after moving the file it has to be turned off and on again (see [Start with Windows](windows.md)).

## The data folder

- **INST-14** Everything Relay keeps is in `%APPDATA%\Relay` (usually `C:\Users\<you>\AppData\Roaming\Relay`): macros, settings and logs.

The data folder holds:

| Rule | Path | Holds |
| --- | --- | --- |
| **INST-15** | `macros\<id>.rly` | One file per macro |
| **INST-16** | `macros\.trash\` | Deleted macros (see [the trash](library.md)) |
| **INST-17** | `screens\` | The screenshots taken as recordings start, for the preview |
| **INST-18** | `library.json` | Run counts, triggers and data files |
| **INST-19** | `runs.json` | Activity's runs |
| **INST-20** | `settings.json` | The settings |
| **INST-21** | `window.json` | Where the window and the player sit, and player mode |
| **INST-22** | `logs\` | Diagnostics from the last 7 days |

- **INST-23** **Open macros folder** in the tray menu opens `%APPDATA%\Relay\macros` in Explorer (see [the tray menu](windows.md)).

## Logs

- **INST-24** Relay writes a diagnostic log to `%APPDATA%\Relay\logs`, one file per day, and keeps the last 7 days.
- **INST-25** The log records what Relay did: sessions starting and stopping, triggers firing or being skipped, errors, and playback timing.
- **INST-26** The log never records what the user types, nor the contents of macros.
- **INST-27** Problems are reported as [issues](https://github.com/HappyRave/Relay/issues) on the repository; the log of the day the problem happened is the one that helps.

## Uninstalling

- **INST-28** The installed Relay is uninstalled from Windows' **Settings → Apps → Installed apps**.
- **INST-29** The portable exe is removed by deleting the file.
- **INST-30** Uninstalling leaves `%APPDATA%\Relay` in place, macros and settings included, in case Relay is reinstalled. Deleting that folder removes them.

## Privacy

- **INST-31** Relay never connects to the internet: it sends nothing anywhere.
- **INST-32** Macros, settings and logs stay on the PC, in `%APPDATA%\Relay`.

---

[Spec index](README.md)
