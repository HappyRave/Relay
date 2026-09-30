//! Pictures on the clipboard, and Windows' screen snip (the Win+Shift+S
//! overlay), which puts its snip there.

use windows::Win32::Foundation::{HANDLE, HGLOBAL};
use windows::Win32::System::DataExchange::{
    CloseClipboard, GetClipboardData, GetClipboardSequenceNumber, IsClipboardFormatAvailable, OpenClipboard,
    RegisterClipboardFormatW,
};
use windows::Win32::System::Memory::{GlobalLock, GlobalSize, GlobalUnlock};
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
use windows::core::w;

use crate::{ClipImage, Clipboard, PlatformError, Result};

/// The standard device-independent bitmap format.
const CF_DIB: u32 = 8;

pub struct WinClipboard;

impl Clipboard for WinClipboard {
    fn sequence(&self) -> u32 {
        unsafe { GetClipboardSequenceNumber() }
    }

    fn image(&self) -> Option<ClipImage> {
        let png = unsafe { RegisterClipboardFormatW(w!("PNG")) };
        // Another program may have it open for a moment.
        let opened = (0..10).any(|i| {
            if i > 0 {
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            unsafe { OpenClipboard(None) }.is_ok()
        });
        if !opened {
            return None;
        }
        let read = |format: u32| unsafe {
            if IsClipboardFormatAvailable(format).is_err() {
                return None;
            }
            let HANDLE(h) = GetClipboardData(format).ok()?;
            let global = HGLOBAL(h);
            let ptr = GlobalLock(global);
            if ptr.is_null() {
                return None;
            }
            let bytes = std::slice::from_raw_parts(ptr as *const u8, GlobalSize(global)).to_vec();
            let _ = GlobalUnlock(global);
            Some(bytes)
        };
        let image = match (png != 0).then(|| read(png)).flatten() {
            Some(bytes) => Some(ClipImage::Png(bytes)),
            None => read(CF_DIB).map(ClipImage::Dib),
        };
        unsafe {
            let _ = CloseClipboard();
        }
        image
    }

    fn start_snip(&self) -> Result<()> {
        let r = unsafe { ShellExecuteW(None, w!("open"), w!("ms-screenclip:"), None, None, SW_SHOWNORMAL) };
        // Values above 32 mean it started.
        if r.0 as usize > 32 { Ok(()) } else { Err(PlatformError::Os("Windows' snipping tool isn't available".into())) }
    }
}
