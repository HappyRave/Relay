//! WH_KEYBOARD_LL / WH_MOUSE_LL hooks on a dedicated thread.
//!
//! Windows calls low-level hooks on the installing thread's message loop and
//! silently removes hooks that are slow, so the callbacks only copy the event,
//! timestamp it, filter it and `try_send` it. No allocation, locks or logging.

use std::cell::RefCell;
use std::sync::Arc;
use std::thread::JoinHandle;

use arc_swap::ArcSwap;
use crossbeam_channel::{Sender, bounded};
use relay_core::model::MouseBtn;
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_CONTROL, VK_END, VK_ESCAPE, VK_MENU};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetForegroundWindow, GetMessageW, HHOOK, KBDLLHOOKSTRUCT, LLKHF_EXTENDED,
    LLKHF_INJECTED, LLKHF_LOWER_IL_INJECTED, LLMHF_INJECTED, LLMHF_LOWER_IL_INJECTED, MSG, MSLLHOOKSTRUCT, PM_NOREMOVE,
    PeekMessageW, PostThreadMessageW, SetWindowsHookExW, TranslateMessage, UnhookWindowsHookEx, WH_KEYBOARD_LL,
    WH_MOUSE_LL, WM_KEYDOWN, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MBUTTONDOWN, WM_MBUTTONUP, WM_MOUSEHWHEEL, WM_MOUSEMOVE,
    WM_MOUSEWHEEL, WM_QUIT, WM_RBUTTONDOWN, WM_RBUTTONUP, WM_SYSKEYDOWN, WM_XBUTTONDOWN, WM_XBUTTONUP,
};

use super::now_ms;
use crate::{HookConfig, HookSession, InputHook, PlatformError, RELAY_MAGIC, RawInput, RawKind, Result};

pub struct LowLevelHook;

struct Ctx {
    tx: Sender<RawInput>,
    cfg: Arc<ArcSwap<HookConfig>>,
    /// Buttons pressed inside Relay's window, whose release must be dropped too.
    pressed_inside: u8,
}

thread_local! {
    static CTX: RefCell<Option<Ctx>> = const { RefCell::new(None) };
}

struct Session {
    thread: Option<JoinHandle<()>>,
    thread_id: u32,
    cfg: Arc<ArcSwap<HookConfig>>,
}

impl InputHook for LowLevelHook {
    fn start(&self, cfg: HookConfig, tx: Sender<RawInput>) -> Result<Box<dyn HookSession>> {
        let cfg = Arc::new(ArcSwap::from_pointee(cfg));
        let (ready_tx, ready_rx) = bounded::<Result<u32>>(1);
        let thread_cfg = cfg.clone();
        let thread = std::thread::Builder::new()
            .name("relay-hook".into())
            .spawn(move || run_hook_thread(thread_cfg, tx, ready_tx))
            .map_err(|e| PlatformError::Os(e.to_string()))?;
        let thread_id = ready_rx.recv().map_err(|e| PlatformError::Os(e.to_string()))??;
        Ok(Box::new(Session { thread: Some(thread), thread_id, cfg }))
    }
}

impl HookSession for Session {
    fn update(&self, cfg: HookConfig) {
        self.cfg.store(Arc::new(cfg));
    }

    fn stop(mut self: Box<Self>) {
        unsafe {
            let _ = PostThreadMessageW(self.thread_id, WM_QUIT, WPARAM(0), LPARAM(0));
        }
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

fn run_hook_thread(cfg: Arc<ArcSwap<HookConfig>>, tx: Sender<RawInput>, ready: Sender<Result<u32>>) {
    unsafe {
        let mut msg = MSG::default();
        // Create this thread's message queue before anyone posts WM_QUIT to it.
        let _ = PeekMessageW(&mut msg, None, 0, 0, PM_NOREMOVE);
        CTX.with(|c| *c.borrow_mut() = Some(Ctx { tx, cfg, pressed_inside: 0 }));

        let module = GetModuleHandleW(None).ok().map(|m| HINSTANCE(m.0));
        let hooks = SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_proc), module, 0)
            .and_then(|k| SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_proc), module, 0).map(|m| (k, m)));
        let (kb, mouse): (HHOOK, HHOOK) = match hooks {
            Ok(h) => h,
            Err(e) => {
                let _ = ready.send(Err(PlatformError::Os(format!("SetWindowsHookEx failed: {e}"))));
                return;
            }
        };
        let _ = ready.send(Ok(GetCurrentThreadId()));

        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        let _ = UnhookWindowsHookEx(kb);
        let _ = UnhookWindowsHookEx(mouse);
        CTX.with(|c| *c.borrow_mut() = None);
    }
}

/// Runs `f` with the hook context; `Some(result)` swallows the event.
fn with_ctx(f: impl FnOnce(&mut Ctx) -> Option<LRESULT>) -> Option<LRESULT> {
    CTX.with(|c| c.borrow_mut().as_mut().and_then(f))
}

