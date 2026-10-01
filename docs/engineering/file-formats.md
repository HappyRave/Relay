# File formats

Everything Relay stores is JSON (and one JPEG per recorded macro) in the data directory, `%APPDATA%\Relay` (or `RELAY_DATA_DIR`). Every file is written atomically: a temporary file, then a rename.

```text
%APPDATA%\Relay\
├── macros\<uuid>.rly       one macro per file, portable
├── macros\.trash\<uuid>.rly
├── screens\<uuid>.jpg       machine-local: the screen as each recording started
├── library.json            machine-local: order, stats, triggers, trash
├── runs.json               machine-local: the run history
├── settings.json           machine-local
├── window.json             machine-local
└── logs\relay.YYYY-MM-DD.log
```

**Portable** files can be copied to another PC. **Machine-local** files describe this PC's use of the macros (run counts, the run history, hotkeys, schedules) and are never exported.

**Screenshots** (`screens\<uuid>.jpg`) are taken when a recording starts, with *Screenshot* on: the whole virtual desktop at the macro's `recording.virtual_desktop`, scaled down to at most 3200 px wide, JPEG quality 80, with Relay's own window left out. The preview draws it at that rect. It's machine-local because it can show anything that was on screen: exports and imports never carry it. A duplicate gets a copy; a trashed macro keeps its screenshot for a restore. A macro without one (recorded before 1.3, imported, or with the setting off) shows the sketch.

