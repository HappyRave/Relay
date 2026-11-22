# Data files

What a macro's data file is, how it drives one run per row, when Relay refuses to play with it, and how its rows show and travel.

Rules are numbered DATA-n. Cite them in tests and PRs.

- [The data file](#the-data-file)
- [The CSV format](#the-csv-format)
- [One run per row](#one-run-per-row)
- [Columns in Text steps](#columns-in-text-steps)
- [When a macro can't play with its file](#when-a-macro-cant-play-with-its-file)
- [Rows while playing and in Activity](#rows-while-playing-and-in-activity)
- [Copies and exports](#copies-and-exports)

## The data file

- **DATA-1** **Data file** is one of a macro's play options, in the inspector's **Macro** tab under **Play options → Data file**. Its default is **None**.
- **DATA-2** **Choose…** picks a CSV file. Once chosen, the option shows the file's name and its number of rows: *customers.csv · 2 rows*.
- **DATA-3** **Change…** picks another file. **Remove** removes the data file and goes back to the repeat the macro had before.
- **DATA-4** Relay keeps where the file is, not a copy of it. If the file is moved, it has to be chosen again.
- **DATA-5** Relay reads the file each time the macro plays, so changes made to it between runs (in Excel, say) are used by the next run.
- **DATA-6** The data file belongs to the macro on this PC only: a macro's data file is remembered on this PC, in `library.json`, not in the macro's file.

## The CSV format

- **DATA-7** The file is a CSV whose first row names the columns. Every following row is one row of data:

  ```text
  Customer;Invoice;Total
  ACME Ltd;F-1042;1250,00
  Globex;F-1043;87,50
  ```

- **DATA-8** Commas and semicolons both work as separators, and so do tabs (tab-separated files). Excel's **CSV** and **CSV UTF-8** formats both work.
- **DATA-9** A value in quotes can hold the separator, quotes (doubled: `""`) and new lines.
- **DATA-10** Column names are matched whatever their case.
- **DATA-11** Empty lines are skipped: they aren't rows.
- **DATA-12** A row with fewer values than there are columns types nothing for the missing ones.

## One run per row

- **DATA-13** A macro with a data file plays **once per row**, and each run types that row's values.
- **DATA-14** While a macro has a data file, **Repeat** says *Once per row of the data file (2 rows)* instead of its count, and the summary next to **Play** shows *Each row (2)* instead of *×N* or *∞*. The number is the file's row count.

## Columns in Text steps

- **DATA-15** A [Text step](text-steps.md) types a column of the data file with `{col:Name}`: the *Name* column's value in this run's row (`{col:Customer}` types `Globex` in the row above).
- **DATA-16** In a macro with a data file, `{n}` is the row number, from 1.
- **DATA-17** The Text step's field has a placeholder button for each column of the macro's data file. What the text would type, and how a column the file doesn't have shows there, is in [Text steps](text-steps.md).
- **DATA-18** A Text step that types a column the macro has no data file for, or that its file doesn't have, is still saved; the macro just won't play until the file has the column (DATA-19).

## When a macro can't play with its file

- **DATA-19** Relay won't play a macro with a data file, and says why, if:
  - the file isn't there;
  - a row has more values than there are columns;
  - a Text step types a column the file doesn't have;
  - the file has no rows.
- **DATA-20** The reason shows under **Data file**, and again when **Play** is pressed.
- **DATA-21** Triggered runs are refused the same way. Activity lists the trigger as *Skipped: data file*, with what was wrong (see [Activity and Problems](library.md#activity-and-problems)).

## Rows while playing and in Activity

- **DATA-22** While a macro with a data file plays, the player shows the row instead of the time: *Row 1 / 2* (*Row 37 / 200*), while *PLAYING* or *PAUSED*. See [the player](windows.md#the-player).
- **DATA-23** While it plays, the editor's Step tab shows the row as well as the step playing (*Now · step 5 of 12*): *Row 3 of 20*.
- **DATA-24** In Activity, a run from a data file shows its rows in **Rows or loops**: *12 rows*.
- **DATA-25** A run's checks in Activity name the row they happened in: *Row 2 of 12 · Step 6 · Image found at 812, 344 (93 %) after 0.4 s*.

## Copies and exports

- **DATA-26** A copy made with **Duplicate** keeps the original's data file: both use the same file.
- **DATA-27** A `.rly` export doesn't carry the data file.
- **DATA-28** A [standalone program](export-import.md) carries the rows the file had when it was exported, and plays once per row of them. Changes to the file afterwards need a new export.
- **DATA-29** An exported program shows the row while it plays: *Row 3 / 12*.
- **DATA-30** An exported program of a macro with a data file plays once per row whatever `--repeat` says.
- **DATA-31** An AutoHotkey v2 export of a macro with a data file has its rows in the `Rows` variable and plays once per row.
- **DATA-32** Relay won't export a macro as a standalone program or an AutoHotkey script if it can't play its file (the file is missing, or lacks a column a Text step types), and says why.

---

[Spec index](README.md)
