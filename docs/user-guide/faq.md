# Can Relay…?

Yes or no, and what to do instead.

- [Recording and playing](#recording-and-playing)
- [Running by itself](#running-by-itself)
- [Text, sharing and files](#text-sharing-and-files)
- [Keys, screens and accessibility](#keys-screens-and-accessibility)

## Recording and playing

### Can I record more onto an existing macro?

**No.** Record a new macro, or add steps with **Add**. [Recording limits](record.md#not-possible-yet)

### Are there default play options?

**No.** **Duplicate** a macro with the options you want. [Play limits](play.md#not-possible-yet)

### Can I start a data file at a row, or repeat it?

**No.** Make a data file with only the rows you need. [Data file limits](spreadsheet.md#not-possible-yet)

### Can Find image match another theme, or pick its click point?

**No.** Snip the image again in that theme. Set the click point in the **Step** tab. [Find image limits](find-image.md#not-possible-yet)

## Running by itself

### Can a macro run monthly?

**No.** Export it as a standalone program, then run it from **Task Scheduler**. [Trigger limits](run-automatically.md#not-possible-yet)

### Can I list every schedule?

**Yes, for schedules that are on.** The Library's **Also runs** column shows them. [Find a macro](library.md#find-a-macro)

### Does a failed run notify me?

**No.** With **Sounds** on, a failed run plays the error sound. Check **Problems** in Activity. [Activity limits](what-ran.md#not-possible-yet)

### Do standalone programs write a log or show in Activity?

**No.** Read the program's exit code. [Exit codes](scripts.md#exit-codes)

## Text, sharing and files

### Can {date} use another format?

**No.** Copy a date in your format first. Type `{clipboard}` instead. [Text step limits](text.md#not-possible-yet)

### What does {clipboard} type if nothing is copied?

**Nothing.** The macro continues without a warning. [Text step facts](text.md#good-to-know)

### Can I use folders or tags?

**No.** Start related macro names with the same word. [Library limits](library.md#not-possible-yet)

### Can I print a list of hotkeys?

**No.** The Library's **Hotkey** column lists them. [Library limits](library.md#not-possible-yet)

### Do exports keep hotkeys and other triggers?

**No.** Set them again after **Import…**. [What an export holds](library.md#good-to-know)

## Keys, screens and accessibility

### Does Relay work on several monitors?

**Yes.** Clicks are positions across all monitors. If the monitor layout changes, clicks land elsewhere. [Make clicks follow a moved window](play.md#make-clicks-follow-a-moved-window)

### Can Relay's keys work only in Relay?

**No.** Set **Record**, **Play / pause** and **Show the player** to **None**. **Stop everything** can't be **None**. [Change Relay's keys](keys.md#change-relays-keys)

### Can a hotkey play only while held, or warn about AltGr?

**No.** Use <kbd>Ctrl</kbd>+<kbd>Shift</kbd> with a letter. [Key limits](keys.md#not-possible-yet)

### Can Relay leave screen-reader keys out of a recording?

**No.** Find them with **Find a step**, then delete them. [Accessibility limits](accessibility.md#not-possible-yet)

### Does the player announce waits?

**No.** Standalone programs show *WAITING*. [Accessibility limits](accessibility.md#not-possible-yet)

### Is Relay on Mac, Linux or in other languages?

**No.** Relay runs only on Windows and in English. It installs for one Windows user. [Install limits](getting-started.md#not-possible-yet)

---

Previous: [Fix a problem](troubleshooting.md) · Next: [Words used in Relay](glossary.md) · [Guide home](README.md)
