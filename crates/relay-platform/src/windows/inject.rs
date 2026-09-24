//! Input synthesis through SendInput. Every event is tagged with RELAY_MAGIC.

use relay_core::keys::KeyStroke;
use relay_core::model::MouseBtn;
use windows::Win32::Foundation::POINT;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBD_EVENT_FLAGS, KEYBDINPUT, KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP,
    KEYEVENTF_SCANCODE, KEYEVENTF_UNICODE, MOUSE_EVENT_FLAGS, MOUSEEVENTF_ABSOLUTE, MOUSEEVENTF_HWHEEL,
    MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP, MOUSEEVENTF_MOVE,
    MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP, MOUSEEVENTF_VIRTUALDESK, MOUSEEVENTF_WHEEL, MOUSEEVENTF_XDOWN,
    MOUSEEVENTF_XUP, MOUSEINPUT, SendInput, VIRTUAL_KEY,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetCursorPos, GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN,
    SetCursorPos,
};

use crate::{Injector, PlatformError, RELAY_MAGIC, Result, keymap};

pub struct SendInputInjector;

fn send(inputs: &[INPUT]) -> Result<()> {
    let sent = unsafe { SendInput(inputs, size_of::<INPUT>() as i32) };
    if sent as usize == inputs.len() {
        Ok(())
    } else {
        // SendInput doesn't say why; UIPI (an elevated target) is the usual cause.
        Err(PlatformError::Os("input was blocked (is the target app running as administrator?)".into()))
    }
}

fn mouse(dx: i32, dy: i32, data: u32, flags: MOUSE_EVENT_FLAGS) -> INPUT {
    INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT { dx, dy, mouseData: data, dwFlags: flags, time: 0, dwExtraInfo: RELAY_MAGIC },
        },
    }
}

fn keyboard(vk: u16, scan: u16, flags: KEYBD_EVENT_FLAGS) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT { wVk: VIRTUAL_KEY(vk), wScan: scan, dwFlags: flags, time: 0, dwExtraInfo: RELAY_MAGIC },
        },
    }
}

impl Injector for SendInputInjector {
    fn move_to(&mut self, x: i32, y: i32) -> Result<()> {
        let (vx, vy, vw, vh) = unsafe {
            (
                GetSystemMetrics(SM_XVIRTUALSCREEN),
                GetSystemMetrics(SM_YVIRTUALSCREEN),
                GetSystemMetrics(SM_CXVIRTUALSCREEN),
                GetSystemMetrics(SM_CYVIRTUALSCREEN),
            )
        };
        // Absolute coordinates are 0..=65535 across the whole virtual desktop.
        let norm = |p: i32, origin: i32, size: i32| ((p - origin) as f64 * 65535.0 / (size - 1).max(1) as f64).round() as i32;
        send(&[mouse(
            norm(x, vx, vw),
            norm(y, vy, vh),
            0,
            MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_VIRTUALDESK,
        )])?;
        // Normalized coordinates can round one pixel off at some scalings; correct exactly.
        let mut p = POINT::default();
        unsafe {
            if GetCursorPos(&mut p).is_ok() && (p.x, p.y) != (x, y) {
                let _ = SetCursorPos(x, y);
            }
        }
        Ok(())
    }

    fn button(&mut self, btn: MouseBtn, down: bool) -> Result<()> {
        let (flags, data) = match (btn, down) {
            (MouseBtn::Left, true) => (MOUSEEVENTF_LEFTDOWN, 0),
            (MouseBtn::Left, false) => (MOUSEEVENTF_LEFTUP, 0),
            (MouseBtn::Right, true) => (MOUSEEVENTF_RIGHTDOWN, 0),
            (MouseBtn::Right, false) => (MOUSEEVENTF_RIGHTUP, 0),
            (MouseBtn::Middle, true) => (MOUSEEVENTF_MIDDLEDOWN, 0),
            (MouseBtn::Middle, false) => (MOUSEEVENTF_MIDDLEUP, 0),
            (MouseBtn::X1, true) => (MOUSEEVENTF_XDOWN, 1),
            (MouseBtn::X1, false) => (MOUSEEVENTF_XUP, 1),
            (MouseBtn::X2, true) => (MOUSEEVENTF_XDOWN, 2),
            (MouseBtn::X2, false) => (MOUSEEVENTF_XUP, 2),
        };
        send(&[mouse(0, 0, data, flags)])
    }

    fn wheel(&mut self, delta: i32, horizontal: bool) -> Result<()> {
        let flags = if horizontal { MOUSEEVENTF_HWHEEL } else { MOUSEEVENTF_WHEEL };
        send(&[mouse(0, 0, delta as u32, flags)])
    }

    fn key(&mut self, key: &KeyStroke, down: bool, ch: Option<&str>) -> Result<()> {
        let up = if down { KEYBD_EVENT_FLAGS(0) } else { KEYEVENTF_KEYUP };
        let by_scan = |scan: u16, ext: bool| {
            let ext = if ext { KEYEVENTF_EXTENDEDKEY } else { KEYBD_EVENT_FLAGS(0) };
            send(&[keyboard(0, scan, KEYEVENTF_SCANCODE | ext | up)])
        };
        // A recorded scan code replays the physical key, which games and DirectInput apps expect.
        if key.scan != 0 {
            return by_scan(key.scan, key.ext);
        }
        // Without one (injected input), the virtual key follows the current layout.
        if key.vk != 0 {
            return send(&[keyboard(key.vk, 0, up)]);
        }
        // Hand-written or migrated macros only know the code.
        if let Some((scan, ext)) = keymap::scan_for_code(&key.code) {
            return by_scan(scan, ext);
        }
        // Nothing but the character is known: type it as Unicode.
        match (down, ch) {
            (true, Some(text)) => {
                let inputs: Vec<INPUT> = text
                    .encode_utf16()
                    .flat_map(|u| [keyboard(0, u, KEYEVENTF_UNICODE), keyboard(0, u, KEYEVENTF_UNICODE | KEYEVENTF_KEYUP)])
                    .collect();
                send(&inputs)
            }
            _ => Ok(()),
        }
    }
}
