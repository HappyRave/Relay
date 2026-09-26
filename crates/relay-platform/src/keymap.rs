//! PC scan codes (set 1, as reported by Windows) → W3C `code` names. Scan
//! codes identify the physical key, so the code is the same on every layout.

pub const VK_END: u16 = 0x23;
/// A key event carrying a UTF-16 unit instead of a key (SendInput Unicode).
pub const VK_PACKET: u16 = 0xE7;
pub const VK_PAUSE: u16 = 0x13;

/// Shift, Ctrl, Alt (generic, left and right) and the Windows keys.
pub fn is_modifier_vk(vk: u16) -> bool {
    matches!(vk, 0x10..=0x12 | 0xA0..=0xA5 | 0x5B | 0x5C)
}

pub fn is_ctrl_vk(vk: u16) -> bool {
    matches!(vk, 0x11 | 0xA2 | 0xA3)
}

pub fn is_alt_vk(vk: u16) -> bool {
    matches!(vk, 0x12 | 0xA4 | 0xA5)
}

/// The W3C code for a scan code, falling back to the virtual key.
pub fn code(scan: u16, ext: bool, vk: u16) -> String {
    scan_code(scan, ext).map(str::to_string).unwrap_or_else(|| vk_code(vk))
}

fn scan_code(scan: u16, ext: bool) -> Option<&'static str> {
    const LETTERS_Q: [&str; 10] = ["KeyQ", "KeyW", "KeyE", "KeyR", "KeyT", "KeyY", "KeyU", "KeyI", "KeyO", "KeyP"];
    const LETTERS_A: [&str; 9] = ["KeyA", "KeyS", "KeyD", "KeyF", "KeyG", "KeyH", "KeyJ", "KeyK", "KeyL"];
    const LETTERS_Z: [&str; 7] = ["KeyZ", "KeyX", "KeyC", "KeyV", "KeyB", "KeyN", "KeyM"];
    const DIGITS: [&str; 10] =
        ["Digit1", "Digit2", "Digit3", "Digit4", "Digit5", "Digit6", "Digit7", "Digit8", "Digit9", "Digit0"];
    const FKEYS: [&str; 10] = ["F1", "F2", "F3", "F4", "F5", "F6", "F7", "F8", "F9", "F10"];
    if ext {
        return Some(match scan {
            0x1C => "NumpadEnter",
            0x1D => "ControlRight",
            0x45 => "NumLock",
            0x35 => "NumpadDivide",
            0x37 => "PrintScreen",
            0x38 => "AltRight",
            0x47 => "Home",
            0x48 => "ArrowUp",
            0x49 => "PageUp",
            0x4B => "ArrowLeft",
            0x4D => "ArrowRight",
            0x4F => "End",
            0x50 => "ArrowDown",
            0x51 => "PageDown",
            0x52 => "Insert",
            0x53 => "Delete",
            0x5B => "MetaLeft",
            0x5C => "MetaRight",
            0x5D => "ContextMenu",
            _ => return None,
        });
    }
    Some(match scan {
        0x01 => "Escape",
        0x02..=0x0B => DIGITS[(scan - 0x02) as usize],
        0x0C => "Minus",
        0x0D => "Equal",
        0x0E => "Backspace",
        0x0F => "Tab",
        0x10..=0x19 => LETTERS_Q[(scan - 0x10) as usize],
        0x1A => "BracketLeft",
        0x1B => "BracketRight",
        0x1C => "Enter",
        0x1D => "ControlLeft",
        0x1E..=0x26 => LETTERS_A[(scan - 0x1E) as usize],
        0x27 => "Semicolon",
        0x28 => "Quote",
        0x29 => "Backquote",
        0x2A => "ShiftLeft",
        0x2B => "Backslash",
        0x2C..=0x32 => LETTERS_Z[(scan - 0x2C) as usize],
        0x33 => "Comma",
        0x34 => "Period",
        0x35 => "Slash",
        0x36 => "ShiftRight",
        0x37 => "NumpadMultiply",
        0x38 => "AltLeft",
        0x39 => "Space",
        0x3A => "CapsLock",
        0x3B..=0x44 => FKEYS[(scan - 0x3B) as usize],
        // Windows reports Pause as 0x45 and NumLock as extended 0x45.
        0x45 => "Pause",
        0x46 => "ScrollLock",
        0x47 => "Numpad7",
        0x48 => "Numpad8",
        0x49 => "Numpad9",
        0x4A => "NumpadSubtract",
        0x4B => "Numpad4",
        0x4C => "Numpad5",
        0x4D => "Numpad6",
        0x4E => "NumpadAdd",
        0x4F => "Numpad1",
        0x50 => "Numpad2",
        0x51 => "Numpad3",
        0x52 => "Numpad0",
        0x53 => "NumpadDecimal",
        0x56 => "IntlBackslash",
        0x57 => "F11",
        0x58 => "F12",
        _ => return None,
    })
}