/// Whether a key is held right now (before the event being processed).
fn is_down(vk: u16) -> bool {
    unsafe { GetAsyncKeyState(vk as i32) as u16 & 0x8000 != 0 }
}

/// Shift, Ctrl, Alt (generic, left and right) and the Windows keys.
fn is_modifier(vk: u16) -> bool {
    matches!(vk, 0x10..=0x12 | 0xA0..=0xA5 | 0x5B | 0x5C)
}

unsafe extern "system" fn keyboard_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 {
        let info = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) };
        let time = now_ms();
        let swallowed = with_ctx(|ctx| {
            let cfg = ctx.cfg.load();
            let injected = (info.flags & (LLKHF_INJECTED | LLKHF_LOWER_IL_INJECTED)).0 != 0;
            if info.dwExtraInfo == RELAY_MAGIC || (injected && cfg.ignore_injected) {
                return None;
            }
            let vk = info.vkCode as u16;
            let down = matches!(wparam.0 as u32, WM_KEYDOWN | WM_SYSKEYDOWN);
            if cfg.swallow_escape && vk == VK_ESCAPE.0 {
                if down {
                    let _ = ctx.tx.try_send(RawInput { time, kind: RawKind::Escape });
                }
                return Some(LRESULT(1));
            }
            // Ctrl + Alt + End is the kill switch: it must always reach its hotkey,
            // or "stop on key press" would swallow it as an ordinary stop.
            if vk == VK_END.0 && is_down(VK_CONTROL.0) && is_down(VK_MENU.0) {
                return None;
            }
            if !cfg.record {
                // Playback: any key but a modifier or a control hotkey stops it.
                if cfg.stop_on_key && !cfg.drop_vks.contains(&vk) && !is_modifier(vk) {
                    if down {
                        let _ = ctx.tx.try_send(RawInput { time, kind: RawKind::StopKey });
                    }
                    return Some(LRESULT(1));
                }
                return None;
            }
            if cfg.drop_vks.contains(&vk) || unsafe { GetForegroundWindow() } == HWND(cfg.own_window as _) {
                return None;
            }
            let ext = (info.flags & LLKHF_EXTENDED).0 != 0;
            let kind = RawKind::Key { vk, scan: info.scanCode as u16, ext, down };
            let _ = ctx.tx.try_send(RawInput { time, kind });
            None
        });
        if let Some(r) = swallowed {
            return r;
        }
    }
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

unsafe extern "system" fn mouse_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 {
        let info = unsafe { &*(lparam.0 as *const MSLLHOOKSTRUCT) };
        let time = now_ms();
        with_ctx(|ctx| {
            let cfg = ctx.cfg.load();
            let injected = info.flags & (LLMHF_INJECTED | LLMHF_LOWER_IL_INJECTED) != 0;
            if info.dwExtraInfo == RELAY_MAGIC || (injected && cfg.ignore_injected) {
                return None;
            }
            if !cfg.record {
                return None;
            }
            let (x, y) = (info.pt.x, info.pt.y);
            let inside = cfg.own_rect.is_some_and(|r| r.contains(x, y));
            let hi = (info.mouseData >> 16) as u16;
            let xbtn = if hi == 1 { MouseBtn::X1 } else { MouseBtn::X2 };
            let button = |btn: MouseBtn, down: bool| Some(RawKind::Button { x, y, btn, down });
            let kind = match wparam.0 as u32 {
                WM_MOUSEMOVE => Some(RawKind::Move { x, y }),
                WM_LBUTTONDOWN => button(MouseBtn::Left, true),
                WM_LBUTTONUP => button(MouseBtn::Left, false),
                WM_RBUTTONDOWN => button(MouseBtn::Right, true),
                WM_RBUTTONUP => button(MouseBtn::Right, false),
                WM_MBUTTONDOWN => button(MouseBtn::Middle, true),
                WM_MBUTTONUP => button(MouseBtn::Middle, false),
                WM_XBUTTONDOWN => button(xbtn, true),
                WM_XBUTTONUP => button(xbtn, false),
                WM_MOUSEWHEEL => Some(RawKind::Wheel { x, y, delta: hi as i16 as i32, horizontal: false }),
                WM_MOUSEHWHEEL => Some(RawKind::Wheel { x, y, delta: hi as i16 as i32, horizontal: true }),
                _ => None,
            };
            let kind = kind?;
            // Clicks and scrolling on Relay's own window are UI, not macro input;
            // a press that started there drops its release even if it ends outside.
            match &kind {
                RawKind::Button { btn, down: true, .. } if inside => {
                    ctx.pressed_inside |= 1 << (*btn as u8);
                    return None;
                }
                RawKind::Button { btn, down: false, .. } if ctx.pressed_inside & (1 << (*btn as u8)) != 0 => {
                    ctx.pressed_inside &= !(1 << (*btn as u8));
                    return None;
                }
                RawKind::Wheel { .. } if inside => return None,
                _ => {}
            }
            let _ = ctx.tx.try_send(RawInput { time, kind });
            None
        });
    }
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}
