//! WH_KEYBOARD_LL / WH_MOUSE_LL hooks on a dedicated thread.
//!
//! Windows calls low-level hooks on the installing thread's message loop and
//! silently removes hooks that are slow, so the callbacks only copy the event,
//! timestamp it, filter it and `try_send` it: no allocation, no blocking, no
//! logging.
//!
//! A release always goes the way its press went: a key whose press was
//! recorded has its release recorded, one whose press was left out (typed
//! into Relay) or swallowed (Esc, a stop key) has its release left out or
//! swallowed too. Otherwise a macro could hold a key forever, or the target
//! app could see a release without its press.

use std::cell::RefCell;
use std::thread::JoinHandle;

use crossbeam_channel::{Sender, bounded};
use relay_core::model::MouseBtn;
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_CONTROL, VK_END, VK_ESCAPE, VK_MENU};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GA_ROOT, GetAncestor, GetForegroundWindow, GetMessageW, HHOOK, KBDLLHOOKSTRUCT,
    LLKHF_EXTENDED, LLKHF_INJECTED, LLKHF_LOWER_IL_INJECTED, LLMHF_INJECTED, LLMHF_LOWER_IL_INJECTED, MSG,
    MSLLHOOKSTRUCT, PM_NOREMOVE, PeekMessageW, PostThreadMessageW, SetWindowsHookExW, TranslateMessage,
    UnhookWindowsHookEx, WH_KEYBOARD_LL, WH_MOUSE_LL, WM_KEYDOWN, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MBUTTONDOWN,
    WM_MBUTTONUP, WM_MOUSEHWHEEL, WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_QUIT, WM_RBUTTONDOWN, WM_RBUTTONUP, WM_SYSKEYDOWN,
    WM_XBUTTONDOWN, WM_XBUTTONUP, WindowFromPoint,
};

use super::now_ms;
use crate::keymap::is_modifier_vk;
use crate::{HookConfig, HookMode, HookSession, InputHook, PlatformError, RELAY_MAGIC, RawInput, RawKind, Result};

pub struct LowLevelHook;

/// A set of virtual keys (0..=255).
#[derive(Default)]
struct VkSet([u64; 4]);

impl VkSet {
    fn has(&self, vk: u16) -> bool {
        self.0[(vk as usize >> 6) & 3] & (1 << (vk & 63)) != 0
    }
    /// Adds or removes `vk`; returns whether it was there.
    fn set(&mut self, vk: u16, on: bool) -> bool {
        let (word, bit) = ((vk as usize >> 6) & 3, 1u64 << (vk & 63));
        let was = self.0[word] & bit != 0;
        if on {
            self.0[word] |= bit;
        } else {
            self.0[word] &= !bit;
        }
        was
    }
}

struct Ctx {
    tx: Sender<RawInput>,
    cfg: HookConfig,
    /// Keys held whose press was recorded,
    reported: VkSet,
    /// passed through but not recorded (typed into Relay, a control hotkey),
    skipped: VkSet,
    /// or swallowed (Esc, a stop key).
    swallowed: VkSet,
    /// Buttons pressed on Relay's window, whose release is left out too.
    pressed_inside: u8,
}

thread_local! {
    static CTX: RefCell<Option<Ctx>> = const { RefCell::new(None) };
}

/// A running hook thread. Dropping it removes the hooks.
struct Session {
    thread: Option<JoinHandle<()>>,
    thread_id: u32,
}

impl InputHook for LowLevelHook {
    fn start(&self, cfg: HookConfig, tx: Sender<RawInput>) -> Result<Box<dyn HookSession>> {
        let (ready_tx, ready_rx) = bounded::<Result<u32>>(1);
        let thread = std::thread::Builder::new()
            .name("relay-hook".into())
            .spawn(move || run_hook_thread(cfg, tx, ready_tx))
            .map_err(|e| PlatformError::Os(e.to_string()))?;
        match ready_rx.recv() {
            Ok(Ok(thread_id)) => Ok(Box::new(Session { thread: Some(thread), thread_id })),
            Ok(Err(e)) => {
                let _ = thread.join();
                Err(e)
            }
            Err(e) => Err(PlatformError::Os(e.to_string())),
        }
    }
}

impl HookSession for Session {}

impl Drop for Session {
    fn drop(&mut self) {
        let posted = unsafe { PostThreadMessageW(self.thread_id, WM_QUIT, WPARAM(0), LPARAM(0)) }.is_ok();
        // If the message couldn't be posted the thread is already gone (or
        // never had a queue); joining could then wait forever.
        if let Some(t) = self.thread.take()
            && posted
        {
            let _ = t.join();
        }
    }
}

