# File formats

Everything Relay stores is JSON (plus one JPEG per recorded macro) in the data folder, `%APPDATA%\Relay` or `RELAY_DATA_DIR`. Every file is written atomically. This page is the reference for each file.

```text
%APPDATA%\Relay\
├── macros\<uuid>.rly         one macro per file          portable
├── macros\.trash\<uuid>.rly  deleted macros
├── screens\<uuid>.jpg        the screen as each recording started
├── library.json              order, run counts, triggers, data files, trash
├── runs.json                 the run history
├── settings.json             settings, Relay's keys, the first run
├── window.json               player mode, window positions, the divider
└── logs\relay.YYYY-MM-DD.log
```

Only `.rly` files are **portable**. Everything else is **machine-local**: how this PC uses the macros and Relay. Exports and imports never carry it; a screenshot is machine-local too, since it can show anything that was on screen.

- [.rly](#rly)
- [The JSON export](#the-json-export)
- [Exported programs](#exported-programs)
- [Versions and migrations](#versions-and-migrations)
- [library.json](#libraryjson)
- [runs.json](#runsjson)
- [settings.json](#settingsjson)
- [window.json](#windowjson)
- [Damaged files](#damaged-files)

## .rly

A single-line JSON object. Pretty-printed (from the snapshot test in `format.rs`):

```json
{
  "format": "relay-macro",
  "version": 1,
  "id": "00000000-0000-0000-0000-000052454c59",
  "name": "Save the file",
  "created_at": "2026-09-24T09:00:00Z",
  "modified_at": "2026-09-24T09:00:00Z",
  "recording": {
    "os": "windows",
    "virtual_desktop": { "x": 0, "y": 0, "w": 1920, "h": 1080 },
    "monitors": [{ "name": "\\\\.\\DISPLAY1", "rect": { "x": 0, "y": 0, "w": 1920, "h": 1080 },
                   "work": { "x": 0, "y": 0, "w": 1920, "h": 1032 }, "dpi": 96, "primary": true }],
    "double_click_ms": 500,
    "double_click_px": 4
  },
  "playback": { "speed": 1.0, "repeat": { "count": 1 }, "humanize": true, "jitter_ms": 40,
                "stop_on_key": true, "coord_mode": "screen" },
  "events": [
    { "type": "move", "t": 0, "x": 960, "y": 540 },
    { "type": "button", "t": 120, "x": 960, "y": 540, "btn": "Left", "down": true, "label": "Save" },
    { "type": "button", "t": 180, "x": 960, "y": 540, "btn": "Left", "down": false },
    { "type": "key", "t": 400, "down": true, "key": { "code": "KeyS", "vk": 83, "scan": 31 }, "ch": "s" },
    { "type": "key", "t": 450, "down": false, "key": { "code": "KeyS", "vk": 83, "scan": 31 } },
    { "type": "pixel_wait", "t": 600, "dur": 900, "x": 10, "y": 20, "color": "#EC3013", "tolerance": 8, "timeout_ms": 5000 }
  ]
}
```

- **Times** (`t`, `dur`) are milliseconds from the start. **Positions and rects** are physical pixels on the virtual desktop; negative left of or above the primary monitor.
- **`recording.anchor_window`** (optional: `exe`, `class`, `title`, `rect`) is the window under the first click, for *Window* coordinates.
- **`repeat`** is `{"count": n}` or `"forever"`. **`coord_mode`** is `"screen"` or `"window"`.
- **Optional fields are omitted** when empty or zero.

| Event `type` | Fields |
| --- | --- |
| `move` | `x`, `y` |
| `button` | `x`, `y`, `btn` (`Left`, `Right`, `Middle`, `X1`, `X2`), `down`, `label`? (on the press) |
| `wheel` | `x`, `y`, `delta` (120 = one notch up or right), `horizontal`? |
| `key` | `down`, `key: { code, vk?, scan?, ext? }`, `ch`? (what the press typed). A hand-written file only needs `code`, a [W3C key code](https://www.w3.org/TR/uievents-code/). |
| `wait` | `dur`, `label`? |
| `pixel_wait` | `dur`, `x`, `y`, `color` (`"#RRGGBB"`), `tolerance` (per channel, 0–255), `timeout_ms`, `label`? |
| `find_image` | `dur`, `image` (base64 PNG, ≤ 512 px a side), `click_x`, `click_y` (in the image's pixels), `btn`, `threshold` (50–100), `timeout_ms`, `area`?, `label`? |
| `text` | `dur`, `text` (a template: [Text steps](../spec/text-steps.md)) |

**Loading normalizes**, so a hand-edited file needn't be perfect: events are sorted, orphan releases dropped, unreleased presses released at the end, and anything inside a wait, check or Text step moved to its end.

## The JSON export

**Export → JSON events** writes the same object pretty-printed, plus `steps`: the derived steps, as the editor shows them (`items` are indices into `events`). It's for reading and scripts:

```bash
jq '[.steps[] | select(.kind == "click")] | length' invoice.json
```

Importing a `.json` ignores `steps` and rebuilds them.

## Exported programs

A **Standalone program** is relay-player's exe with the macro appended:

| Bytes | Content |
| --- | --- |
| … | The player, a normal Windows program |
| *n* | The macro as a compact `.rly`, plus its data file's rows as `"data": { "columns", "rows" }` if it has one |
| 8 | *n*, little-endian u64 |
| 4 | The bundle version, 1, little-endian u32 |
| 8 | `RELAYRLY` |

Windows ignores what follows the image, so the program runs as is and reads the macro from its own file (`format::from_program`). Import recognizes one by its `MZ` header. Loading it as a macro ignores `data`, so older Relays import these too. Appending the macro would break a signature, so exported programs can't be signed.

## Versions and migrations

`version` changes only when the event model does, and a macro is written with the **lowest version that holds it**, so older Relays keep opening it:

| Version | Since | Adds |
| --- | --- | --- |
| 0 | The M0 prototype | High-level events (`click`, `key` combos, `char`, `cond`), migrated by `migrate_v0` |
| 1 | 0.2 | Low-level presses and releases |
| 2 | 1.4 | `find_image` (written only when the macro has one) |
| 3 | 1.5 | `text` (written only when the macro has one) |

Loading: not a `"format": "relay-macro"` object → *not a Relay macro*; a newer version → *saved by a newer Relay*; an older one → migrations, one version at a time, on `serde_json::Value`; then deserialize and normalize.

Changing the format:

1. Bump `VERSION` in `format.rs`, and write the new version only for macros that need it.
2. Add `migrate_vN` from the previous version, on `serde_json::Value`.
3. Add a fixture of the old format and snapshot its migration.
4. Update this page and [Export and import](../spec/export-import.md).

## library.json

```json
{
  "version": 1,
  "order": ["3f0c…", "8a21…"],
  "entries": {
    "3f0c…": {
      "runs": 12,
      "last_run": "2026-09-24T07:00:03Z",
      "triggers": {
        "hotkey": { "enabled": true, "combo": "Ctrl + Alt + 1" },
        "schedule": { "enabled": true, "schedule": { "days": [true, true, true, true, true, false, false], "time": "09:00" } },
        "interval": { "enabled": false, "every": 10, "unit": "min" },
        "app_launch": { "enabled": false, "exe": "", "delay_ms": 2000 },
        "pixel": { "enabled": false, "x": 0, "y": 0, "color": "#EC3013", "tolerance": 8 },
        "image": { "enabled": false, "image": null, "threshold": 85, "area": null }
      },
      "data_file": "C:\\Users\\me\\Documents\\customers.csv"
    }
  },
  "trash": {
    "5b77…": { "position": 2, "next": "9c1e…", "name": "Weekly report", "runs": 0, "last_run": null, "triggers": { … } }
  }
}
```

- **`order`** is the Library's order. Macro files it doesn't list come after, newest first.
- **Every entry field has a default**, so partial entries load, and older Relays ignore fields they don't know (`interval`, `data_file`).
- **Schedules:** `days` is Monday first, `time` local `"HH:MM"`.
- **`data_file`** is an absolute path, read each time the macro plays, never copied.
- **`trash.next`** is the macro that came after the deleted one; a restore follows it (through other trashed macros) to put the macro back in place.

## runs.json

The run history, oldest first, the newest 200 kept (`runlog::MAX_RUNS`):

```json
{
  "version": 1,
  "entries": [
    {
      "at": "2026-09-30T07:00:02Z", "macro_id": "3f0c…", "macro_name": "Daily report",
      "source": "schedule",
      "outcome": { "type": "finished", "reason": "pixel_timeout" },
      "duration_ms": 8420, "from_ms": 0, "loops": 1, "speed": 1.0, "humanize": false,
      "checks": [
        { "step": 7, "loop_idx": 0, "image": false, "after_ms": 5000, "outcome": { "type": "timed_out" } }
      ],
      "checks_dropped": 0,
      "rows": 3
    }
  ]
}
```

- **`macro_name`** is the name at the time; the macro may be renamed or gone since.
- **`source`**: `manual`, `hotkey`, `schedule`, `interval`, `app_launch`, `pixel` or `image`.
- **`outcome`**: `finished` with a reason (`completed`, `stopped`, `key_pressed`, `killed`, `pixel_timeout`, `error`), or `skipped` with `busy`, `locked`, `missed` or `data_file` (plus a `note`).
- **`checks`** keeps a run's last 50 pixel checks and Find image steps; `step` is the number the UI shows (moves don't count).
- **`rows`** is only there for a run from a data file.

## settings.json

Read field by field: unknown fields are ignored, and a missing one takes its default. What each setting does and its default is [Settings](../spec/settings.md). The fields that need explaining:

| Field | Notes |
| --- | --- |
| `keys` | Relay's keys as the UI shows them: `record`, `play`, `player` (a combo or `null`) and `kill` (never `null`) |
| `intro` | What the first run shows next: `"welcome"`, `"keys"` (once, after an upgrade) or `"done"` |
| `glass` | `true`, `false`, or `null` for this Windows' default |

**A missing value depends on the folder.** In one an older Relay used (it has `settings.json`, `library.json` or `macros\`), `keys` defaults to F9, F10, Ctrl + Shift + M and Ctrl + Alt + End, and `intro` to `"keys"`, so an upgrade keeps the keys its user knows (`Settings::upgraded`). A new folder gets the new install's keys and `"welcome"`.

*Start with Windows* isn't stored here: it's an entry in `HKCU\…\CurrentVersion\Run` that starts Relay with `--autostart`.

## window.json

```json
{ "player_mode": false, "anchor": [1280, 1384], "size": [1100, 720],
  "panes": { "inspector_w": 380 }, "player_anchor": [2240, 120] }
```

Only Rust writes it, and every field may be missing (`null`: the default). `anchor` and `player_anchor` are each window's bottom center in physical pixels, moved only by the user's drag. `size` is the main window's size in CSS pixels once the user resized it. `panes.inspector_w` is the editor's divider.

## Damaged files

| File | Can't be parsed | Exists but can't be read (locked) |
| --- | --- | --- |
| `library.json` | Set aside as `library.json.bad` | Left alone; the index isn't written for the rest of the run |
| `runs.json` | Set aside as `runs.json.bad` | Left alone until Relay restarts |
| `settings.json` | Only the invalid fields reset; the file is set aside as `settings.json.bad` | Left alone |
| a `.rly` | Skipped, and the user is told | Skipped |

The rule behind all of them: **a file that exists but can't be read is never overwritten.**

---

<p align="center"><a href="frontend.md">← The frontend</a> · <a href="testing.md">Testing and CI →</a></p>