/// The scan code (and extended flag) of a W3C code, for replaying keys
/// that were stored without one (migrated or hand-written macros).
pub fn scan_for_code(code: &str) -> Option<(u16, bool)> {
    [false, true].into_iter().find_map(|ext| (1..0x60u16).find(|&s| scan_code(s, ext) == Some(code)).map(|s| (s, ext)))
}

/// Codes for keys without a known scan code (media keys, injected input).
fn vk_code(vk: u16) -> String {
    match vk {
        0x41..=0x5A => format!("Key{}", vk as u8 as char),
        0x30..=0x39 => format!("Digit{}", vk as u8 as char),
        0x70..=0x87 => format!("F{}", vk - 0x6F),
        0x08 => "Backspace".into(),
        0x13 => "Pause".into(),
        0x90 => "NumLock".into(),
        0x91 => "ScrollLock".into(),
        0x09 => "Tab".into(),
        0x0D => "Enter".into(),
        0x1B => "Escape".into(),
        0x20 => "Space".into(),
        0x25 => "ArrowLeft".into(),
        0x26 => "ArrowUp".into(),
        0x27 => "ArrowRight".into(),
        0x28 => "ArrowDown".into(),
        0x21 => "PageUp".into(),
        0x22 => "PageDown".into(),
        0x23 => "End".into(),
        0x24 => "Home".into(),
        0x2C => "PrintScreen".into(),
        0x2D => "Insert".into(),
        0x2E => "Delete".into(),
        0x5D => "ContextMenu".into(),
        0x60..=0x69 => format!("Numpad{}", vk - 0x60),
        0xA0 => "ShiftLeft".into(),
        0xA1 => "ShiftRight".into(),
        0xA2 => "ControlLeft".into(),
        0xA3 => "ControlRight".into(),
        0xA4 => "AltLeft".into(),
        0xA5 => "AltRight".into(),
        0x5B => "MetaLeft".into(),
        0x5C => "MetaRight".into(),
        other => format!("Vk{other:02X}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn physical_keys() {
        assert_eq!(code(0x1E, false, 0x41), "KeyA");
        assert_eq!(code(0x10, false, 0x41), "KeyQ"); // the A key on AZERTY
        assert_eq!(code(0x1D, true, 0xA3), "ControlRight");
        assert_eq!(code(0x4B, true, 0x25), "ArrowLeft");
        assert_eq!(code(0x4B, false, 0x64), "Numpad4");
        assert_eq!(code(0x43, false, 0x78), "F9");
        assert_eq!(code(0x5B, true, 0x5B), "MetaLeft");
        assert_eq!(code(0x45, true, 0x90), "NumLock");
        assert_eq!(code(0x45, false, 0x13), "Pause");
    }

    #[test]
    fn codes_map_back_to_scan_codes() {
        assert_eq!(scan_for_code("KeyA"), Some((0x1E, false)));
        assert_eq!(scan_for_code("ArrowLeft"), Some((0x4B, true)));
        assert_eq!(scan_for_code("ControlRight"), Some((0x1D, true)));
        assert_eq!(scan_for_code("NumLock"), Some((0x45, true)));
        assert_eq!(scan_for_code("Unidentified"), None);
    }

    #[test]
    fn virtual_key_fallback() {
        assert_eq!(code(0, false, 0x5A), "KeyZ");
        assert_eq!(code(0, false, 0x7B), "F12");
        assert_eq!(code(0, false, 0x23), "End");
        assert_eq!(code(0, false, 0x63), "Numpad3");
        assert_eq!(code(0, false, 0xB3), "VkB3");
    }

    #[test]
    fn every_scan_code_round_trips() {
        let mut seen = std::collections::HashSet::new();
        for ext in [false, true] {
            for scan in 1..0x60u16 {
                let Some(c) = scan_code(scan, ext) else { continue };
                assert!(seen.insert(c), "{c} is mapped twice");
                assert_eq!(scan_for_code(c), Some((scan, ext)), "{c}");
            }
        }
        assert!(seen.len() > 100, "{}", seen.len());
    }

    #[test]
    fn all_letters_digits_and_function_keys_have_scan_codes() {
        for c in 'A'..='Z' {
            assert!(scan_for_code(&format!("Key{c}")).is_some(), "Key{c}");
        }
        for d in 0..=9 {
            assert!(scan_for_code(&format!("Digit{d}")).is_some(), "Digit{d}");
            assert!(scan_for_code(&format!("Numpad{d}")).is_some(), "Numpad{d}");
        }
        for f in 1..=12 {
            assert!(scan_for_code(&format!("F{f}")).is_some(), "F{f}");
        }
    }

    #[test]
    fn unknown_scan_codes_fall_back_to_the_virtual_key() {
        // An extended scan code with no mapping, e.g. a media key.
        assert_eq!(code(0x22, true, 0xB3), "VkB3");
        assert_eq!(code(0x7F, false, 0x41), "KeyA");
    }

    #[test]
    fn virtual_keys_cover_navigation_and_modifiers() {
        let cases = [
            (0x08, "Backspace"),
            (0x09, "Tab"),
            (0x0D, "Enter"),
            (0x13, "Pause"),
            (0x1B, "Escape"),
            (0x20, "Space"),
            (0x21, "PageUp"),
            (0x22, "PageDown"),
            (0x24, "Home"),
            (0x25, "ArrowLeft"),
            (0x26, "ArrowUp"),
            (0x27, "ArrowRight"),
            (0x28, "ArrowDown"),
            (0x2C, "PrintScreen"),
            (0x2D, "Insert"),
            (0x2E, "Delete"),
            (0x30, "Digit0"),
            (0x39, "Digit9"),
            (0x41, "KeyA"),
            (0x5B, "MetaLeft"),
            (0x5C, "MetaRight"),
            (0x5D, "ContextMenu"),
            (0x60, "Numpad0"),
            (0x69, "Numpad9"),
            (0x70, "F1"),
            (0x87, "F24"),
            (0x90, "NumLock"),
            (0x91, "ScrollLock"),
            (0xA0, "ShiftLeft"),
            (0xA1, "ShiftRight"),
            (0xA2, "ControlLeft"),
            (0xA3, "ControlRight"),
            (0xA4, "AltLeft"),
            (0xA5, "AltRight"),
            (0x07, "Vk07"),
        ];
        for (vk, want) in cases {
            assert_eq!(vk_code(vk), want, "{vk:#x}");
        }
    }

    #[test]
    fn modifier_classes() {
        for vk in [0x10, 0x11, 0x12, 0xA0, 0xA1, 0xA2, 0xA3, 0xA4, 0xA5, 0x5B, 0x5C] {
            assert!(is_modifier_vk(vk), "{vk:#x}");
        }
        for vk in [0x41, 0x14, 0x5D, 0x0D] {
            assert!(!is_modifier_vk(vk), "{vk:#x} (CapsLock, the menu key and Enter aren't)");
        }
        assert!(is_ctrl_vk(0x11) && is_ctrl_vk(0xA2) && is_ctrl_vk(0xA3) && !is_ctrl_vk(0x12));
        assert!(is_alt_vk(0x12) && is_alt_vk(0xA4) && is_alt_vk(0xA5) && !is_alt_vk(0x11));
    }
}
