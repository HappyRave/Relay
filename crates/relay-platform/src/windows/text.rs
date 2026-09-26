use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetKeyState, GetKeyboardLayout, ToUnicodeEx, VK_CAPITAL, VK_CONTROL, VK_LCONTROL, VK_LMENU, VK_LSHIFT, VK_MENU,
    VK_RCONTROL, VK_RMENU, VK_RSHIFT, VK_SHIFT,
};
use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};

use crate::{CharTranslator, HeldKeys};

/// Translates with the foreground window's keyboard layout. It runs on the
/// recorder thread a moment after the key press, so a key typed just before
/// switching windows can be read with the next window's layout; Caps Lock is
/// read the same way.
pub struct ToUnicodeTranslator;

/// Don't change the keyboard state (Windows 10 1607+), so dead keys typed in
/// the target app are not consumed by our translation.
const NO_STATE_CHANGE: u32 = 0x4;

impl CharTranslator for ToUnicodeTranslator {
    fn translate(&mut self, vk: u16, scan: u16, held: &HeldKeys) -> Option<String> {
        let mut state = [0u8; 256];
        for &k in held {
            state[k as usize & 0xFF] = 0x80;
        }
        // Low-level hooks report left/right modifiers; ToUnicode also wants the generic ones.
        let any = |a: u16, b: u16| held.contains(&a) || held.contains(&b);
        if any(VK_LSHIFT.0, VK_RSHIFT.0) {
            state[VK_SHIFT.0 as usize] = 0x80;
        }
        if any(VK_LCONTROL.0, VK_RCONTROL.0) {
            state[VK_CONTROL.0 as usize] = 0x80;
        }
        if any(VK_LMENU.0, VK_RMENU.0) {
            state[VK_MENU.0 as usize] = 0x80;
        }
        unsafe {
            if GetKeyState(VK_CAPITAL.0 as i32) & 1 != 0 {
                state[VK_CAPITAL.0 as usize] |= 1;
            }
            let thread = GetWindowThreadProcessId(GetForegroundWindow(), None);
            let layout = GetKeyboardLayout(thread);
            let mut buf = [0u16; 8];
            let n = ToUnicodeEx(vk as u32, scan as u32, &state, &mut buf, NO_STATE_CHANGE, Some(layout));
            // n < 0 is a dead key (e.g. ^ on AZERTY): it types nothing by itself.
            (n > 0).then(|| String::from_utf16_lossy(&buf[..n as usize]))
        }
    }
}
