# File formats

Everything Relay stores is JSON in the data directory, `%APPDATA%\Relay` (or `RELAY_DATA_DIR`). Every file is written atomically: a temporary file, then a rename.

```text
%APPDATA%\Relay\
├── macros\<uuid>.rly       one macro per file, portable
├── macros\.trash\<uuid>.rly
├── library.json            machine-local: order, stats, triggers, trash
├── settings.json           machine-local
├── window.json             machine-local
└── logs\relay.YYYY-MM-DD.log
```

**Portable** files can be copied to another PC. **Machine-local** files describe this PC's use of the macros (run counts, hotkeys, schedules) and are never exported.

- [The .rly macro file](#the-rly-macro-file)
- [The JSON export](#the-json-export)
- [Versioning and migrations](#versioning-and-migrations)
- [library.json](#libraryjson)
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
| `version` | integer | `1`. See [migrations](#versioning-and-migrations). |
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

**Keys**: `code` is the W3C [`KeyboardEvent.code`](https://www.w3.org/TR/uievents-code/) of the physical key (`KeyA`, `Digit1`, `ShiftLeft`, `Enter`, `ArrowLeft`, `Numpad5`, `F9`…). `vk` is the Windows virtual-key code, `scan` the hardware scan code (set 1) and `ext` the extended-key flag. Playback prefers `scan`, then `vk`, then a scan code looked up from `code`, then types `ch` as Unicode. A hand-written file only needs `code`.

### Rules a valid file follows

Relay enforces these when loading (by normalizing), so a hand-edited file doesn't need to be perfect:

- Events are sorted by `t`.
- Every `down` has a matching release later. Releases without a press are dropped, and presses without a release are released at the end.
- Nothing happens inside a `wait` or `pixel_wait` (between `t` and `t + dur`).

## The JSON export

**Export → JSON events** writes the same object, **pretty-printed**, plus a `steps` array: the derived steps, as the editor shows them. It's for reading and for scripts. For example, count the clicks:

```bash
jq '[.steps[] | select(.kind == "click")] | length' export-invoice-to-pdf.json
```

A step looks like:

```json
{ "t": 120, "end": 180, "items": [1, 2], "kind": "click", "x": 960, "y": 540, "btn": "Left", "count": 1, "label": "Save" }
```

`items` are indices into `events`. Importing a `.json` export ignores `steps` and rebuilds them from the events.

## Versioning and migrations

`version` only changes when the event model changes incompatibly. Loading does:

1. not JSON, or no `"format": "relay-macro"` → *not a Relay macro*,
2. `version` newer than this Relay supports → *this macro was saved by a newer Relay (format version N)*,
3. older → run the migration chain on the raw JSON, one version at a time,
4. deserialize, then normalize.

| Version | Written by | Shape |
| --- | --- | --- |
| **0** | The M0 prototype's export | High-level events: `click` (with `count`), `key` combos like `"Ctrl + A"`, `char`, `wait`, `cond`. No recording metadata. |
| **1** | Relay 0.2 and later | Low-level presses and releases, as above |

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
        "pixel": { "enabled": false, "x": 0, "y": 0, "color": "#EC3013", "tolerance": 8 }
      }
    }
  },
  "trash": {
    "5b77…": { "position": 2, "runs": 0, "last_run": null, "triggers": { … } }
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
| `trash` | Deleted macros: where they were and their stats, so *Undo* restores them exactly |

Before triggers existed (up to M6), entries had a plain `hotkey` label. It's read as a disabled hotkey trigger.

## settings.json

```json
{
  "capture_moves": true,
  "capture_keys": true,
  "countdown": true,
  "ignore_injected": true,
  "path_mode": "full",
  "show_click_labels": true,
  "close_to_tray": true
}
```

`path_mode` is `"full"` or `"trail"`. Missing fields take their defaults (shown above), and unknown fields are ignored. *Start with Windows* isn't stored here: it's an entry in `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` (managed by `tauri-plugin-autostart`) that starts Relay with `--autostart`.

## window.json

```json
{ "expanded": true, "anchor": [1280, 1384] }
```

`anchor` is the widget's bottom-center in physical virtual-desktop pixels, or `null` for the default position. Only Rust writes this file.

---

<p align="center"><a href="frontend.md">← The frontend</a> · <a href="README.md">Contents</a> · <a href="testing.md">Testing and CI →</a></p>