- [The .rly macro file](#the-rly-macro-file)
- [The JSON export](#the-json-export)
- [Exported programs](#exported-programs)
- [Versioning and migrations](#versioning-and-migrations)
- [library.json](#libraryjson)
- [runs.json](#runsjson)
- [settings.json](#settingsjson)
- [window.json](#windowjson)

## The .rly macro file

A `.rly` file is a single-line JSON object. Here's a small one, pretty-printed (from the snapshot test in `format.rs`):

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
    "monitors": [
      {
        "name": "\\\\.\\DISPLAY1",
        "rect": { "x": 0, "y": 0, "w": 1920, "h": 1080 },
        "work": { "x": 0, "y": 0, "w": 1920, "h": 1032 },
        "dpi": 96,
        "primary": true
      }
    ],
    "double_click_ms": 500,
    "double_click_px": 4
  },
  "playback": {
    "speed": 1.0,
    "repeat": { "count": 1 },
    "humanize": true,
    "jitter_ms": 40,
    "stop_on_key": true,
    "coord_mode": "screen"
  },
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

### Top level

| Field | Type | Notes |
| --- | --- | --- |
| `format` | `"relay-macro"` | Required. Anything else is *not a Relay macro*. |
| `version` | integer | `2` if the macro has a `find_image` event, else `1`. See [migrations](#versioning-and-migrations). |
| `id` | UUID | Also the file name |
| `name` | string | |
| `created_at`, `modified_at` | RFC 3339 UTC | |
| `recording` | object | Where and how it was recorded |
| `playback` | object | This macro's playback options |
| `events` | array | Sorted by `t` |

### `recording`

| Field | Type | Notes |
| --- | --- | --- |
| `os` | string | `"windows"` |
| `virtual_desktop` | rect | All monitors' bounding box, physical pixels |
| `monitors` | array | `name`, `rect`, `work` (without the taskbar), `dpi`, `primary` |
| `double_click_ms`, `double_click_px` | integers | The system settings when recorded, used to group double clicks |
| `anchor_window` | object, optional | The top-level window under the first click: `exe`, `class`, `title`, `rect`. Used by *Window* coordinates. |

A rect is `{ "x", "y", "w", "h" }` in physical pixels. `x` and `y` can be negative for monitors left of or above the primary.

### `playback`

| Field | Type | Default | Notes |
| --- | --- | --- | --- |
| `speed` | number | `1.0` | The UI offers 0.5, 1, 2 and 4 |
| `repeat` | `{"count": n}` or `"forever"` | `{"count": 1}` | |
| `humanize` | bool | `true` | |
| `jitter_ms` | integer | `40` | ± per step, used when `humanize` is on |
| `stop_on_key` | bool | `true` | |
| `coord_mode` | `"screen"` or `"window"` | `"screen"` | |

### `events`

Every event has a `type` and `t`, the time in milliseconds from the start. Optional fields are omitted when empty or zero.

| `type` | Fields | Notes |
| --- | --- | --- |
| `move` | `x`, `y` | A cursor sample |
| `button` | `x`, `y`, `btn`, `down`, `label`? | `btn`: `Left`, `Right`, `Middle`, `X1`, `X2`. The click's label is on the `down` event. |
| `wheel` | `x`, `y`, `delta`, `horizontal`? | `delta` in wheel units: `120` is one notch up (or right), `-120` one down |
| `key` | `down`, `key`, `ch`? | `key` is `{ code, vk?, scan?, ext? }` (see below). `ch` is the text the press typed, on `down` events only. |
| `wait` | `dur`, `label`? | |
| `pixel_wait` | `dur`, `x`, `y`, `color`, `tolerance`, `timeout_ms`, `label`? | `color` is `"#RRGGBB"`. `tolerance` is per channel, 0–255. |
| `find_image` | `dur`, `image`, `click_x`, `click_y`, `btn`, `threshold`, `timeout_ms`, `area`?, `label`? | `image` is a PNG in base64 (at most 512 px a side). `click_x`, `click_y` are where to click, from the image's top-left in its own pixels (they scale with it). `threshold` is the lowest match accepted, 50–100 %. `area` is a rect to look in; without it, all monitors. |

**Keys**: `code` is the W3C [`KeyboardEvent.code`](https://www.w3.org/TR/uievents-code/) of the physical key (`KeyA`, `Digit1`, `ShiftLeft`, `Enter`, `ArrowLeft`, `Numpad5`, `F9`…). `vk` is the Windows virtual-key code, `scan` the hardware scan code (set 1) and `ext` the extended-key flag. Playback prefers `scan`, then `vk`, then a scan code looked up from `code`, then types `ch` as Unicode. A hand-written file only needs `code`.

### Rules a valid file follows

Relay enforces these when loading (by normalizing), so a hand-edited file doesn't need to be perfect:

- Events are sorted by `t`.
- Every `down` has a matching release later. Releases without a press are dropped, and presses without a release are released at the end.
- Nothing happens inside a `wait`, `pixel_wait` or `find_image` (between `t` and `t + dur`). An event inside one, or at its start but listed after it, is moved to its end.

## The JSON export

**Export → JSON events** writes the same object, **pretty-printed**, plus a `steps` array: the derived steps, as the editor shows them. It's for reading and for scripts. For example, count the clicks:

```bash
jq '[.steps[] | select(.kind == "click")] | length' export-invoice-to-pdf.json
```

A step looks like:

```json
{ "t": 120, "end": 180, "pause": 120, "items": [1, 2], "kind": "click", "x": 960, "y": 540, "btn": "Left", "count": 1, "label": "Save" }
```

`items` are indices into `events`, and `pause` is the idle time before the step. Importing a `.json` export ignores `steps` and rebuilds them from the events.

## Exported programs

A macro exported as a **Standalone program** is the player's exe with the macro appended:

| Bytes | Content |
| --- | --- |
| … | The player (`crates/relay-player`), a normal Windows program |
| *n* | The macro as a compact `.rly` (UTF-8), exactly as a `.rly` export would write it (v2 with a `find_image` event) |
| 8 | *n*, as a little-endian u64 |
| 4 | The bundle version, 1, as a little-endian u32 |
| 8 | `RELAYRLY` |

Windows loads the program's image and ignores what follows it, so the player runs as it is, and reads the macro from its own file. Import recognizes a program by its `MZ` header: without the trailer it's *not a program exported by Relay*, with a newer bundle version it's *saved by a newer Relay*. The player isn't signed, and appending the macro would invalidate a signature anyway, so an exported program can never carry Relay's.

## Versioning and migrations

`version` only changes when the event model changes incompatibly. Loading does:

1. not JSON, or no `"format": "relay-macro"` → *not a Relay macro*,
2. `version` newer than this Relay supports → *this macro was saved by a newer Relay (format version N)*,
3. older → run the migration chain on the raw JSON, one version at a time,
4. deserialize, then normalize. A Relay document that fails here (a bad value, a missing field, a missing `version`), or text that mentions `"relay-macro"` but isn't valid JSON (a truncated file), is *invalid macro file: …* with the reason.

| Version | Written by | Shape |
| --- | --- | --- |
| **0** | The M0 prototype's export | High-level events: `click` (with `count`), `key` combos like `"Ctrl + A"`, `char`, `wait`, `cond`. No recording metadata. |
| **1** | Relay 0.2 and later | Low-level presses and releases, as above |
| **2** | Relay 1.4 and later, for macros with a `find_image` event | Version 1 plus `find_image`. Loading a v1 file as v2 needs no change. A macro without one is still written as v1, so older Relays keep reading it; one with one is refused by them as too new rather than as invalid. |

`migrate_v0` expands each v0 event: a `click` with `count: 2` becomes two press/release pairs 120 ms apart, a combo becomes modifier presses around the key, a `char` becomes its US-layout key (with Shift if needed), and a `cond` becomes a `pixel_wait` with tolerance 8 and a 5 s timeout. Missing metadata defaults to a single 1920×1080 monitor.

To change the format:

1. Bump `VERSION` in `format.rs`.
2. Add `migrate_vN` from the previous version, working on `serde_json::Value`.
3. Add a fixture of the old format and snapshot its migration.
4. Update this page.

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
        "app_launch": { "enabled": false, "exe": "", "delay_ms": 2000 },
        "pixel": { "enabled": false, "x": 0, "y": 0, "color": "#EC3013", "tolerance": 8 },
        "image": { "enabled": false, "image": null, "threshold": 85, "area": null }
      }
    }
  },
  "trash": {
    "5b77…": { "position": 2, "next": "9c1e…", "name": "Weekly report", "runs": 0, "last_run": null, "triggers": { … } }
  }
}
```

| Field | Notes |
| --- | --- |
| `order` | Library order, top first. Macro files not listed are shown after, newest first. |
| `entries` | Per-macro stats and triggers. Every field has a default, so partial entries load. |
| `triggers.hotkey.combo` | Labels joined by `" + "`, modifiers first: `Ctrl`, `Alt`, `Shift`, `Win` |
| `triggers.schedule.schedule.days` | Monday first |
| `triggers.schedule.schedule.time` | Local `"HH:MM"` |
| `triggers.app_launch.exe` | File name, matched case-insensitively |
| `triggers.image.image` | A PNG in base64, as in `find_image`, or `null` until one is set |
| `trash` | Deleted macros and their stats, so *Undo* restores them exactly. `next` is the macro that came right after (`null` at the end); restoring follows it (through other trashed macros) to the first one still in the Library, so the macro returns to the right place even after others were added or restored. Older files have only `position`, used as is. `name` keeps recording names from repeating one that's in the trash. |

Before triggers existed (up to M6), entries had a plain `hotkey` label. It's read as a disabled hotkey trigger.

## runs.json

The run history (Library → Runs), oldest first. Relay keeps the newest 200 entries (`runlog::MAX_RUNS`).

```json
{
  "version": 1,
  "entries": [
    {
      "at": "2026-09-30T07:00:02Z",
      "macro_id": "3f0c…",
      "macro_name": "Daily report",
      "source": "schedule",
      "outcome": { "type": "finished", "reason": "pixel_timeout" },
      "duration_ms": 8420,
      "from_ms": 0,
      "loops": 1,
      "speed": 1.0,
      "humanize": false,
      "checks": [
        { "step": 4, "loop_idx": 0, "image": true, "after_ms": 380, "outcome": { "type": "found", "x": 812, "y": 344, "score": 93 } },
        { "step": 7, "loop_idx": 0, "image": false, "after_ms": 5000, "outcome": { "type": "timed_out" } }
      ],
      "checks_dropped": 0
    },
    {
      "at": "2026-09-30T08:00:00Z",
      "macro_id": "8a21…",
      "macro_name": "Backup",
      "source": "schedule",
      "outcome": { "type": "skipped", "reason": "missed" },
      "duration_ms": 0, "from_ms": 0, "loops": 0, "speed": 1.0, "humanize": false, "checks": [], "checks_dropped": 0
    }
  ]
}
```

| Field | Notes |
| --- | --- |
| `at` | When the run started, or when the skipped trigger fired |
| `macro_name` | The name at that time: the macro may be renamed or deleted since |
| `source` | `manual` (the Play button or F10), `hotkey`, `schedule`, `app_launch`, `pixel` or `image` |
| `outcome` | `finished` with a `FinishReason` (`completed`, `stopped`, `key_pressed`, `killed`, `pixel_timeout`, `error`), or `skipped` with `busy`, `locked` or `missed` (a schedule slept through) |
| `duration_ms` | Wall time, pauses included; 0 for a skip |
| `loops` | Loops played, the last one included even if it didn't finish |
| `checks` | The last 50 pixel checks and Find image steps (`runlog::MAX_CHECKS`), with `checks_dropped` counting earlier ones. `step` is 1-based, `loop_idx` 0-based, `after_ms` excludes pauses. `outcome` is `matched`, `found` (an image's top-left corner and score in percent), `timed_out` or `interrupted` (stopped while waiting). |

A run still going when Relay quits isn't recorded. Paused triggers and deleted macros aren't logged. Like `library.json`, a `runs.json` that exists but can't be read is left alone until Relay restarts, and one that doesn't parse is set aside as `runs.json.bad`. A failed save keeps the entry in memory and tells the user once, until a save succeeds again.

## settings.json

```json
{
  "capture_moves": true,
  "capture_keys": true,
  "countdown": true,
  "esc_stops_recording": true,
  "ignore_injected": true,
  "capture_screen": true,
  "path_mode": "full",
  "show_click_labels": true,
  "preview_background": "screen",
  "close_to_tray": true,
  "keep_on_top": "always"
}
```

`path_mode` is `"full"` or `"trail"`, `preview_background` is `"screen"` (the screenshot, when the macro has one) or `"sketch"`, and `keep_on_top` is `"always"`, `"sessions"` (while recording or playing) or `"never"`. Missing fields take their defaults (shown above), and unknown fields are ignored. A field with an invalid value (`"keep_on_top": "sometimes"`, or a value from a newer Relay) takes its default and the others are kept; the original file is set aside as `settings.json.bad`, the recovered settings are saved, and the user is told. *Start with Windows* isn't stored here: it's an entry in `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` (managed by `tauri-plugin-autostart`) that starts Relay with `--autostart`.

## window.json

```json
{
  "expanded": true,
  "anchor": [1280, 1384],
  "size": [1100, 720],
  "panes": { "preview_w": 520, "transport_h": null, "timeline_h": null }
}
```

`anchor` is the widget's bottom-center in physical virtual-desktop pixels, or `null` for the default position. `size` is the editor's size in CSS px if the user resized it (`null`: 944 × 612); it never goes below 760 × 520. `panes` are the editor's dividers in CSS px: the preview's width, the button row's height and the timeline's height, each `null` for the default (600, 102 and 146); the UI keeps them inside the window. Every field may be missing (older files), and only Rust writes this file.

---

<p align="center"><a href="frontend.md">← The frontend</a> · <a href="README.md">Contents</a> · <a href="testing.md">Testing and CI →</a></p>
