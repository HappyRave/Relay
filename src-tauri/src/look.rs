//! The window's look on Windows: frosted glass (Acrylic) or a solid ground,
//! the theme the frame and the glass follow, and what this Windows offers.

use serde::Serialize;
use tauri::utils::config::WindowEffectsConfig;
use tauri::window::{Effect, EffectsBuilder};
use tauri::{Runtime, WebviewWindow};
use ts_rs::TS;

use crate::settings::{Settings, Theme};

/// What Windows offers for the look; the UI shows the glass setting with it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct SystemLook {
    /// Glass is on by default: Windows 11, where Acrylic moves smoothly.
    pub glass_default: bool,
    /// Windows' *Transparency effects*: off, Relay is solid whatever its setting.
    pub transparency: bool,
}

impl SystemLook {
    #[cfg(windows)]
    pub fn read() -> SystemLook {
        SystemLook { glass_default: windows_build() >= 22000, transparency: transparency_effects() }
    }

    #[cfg(not(windows))]
    pub fn read() -> SystemLook {
        SystemLook { glass_default: false, transparency: false }
    }

    /// Whether the window is frosted with these settings.
    pub fn glass(self, settings: &Settings) -> bool {
        self.transparency && settings.glass.unwrap_or(self.glass_default)
    }
}

/// Frosts the window (or not), and gives the frame the settings' theme.
pub fn apply<R: Runtime>(window: &WebviewWindow<R>, settings: &Settings, look: SystemLook) {
    let effects: Option<WindowEffectsConfig> =
        look.glass(settings).then(|| EffectsBuilder::new().effect(Effect::Acrylic).build());
    let _ = window.set_effects(effects);
    let _ = window.set_theme(match settings.theme {
        Theme::System => None,
        Theme::Light => Some(tauri::Theme::Light),
        Theme::Dark => Some(tauri::Theme::Dark),
    });
}

#[cfg(windows)]
fn windows_build() -> u32 {
    use windows::Win32::System::Registry::{HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ, RegGetValueW};
    use windows::core::w;
    let mut buf = [0u16; 16];
    let mut len = size_of_val(&buf) as u32;
    let read = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            w!(r"SOFTWARE\Microsoft\Windows NT\CurrentVersion"),
            w!("CurrentBuildNumber"),
            RRF_RT_REG_SZ,
            None,
            Some(buf.as_mut_ptr().cast()),
            Some(&mut len),
        )
    };
    if read.is_err() {
        return 0;
    }
    let text = String::from_utf16_lossy(&buf[..(len as usize / 2).saturating_sub(1)]);
    text.trim().parse().unwrap_or(0)
}

#[cfg(windows)]
fn transparency_effects() -> bool {
    use windows::Win32::System::Registry::{HKEY_CURRENT_USER, RRF_RT_REG_DWORD, RegGetValueW};
    use windows::core::w;
    let mut value = 1u32;
    let mut len = size_of_val(&value) as u32;
    let _ = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            w!(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize"),
            w!("EnableTransparency"),
            RRF_RT_REG_DWORD,
            None,
            Some((&mut value as *mut u32).cast()),
            Some(&mut len),
        )
    };
    value != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glass_follows_the_setting_then_windows() {
        let win11 = SystemLook { glass_default: true, transparency: true };
        let win10 = SystemLook { glass_default: false, transparency: true };
        let solid = SystemLook { glass_default: true, transparency: false };
        let with = |glass| Settings { glass, ..Settings::default() };
        assert!(win11.glass(&with(None)) && !win10.glass(&with(None)));
        assert!(win10.glass(&with(Some(true))) && !win11.glass(&with(Some(false))));
        assert!(!solid.glass(&with(Some(true))), "Windows' transparency effects off: always solid");
    }

    #[cfg(windows)]
    #[test]
    fn reads_this_windows() {
        assert!(windows_build() >= 10240, "Windows 10 or later");
    }
}