fn run_hook_thread(cfg: HookConfig, tx: Sender<RawInput>, ready: Sender<Result<u32>>) {
    unsafe {
        let mut msg = MSG::default();
        // Create this thread's message queue before anyone posts WM_QUIT to it.
        let _ = PeekMessageW(&mut msg, None, 0, 0, PM_NOREMOVE);
        CTX.with(|c| {
            *c.borrow_mut() =
                Some(Ctx { tx, cfg, reported: VkSet::default(), skipped: VkSet::default(), swallowed: VkSet::default(), pressed_inside: 0 })
        });
        let module = GetModuleHandleW(None).ok().map(|m| HINSTANCE(m.0));
        let installed = SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_proc), module, 0).and_then(|kb| {
            match SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_proc), module, 0) {
                Ok(mouse) => Ok((kb, mouse)),
                Err(e) => {
                    let _ = UnhookWindowsHookEx(kb);
                    Err(e)
                }
            }
        });
        let (kb, mouse): (HHOOK, HHOOK) = match installed {
            Ok(h) => h,
            Err(e) => {
                CTX.with(|c| *c.borrow_mut() = None);
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

/// Whether the top-level window under a screen point is `own`.
fn is_own_window_at(own: isize, x: i32, y: i32) -> bool {
    own != 0 && unsafe { GetAncestor(WindowFromPoint(POINT { x, y }), GA_ROOT) } == HWND(own as _)
}

const SWALLOW: Option<LRESULT> = Some(LRESULT(1));

unsafe extern "system" fn keyboard_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 {
        let info = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) };
        let time = now_ms();
        let swallowed = with_ctx(|ctx| {
            let injected = (info.flags & (LLKHF_INJECTED | LLKHF_LOWER_IL_INJECTED)).0 != 0;
            if info.dwExtraInfo == RELAY_MAGIC || (injected && ctx.cfg.ignore_injected) {
                return None;
            }
            let vk = info.vkCode as u16;
            let down = matches!(wparam.0 as u32, WM_KEYDOWN | WM_SYSKEYDOWN);
            // Ctrl + Alt + End is the kill switch: it must always reach its hotkey.
            if vk == VK_END.0 && is_down(VK_CONTROL.0) && is_down(VK_MENU.0) {
                return None;
            }
            let report = |ctx: &Ctx, kind: RawKind| {
                let _ = ctx.tx.try_send(RawInput { time, kind });
            };
            // A release goes the way its press went; one whose press came
            // before the hook started is let through unrecorded.
            if !down {
                if ctx.swallowed.set(vk, false) {
                    return SWALLOW;
                }
                if ctx.reported.set(vk, false) {
                    let ext = (info.flags & LLKHF_EXTENDED).0 != 0;
                    report(ctx, RawKind::Key { vk, scan: info.scanCode as u16, ext, down: false });
                }
                ctx.skipped.set(vk, false);
                return None;
            }
            // Auto-repeat of a key already decided on goes the same way.
            if ctx.swallowed.has(vk) {
                return SWALLOW;
            }
            if ctx.skipped.has(vk) {
                return None;
            }
            match &ctx.cfg.mode {
                HookMode::Record { own_window, skip_vks, esc_stops } => {
                    if !ctx.reported.has(vk) {
                        if *esc_stops && vk == VK_ESCAPE.0 {
                            ctx.swallowed.set(vk, true);
                            report(ctx, RawKind::Escape);
                            return SWALLOW;
                        }
                        let into_relay = *own_window != 0 && unsafe { GetForegroundWindow() } == HWND(*own_window as _);
                        if skip_vks.contains(&vk) || into_relay {
                            ctx.skipped.set(vk, true);
                            return None;
                        }
                        ctx.reported.set(vk, true);
                    }
                    let ext = (info.flags & LLKHF_EXTENDED).0 != 0;
                    report(ctx, RawKind::Key { vk, scan: info.scanCode as u16, ext, down: true });
                    None
                }
                HookMode::Watch { stop_on_key, pass_vks } => {
                    let kind = if vk == VK_ESCAPE.0 {
                        RawKind::Escape
                    } else if *stop_on_key && !pass_vks.contains(&vk) && !is_modifier_vk(vk) {
                        RawKind::StopKey
                    } else {
                        return None;
                    };
                    ctx.swallowed.set(vk, true);
                    report(ctx, kind);
                    SWALLOW
                }
            }
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
            let HookMode::Record { own_window, .. } = ctx.cfg.mode else { return None };
            let injected = info.flags & (LLMHF_INJECTED | LLMHF_LOWER_IL_INJECTED) != 0;
            if info.dwExtraInfo == RELAY_MAGIC || (injected && ctx.cfg.ignore_injected) {
                return None;
            }
            let (x, y) = (info.pt.x, info.pt.y);
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
            // a press that started there drops its release even if it ends
            // outside. (Only presses and wheel events look up the window under
            // the cursor, not the frequent moves.)
            match &kind {
                RawKind::Button { btn, down: true, .. } if is_own_window_at(own_window, x, y) => {
                    ctx.pressed_inside |= 1 << (*btn as u8);
                    return None;
                }
                RawKind::Button { btn, down: false, .. } if ctx.pressed_inside & (1 << (*btn as u8)) != 0 => {
                    ctx.pressed_inside &= !(1 << (*btn as u8));
                    return None;
                }
                RawKind::Wheel { .. } if is_own_window_at(own_window, x, y) => return None,
                _ => {}
            }
            let _ = ctx.tx.try_send(RawInput { time, kind });
            None
        });
    }
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}
