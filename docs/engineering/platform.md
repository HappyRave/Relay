# relay-platform

`crates/relay-platform` is the boundary with the OS. It defines a trait for each OS service, implements them for Windows in `src/windows/`, and has a stub so everything else builds and tests on Linux. The recorder, the key map and the picker's layout are OS-independent and live here too.

- [The traits](#the-traits)
- [The input hook](#the-input-hook)
- [Injection](#injection)
- [The precision timer](#the-precision-timer)
- [Screen and captures](#screen-and-captures)
- [The picker](#the-picker)
- [Show targets](#show-targets)
- [Keys and characters](#keys-and-characters)
- [The recorder](#the-recorder)
- [Other backends](#other-backends)

## The traits

[`platform()`](../../crates/relay-platform/src/lib.rs) returns the backend for the current OS:

| Trait | Does | Windows |
| --- | --- | --- |
| `InputHook` | Starts a hook session; dropping it removes the hooks | `WH_KEYBOARD_LL` + `WH_MOUSE_LL` |
| `Injector` | Mouse, keys, text | `SendInput` |
| `Timer` | `wait_until(deadline)`, wakeable | High-resolution waitable timer + spin |
| `Screen` | Monitors, pixel, capture, mark, pick | GDI, DWM, layered windows |
| `WindowQuery` | Window under a point, foreground, integrity, lock screen | Win32, DWM, tokens |
| `CharTranslator` | What a key press types | `ToUnicodeEx` |
| `Clipboard` | Image, text, the snipping overlay | Clipboard formats, `ms-screenclip:` |

- **Injectors, translators and timers come from factories** (`fn() -> Box<dyn …>`), so each thread owns its own.
- **One clock:** `now_ms` is `QueryPerformanceCounter` in milliseconds. Hook timestamps and engine deadlines both use it.
- **Two own windows:** whatever must leave Relay out takes `OwnWindows` (the main window and the player; an exported program has one).
- **Physical pixels:** the app is per-monitor DPI aware (v2), so every recorded or injected coordinate is a physical pixel and display scaling never distorts a macro.

## The input hook

[`windows/hook.rs`](../../crates/relay-platform/src/windows/hook.rs). Windows **silently removes a low-level hook that takes too long**, so:

- The hooks live on their own `relay-hook` thread. Dropping the session joins it, so a hook can't outlive its session.
- Callbacks **only filter, timestamp and `try_send`** to a bounded channel. If it's full, the event is dropped rather than stalling Windows' input queue.
- The app's [watchdog](app.md#recording-and-the-watchdog) still detects a hook lost under load and reinstalls it. Every mouse event beats `mouse_pulse`, filtered or not, so it doesn't false-alarm.

What the callback decides, with a `HookConfig` fixed for the session:

| Input | Fate |
| --- | --- |
| Tagged `RELAY_MAGIC` (`0x52454C59`) in `dwExtraInfo`, or any injected input with *Ignore simulated input* | Passed, not reported. Relay's own input is recognized even with the setting off. |
| The kill switch (a `KeyChord` from settings) | Always passed through to its hotkey |
| Recording: a press over Relay's windows, a key while one is in front, the record key | Passed, not reported |
| Recording: Esc, while *Esc stops recording* is on | Reported as `Escape`, swallowed |
| Playing (`Watch`): Esc, or with *Stop on key press* any key but a modifier or the play key | Reported as a stop (`Escape` / `StopKey`), swallowed |
| A release or auto-repeat | **The same fate as its press**, so the app never sees a release without its press |

"Over Relay's windows" is looked up per press (`WindowFromPoint`), never per move, so it's right wherever the windows are and costs nothing on the frequent moves.

## Injection

[`windows/inject.rs`](../../crates/relay-platform/src/windows/inject.rs). Every `INPUT` carries `RELAY_MAGIC`. A key is sent in the most faithful form known:

| Known | Sent as | Why |
| --- | --- | --- |
| Scan code | `KEYEVENTF_SCANCODE` | The physical key, which games and DirectInput expect |
| Virtual key only | Virtual key | Input that was itself injected still types the right character |
| W3C code only | `keymap::scan_for_code` | Hand-written or migrated macros |
| Character only | `KEYEVENTF_UNICODE` | Last resort |

Text steps type with `KEYEVENTF_UNICODE` whatever the layout, except line feeds and tabs, which are sent as `VK_RETURN` and `VK_TAB` because apps read the Unicode ones as characters.

Absolute moves are normalized across the virtual desktop, which can land a pixel off at some scalings, so `move_to` reads the cursor back and corrects it. When `SendInput` inserts fewer events than asked, it's almost always UIPI (the target runs as administrator), and the error says so.

## The precision timer

[`windows/timer.rs`](../../crates/relay-platform/src/windows/timer.rs). `thread::sleep` has 1–15 ms of slack on Windows, which fast macros show. The timer sleeps on a `CREATE_WAITABLE_TIMER_HIGH_RESOLUTION` timer until 1 ms before the deadline, then spins. It waits on a wake event too, so a pause, seek or stop interrupts it at once.

Creating it raises the thread to `THREAD_PRIORITY_HIGHEST` and opts the process out of power throttling, which Windows 11 applies to background processes (Relay usually is, while it plays). Dropping it hands throttling back, so an idle Relay doesn't cost battery.

## Screen and captures

[`windows/screen.rs`](../../crates/relay-platform/src/windows/screen.rs)

- **`pixel`** (`GetPixel`) costs about 10 ms, because reading the screen waits for the compositor. Fine for checks every 30 ms and triggers every 250 ms.
- **The recording's screenshot** leaves Relay out with `WDA_EXCLUDEFROMCAPTURE`, set only for the capture and reset right after, so other screenshot tools still see Relay.
- **Image searches don't use that.** On several monitors the affinity can still be applying when the capture is taken. The app paints over what shows of Relay's windows instead: each one's frame (`shown_rect`) minus the windows in front of it (`covering`). A dialog over the main window is still searched.
- **The marker** (*Show* on a match) is a layered, click-through, non-activating window, excluded from capture before it shows, so no search ever finds it.

## The picker

`Screen::pick` blocks until the user picks a pixel or cancels. [`picker.rs`](../../crates/relay-platform/src/picker.rs) has the testable part: the keys (`pick_key`) and where the loupe and help bar go (`loupe_rect`, `help_rect`). [`windows/picker.rs`](../../crates/relay-platform/src/windows/picker.rs) draws three layered windows: the catcher, the loupe and the help bar.

The picker **never takes the focus**: every window is `WS_EX_NOACTIVATE` and answers `MA_NOACTIVATE`, and its keys come through its own keyboard hook, which swallows only the picker's keys. A menu open in the app underneath stays open. Freezing copies the screen without `CAPTUREBLT`, which leaves layered windows out, so the picker never shows up in its own loupe.

## Show targets

The marks ([spec](../spec/targets.md)) come from `relay_core::marks`, which turns steps into marks with the *Window* offset applied. [`overlay.rs`](../../crates/relay-platform/src/overlay.rs) has the testable part: sizes, timings, ripples, a path drawn part way, and where captions and number badges go. [`windows/overlay.rs`](../../crates/relay-platform/src/windows/overlay.rs) draws them with Direct2D into a bitmap and shows it with `UpdateLayeredWindow` (per-pixel alpha, so marks are smooth and fade).

- `Screen::live_mark` returns at once: one `relay-targets` thread owns every live mark, each a small window sized to what it draws, and animates them on a 15 ms thread timer until each has faded. The engine calls it as a step's first event plays (`Engine::set_marker`), so drawing never delays input.
- `Screen::show_targets` blocks like the picker: one window over the virtual desktop, a keyboard hook for <kbd>Esc</kbd>, and a faint wash so a click anywhere lands on it (a layered window's fully transparent pixels let clicks through).
- Every mark window is `WS_EX_TRANSPARENT` (live ones), `WS_EX_NOACTIVATE`, and `WDA_EXCLUDEFROMCAPTURE`, set before it shows: pixel checks, Find image and the triggers read the screen without them. A Windows test checks a mark shows and isn't in `Screen::pixel`.
- Screenshots can't show marks, so `cargo test -p relay-platform draws_the_marks -- --ignored` renders them to `target/targets-look.png` for a look.

## Keys and characters

- **[`keymap.rs`](../../crates/relay-platform/src/keymap.rs)** maps scan codes to W3C codes and back. Windows swaps Pause and NumLock's scan codes; the map follows, and Pause is injected by virtual key.
- **`KeyChord`** ([`types.rs`](../../crates/relay-platform/src/types.rs)) is a combination as the hook sees it, parsed from the UI's label. `pressed` matches its key with *at least* its modifiers, so a stray Shift still stops everything.
- **[`windows/text.rs`](../../crates/relay-platform/src/windows/text.rs)** asks `ToUnicodeEx` what a key typed, with the foreground window's layout and flag `0x4`. Without that flag, translating a dead key consumes it, and the user's next character comes out wrong in the app they're typing into. The same translator tells key warnings what Ctrl + Alt + a key types (AltGr on European layouts).

## The recorder

[`recorder.rs`](../../crates/relay-platform/src/recorder.rs) turns `RawInput`s into `Event`s. The translator is injected, so it's tested with synthetic input and a fake US layout.

- Cursor samples are throttled to one per 16 ms, but the last skipped one is kept before the next action, so the cursor is where it really was.
- The kill switch is dropped here as well as in the hook, so the recorder is right on its own.
- `finish` trims modifiers pressed after the last real action (those of the kill switch that stopped it), then normalizes. `is_meaningful` decides whether to keep the recording.

## Other backends

[`stub.rs`](../../crates/relay-platform/src/stub.rs) builds on every non-Windows target: hooks and injection return `Unsupported`, the screen is one 1920 × 1080 monitor. It exists so CI can build and test the portable crates on Linux.

A real macOS or Linux backend is a new module beside `windows/`, chosen with `#[cfg]` in `platform()`:

| Trait | macOS | Linux (X11) | Linux (Wayland) |
| --- | --- | --- | --- |
| `InputHook` | `CGEventTap` (Accessibility permission) | XRecord | No global capture by design: a portal or root `evdev` |
| `Injector` | `CGEventPost` | XTest | `uinput` |
| `Screen::pixel` | `CGWindowListCreateImage` | `XGetImage` | Screenshot portal |

The recorder, the key map and everything above them don't change. See [Recipes](recipes.md#add-an-os-backend).

---

<p align="center"><a href="core.md">← relay-core</a> · <a href="app.md">The app →</a></p>
