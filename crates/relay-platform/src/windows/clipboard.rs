//! Pictures and text on the clipboard, and Windows' screen snip (the
//! Win+Shift+S overlay), which puts its snip there.

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
/// The standard text format: UTF-16, ending with a NUL.
const CF_UNICODETEXT: u32 = 13;

pub struct WinClipboard;

/// Runs `f` with the clipboard open; `None` if it can't be opened.
fn with_open<T>(f: impl FnOnce() -> Option<T>) -> Option<T> {
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
    let out = f();
    unsafe {
        let _ = CloseClipboard();
    }
    out
}

/// The clipboard's data in `format`, while it's open.
fn read(format: u32) -> Option<Vec<u8>> {
    unsafe {
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
    }
}

impl Clipboard for WinClipboard {
    fn sequence(&self) -> u32 {
        unsafe { GetClipboardSequenceNumber() }
    }

    fn image(&self) -> Option<ClipImage> {
        let png = unsafe { RegisterClipboardFormatW(w!("PNG")) };
        with_open(|| match (png != 0).then(|| read(png)).flatten() {
            Some(bytes) => Some(ClipImage::Png(bytes)),
            None => read(CF_DIB).map(ClipImage::Dib),
        })
    }

    fn text(&self) -> Option<String> {
        let bytes = with_open(|| read(CF_UNICODETEXT))?;
        let units: Vec<u16> =
            bytes.as_chunks::<2>().0.iter().map(|&b| u16::from_le_bytes(b)).take_while(|&u| u != 0).collect();
        Some(String::from_utf16_lossy(&units))
    }

    fn start_snip(&self) -> Result<()> {
        let r = unsafe { ShellExecuteW(None, w!("open"), w!("ms-screenclip:"), None, None, SW_SHOWNORMAL) };
        // Values above 32 mean it started.
        if r.0 as usize > 32 { Ok(()) } else { Err(PlatformError::Os("Windows' snipping tool isn't available".into())) }
    }
}
