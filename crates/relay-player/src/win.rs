//! The player's Win32 shell: reads the macro appended to this program, shows
//! a small status window that never takes focus, counts down, plays, and
//! exits with a code that says how it ended.

use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use relay_core::data::DataTable;
use relay_core::format;
use relay_core::model::{Macro, Ms, Rect};
use relay_core::session::FinishReason;
use relay_platform::{HookConfig, HookMode, HookSession, Platform, RawKind};
use relay_playback::{EngineHandle, PlayPlan, PlaybackSink, Tick, plan};
use relay_player::args::{self, Cli, Parsed, USAGE};
use relay_player::exit::Exit;
use relay_player::place::{Layout, busy_points, choose_place, layout};
use relay_player::status::{Phase, Status};
use windows::Win32::Foundation::{COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, BitBlt, CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS, CreateCompatibleBitmap, CreateCompatibleDC,
    CreateFontW, CreateSolidBrush, DEFAULT_CHARSET, DT_END_ELLIPSIS, DT_NOPREFIX, DT_SINGLELINE, DT_VCENTER,
    DT_WORDBREAK, DeleteDC, DeleteObject, DrawTextW, EndPaint, FF_DONTCARE, FW_NORMAL, FW_SEMIBOLD, FillRect, HDC,
    HFONT, InvalidateRect, OUT_DEFAULT_PRECIS, PAINTSTRUCT, SRCCOPY, ScreenToClient, SelectObject, SetBkMode,
    SetTextColor, TRANSPARENT,
};
use windows::Win32::System::Console::{ATTACH_PARENT_PROCESS, AttachConsole, GetStdHandle, STD_ERROR_HANDLE};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    MOD_ALT, MOD_CONTROL, MOD_NOREPEAT, RegisterHotKey, TME_LEAVE, TRACKMOUSEEVENT, TrackMouseEvent, UnregisterHotKey,
    VK_END,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW, HTCAPTION, HTCLIENT, HWND_MESSAGE,
    IDC_ARROW, KillTimer, LWA_ALPHA, LoadCursorW, MA_NOACTIVATE, MB_ICONWARNING, MB_OK, MSG, MessageBoxW, PostMessageW,
    PostQuitMessage, RegisterClassW, SW_SHOWNOACTIVATE, SWP_NOACTIVATE, SWP_NOZORDER, SetLayeredWindowAttributes,
    SetTimer, SetWindowPos, ShowWindow, TranslateMessage, WM_APP, WM_DESTROY, WM_DPICHANGED, WM_HOTKEY, WM_LBUTTONUP,
    WM_MOUSEACTIVATE, WM_MOUSEMOVE, WM_NCHITTEST, WM_PAINT, WM_TIMER, WNDCLASSW, WS_EX_LAYERED, WS_EX_NOACTIVATE,
    WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP,
};
use windows::core::{HSTRING, w};

/// Sent once the mouse leaves, after `TrackMouseEvent` (in `Win32::UI::Controls`).
const WM_MOUSELEAVE: u32 = 0x02A3;
const WM_APP_TICK: u32 = WM_APP + 1;
const WM_APP_NOTICE: u32 = WM_APP + 2;
const WM_APP_DONE: u32 = WM_APP + 3;
/// `wparam`: the [`FinishReason`] it asks for, as [`reason_code`] gives it.
const WM_APP_STOP: u32 = WM_APP + 4;
const TIMER_COUNTDOWN: usize = 1;
const TIMER_CLOSE: usize = 2;
const HOTKEY_KILL: i32 = 1;
/// How long "Done" (or "Stopped") shows before the window closes.
const CLOSE_AFTER_MS: u32 = 1500;
const MARGIN: i32 = 16;

// Relay's colors (src/styles/tokens.css), as 0x00BBGGRR.
const BG: COLORREF = COLORREF(0x00F2_F2F3);
const INK: COLORREF = COLORREF(0x001D_1E20);
const MUTED: COLORREF = COLORREF(0x005D_5D60);
const TRACK: COLORREF = COLORREF(0x00E7_E7EA);
const ACCENT: COLORREF = COLORREF(0x0013_30EC);
const WHITE: COLORREF = COLORREF(0x00FF_FFFF);

/// Everything the window works with, on its thread.
struct App {
    platform: Platform,
    hwnd: HWND,
    quiet: bool,
    /// The macro, until playback takes it.
    pending: Option<(Macro, Option<DataTable>)>,
    status: Status,
    layout: Layout,
    fonts: Fonts,
    hover: bool,
    tracking: bool,
    countdown_from: f64,
    hook: Option<Box<dyn HookSession>>,
    engine: Option<EngineHandle>,
    shared: Arc<Shared>,
    exit: Option<Exit>,
}

/// What the engine thread hands to the window thread.
#[derive(Default)]
struct Shared {
    tick: Mutex<Option<Tick>>,
    /// A tick message is on its way: later ticks only update `tick`.
    tick_posted: AtomicBool,
    notices: Mutex<Vec<String>>,
    done: Mutex<Option<FinishReason>>,
}

struct WinSink {
    hwnd: isize,
    shared: Arc<Shared>,
}

impl WinSink {
    fn post(&self, msg: u32) {
        let _ = unsafe { PostMessageW(Some(HWND(self.hwnd as _)), msg, WPARAM(0), LPARAM(0)) };
    }
}

impl PlaybackSink for WinSink {
    fn tick(&self, tick: Tick) {
        *self.shared.tick.lock().unwrap() = Some(tick);
        if !self.shared.tick_posted.swap(true, Ordering::AcqRel) {
            self.post(WM_APP_TICK);
        }
    }

    fn notice(&self, message: String) {
        self.shared.notices.lock().unwrap().push(message);
        self.post(WM_APP_NOTICE);
    }

    fn done(&self, reason: FinishReason, _: Option<Ms>) {
        *self.shared.done.lock().unwrap() = Some(reason);
        self.post(WM_APP_DONE);
    }
}

struct Fonts {
    name: HFONT,
    text: HFONT,
}

impl Fonts {
    fn new(l: &Layout) -> Fonts {
        let font = |px: i32, weight: u32| unsafe {
            CreateFontW(
                -px,
                0,
                0,
                0,
                weight as i32,
                0,
                0,
                0,
                DEFAULT_CHARSET,
                OUT_DEFAULT_PRECIS,
                CLIP_DEFAULT_PRECIS,
                CLEARTYPE_QUALITY,
                FF_DONTCARE.0 as u32,
                w!("Segoe UI"),
            )
        };
        Fonts { name: font(l.name_px, FW_SEMIBOLD.0), text: font(l.text_px, FW_NORMAL.0) }
    }
}

impl Drop for Fonts {
    fn drop(&mut self) {
        unsafe {
            let _ = DeleteObject(self.name.into());
            let _ = DeleteObject(self.text.into());
        }
    }
}

thread_local! {
    static APP: RefCell<Option<App>> = const { RefCell::new(None) };
}

/// Runs `f` on the app, unless it's busy (a message sent while handling
/// another) or gone.
fn with_app<R>(f: impl FnOnce(&mut App) -> R) -> Option<R> {
    APP.with(|a| a.try_borrow_mut().ok().and_then(|mut a| a.as_mut().map(f)))
}

pub fn main() -> i32 {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let quiet = args.iter().any(|a| a == "--quiet");
    let (m, data) = match load() {
        Ok(loaded) => loaded,
        Err(e) => {
            tell(&format!("This program's macro can't be read: {e}"), quiet);
            return Exit::Error.code();
        }
    };
    match args::parse(args, &m.playback) {
        Ok(Parsed::Run(cli)) => run(m, data, cli),
        Ok(Parsed::Help) => {
            tell(USAGE, false);
            Exit::Completed.code()
        }
        Err(e) => {
            tell(&format!("{e}\n\n{USAGE}"), quiet);
            Exit::BadArgs.code()
        }
    }
}

fn load() -> Result<(Macro, Option<DataTable>), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let bytes = std::fs::read(exe).map_err(|e| e.to_string())?;
    format::from_program(&bytes).map_err(|e| e.to_string())
}

/// Writes `text` to the console this was started from, or shows it in a
/// message box (unless `quiet`) when there's none.
fn tell(text: &str, quiet: bool) {
    let console = unsafe { GetStdHandle(STD_ERROR_HANDLE) }.is_ok_and(|h| !h.is_invalid() && !h.0.is_null())
        || unsafe { AttachConsole(ATTACH_PARENT_PROCESS) }.is_ok();
    if console {
        eprintln!("{text}");
    } else if !quiet {
        unsafe { MessageBoxW(None, &HSTRING::from(text), w!("Relay player"), MB_OK | MB_ICONWARNING) };
    }
}

fn run(mut m: Macro, data: Option<DataTable>, cli: Cli) -> i32 {
    m.playback = cli.apply(&m.playback);
    let platform = relay_platform::platform();
    let monitors = platform.screen.monitors();
    let primary = monitors.iter().find(|m| m.primary).or(monitors.first());
    let (work, dpi) = primary.map_or((platform.screen.virtual_desktop(), 96), |p| (p.work, p.dpi));
    let l = layout(dpi);
    let (offset, _) = plan::window_offset(&m, |exe, class| platform.windows.find_window(exe, class));
    let (at, click_through) = choose_place(work, l.w, l.h, MARGIN, &busy_points(&m, offset));
    let Some(hwnd) = create_window(&m.name, at, click_through, cli.quiet) else {
        tell("Couldn't open the player's window.", cli.quiet);
        return Exit::Error.code();
    };
    let status = Status::new(m.name.clone(), relay_core::timeline::duration(&m.events), cli.countdown);
    let app = App {
        countdown_from: (platform.now_ms)(),
        platform,
        hwnd,
        quiet: cli.quiet,
        pending: Some((m, data)),
        status,
        fonts: Fonts::new(&l),
        layout: l,
        hover: false,
        tracking: false,
        hook: None,
        engine: None,
        shared: Arc::default(),
        exit: None,
    };
    APP.with(|a| *a.borrow_mut() = Some(app));
    unsafe {
        let _ = RegisterHotKey(Some(hwnd), HOTKEY_KILL, MOD_CONTROL | MOD_ALT | MOD_NOREPEAT, VK_END.0 as u32);
    }
    with_app(|a| {
        a.watch(false);
        if cli.countdown {
            unsafe { SetTimer(Some(hwnd), TIMER_COUNTDOWN, 100, None) };
        } else {
            a.play();
        }
    });
    unsafe {
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        let _ = UnregisterHotKey(Some(hwnd), HOTKEY_KILL);
    }
    // Dropped here, not at thread exit: stopping the engine releases held input.
    let app = APP.with(|a| a.borrow_mut().take());
    app.and_then(|a| a.exit).unwrap_or(Exit::Error).code()
}

fn create_window(title: &str, at: Rect, click_through: bool, quiet: bool) -> Option<HWND> {
    unsafe {
        let instance = HINSTANCE(GetModuleHandleW(None).ok()?.0);
        let class = WNDCLASSW {
            lpfnWndProc: Some(proc),
            hInstance: instance,
            hCursor: LoadCursorW(None, IDC_ARROW).ok()?,
            lpszClassName: w!("RelayPlayer"),
            ..Default::default()
        };
        RegisterClassW(&class);
        let mut ex = WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE;
        if click_through {
            ex |= WS_EX_LAYERED | WS_EX_TRANSPARENT;
        }
        let parent = quiet.then_some(HWND_MESSAGE);
        let hwnd = CreateWindowExW(
            ex,
            w!("RelayPlayer"),
            &HSTRING::from(title),
            WS_POPUP,
            at.x,
            at.y,
            at.w,
            at.h,
            parent,
            None,
            Some(instance),
            None,
        )
        .ok()?;
        if click_through {
            let _ = SetLayeredWindowAttributes(hwnd, COLORREF(0), 255, LWA_ALPHA);
        }
        if !quiet {
            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        }
        Some(hwnd)
    }
}

fn reason_code(r: FinishReason) -> usize {
    match r {
        FinishReason::KeyPressed => 1,
        FinishReason::Killed => 2,
        _ => 0,
    }
}

fn reason_of(code: usize) -> FinishReason {
    match code {
        1 => FinishReason::KeyPressed,
        2 => FinishReason::Killed,
        _ => FinishReason::Stopped,
    }
}

impl App {
    fn redraw(&self) {
        let _ = unsafe { InvalidateRect(Some(self.hwnd), None, false) };
    }

    /// (Re)starts the hook that watches for Esc, the kill switch and, while
    /// playing with "Stop on key press", any key.
    fn watch(&mut self, stop_on_key: bool) {
        self.hook = None;
        let cfg = HookConfig {
            mode: HookMode::Watch { stop_on_key, pass_vks: Vec::new(), report_kill_switch: true },
            ignore_injected: true,
            mouse_pulse: None,
        };
        let (tx, rx) = crossbeam_channel::bounded(64);
        match self.platform.hook.start(cfg, tx) {
            Ok(hook) => {
                let hwnd = self.hwnd.0 as isize;
                std::thread::spawn(move || {
                    for raw in rx {
                        let reason = match raw.kind {
                            RawKind::Escape => FinishReason::Stopped,
                            RawKind::StopKey => FinishReason::KeyPressed,
                            RawKind::KillSwitch => FinishReason::Killed,
                            _ => continue,
                        };
                        let wp = WPARAM(reason_code(reason));
                        let _ = unsafe { PostMessageW(Some(HWND(hwnd as _)), WM_APP_STOP, wp, LPARAM(0)) };
                    }
                });
                self.hook = Some(hook);
            }
            Err(e) => self.status.notice = Some(format!("Esc won't stop playback: {e}")),
        }
    }

    fn countdown(&mut self) {
        let left = relay_player::status::COUNTDOWN_MS - ((self.platform.now_ms)() - self.countdown_from);
        if left > 0.0 {
            self.status.phase = Phase::Countdown { left_ms: left };
            self.redraw();
        } else {
            let _ = unsafe { KillTimer(Some(self.hwnd), TIMER_COUNTDOWN) };
            self.play();
        }
    }

    fn play(&mut self) {
        let Some((m, data)) = self.pending.take() else { return };
        let windows = self.platform.windows.clone();
        if !windows.input_desktop_available() {
            self.status.notice = Some("The screen is locked, so nothing can be played.".into());
            return self.finish(Exit::Locked);
        }
        if windows.foreground().is_some_and(|t| windows.input_blocked(t.pid)) {
            self.status.notice = Some(
                "The app in front runs as administrator, so Windows blocks this program's input to it. \
                 Run this program as administrator to automate it."
                    .into(),
            );
        }
        let (offset, notice) = plan::window_offset(&m, |exe, class| windows.find_window(exe, class));
        if notice.is_some() {
            self.status.notice = notice;
        }
        self.watch(m.playback.stop_on_key);
        let seed = (self.platform.now_ms)().to_bits() ^ (m.id.as_u128() as u64);
        let plan = PlayPlan::for_macro(m, data, 0, seed, offset, self.hwnd.0 as isize);
        let loops = plan.repeat.loops();
        self.status.phase = Phase::Playing(Tick { t: 0.0, advancing: true, speed: plan.speed, loop_idx: 0, loops });
        let sink = WinSink { hwnd: self.hwnd.0 as isize, shared: self.shared.clone() };
        self.engine = Some(relay_playback::spawn(plan, &self.platform, sink));
        self.redraw();
    }

    /// Stop, Esc, a key or the kill switch.
    fn stop(&mut self, reason: FinishReason) {
        if matches!(self.status.phase, Phase::Ended { .. }) {
            return;
        }
        let _ = unsafe { KillTimer(Some(self.hwnd), TIMER_COUNTDOWN) };
        if let Some(engine) = self.engine.take() {
            engine.stop();
        }
        self.finish(Exit::of(reason));
    }

    fn finish(&mut self, exit: Exit) {
        self.engine = None;
        self.hook = None;
        self.pending = None;
        self.status.end(exit);
        self.exit = Some(exit);
        if self.quiet {
            unsafe { PostQuitMessage(0) };
        } else if self.status.closes_on_its_own() {
            unsafe { SetTimer(Some(self.hwnd), TIMER_CLOSE, CLOSE_AFTER_MS, None) };
        }
        self.redraw();
    }

    fn on_tick(&mut self) {
        self.shared.tick_posted.store(false, Ordering::Release);
        let tick = self.shared.tick.lock().unwrap().take();
        if let (Some(tick), Phase::Playing(_)) = (tick, &self.status.phase) {
            self.status.phase = Phase::Playing(tick);
            self.redraw();
        }
    }

    fn on_notice(&mut self) {
        if let Some(n) = self.shared.notices.lock().unwrap().drain(..).next_back() {
            self.status.notice = Some(n);
            self.redraw();
        }
    }

    fn on_done(&mut self) {
        let reason = self.shared.done.lock().unwrap().take();
        if let Some(reason) = reason
            && !matches!(self.status.phase, Phase::Ended { .. })
        {
            self.finish(Exit::of(reason));
        }
    }

    fn on_dpi(&mut self, dpi: u32) {
        self.layout = layout(dpi);
        self.fonts = Fonts::new(&self.layout);
    }

    fn in_stop(&self, x: i32, y: i32) -> bool {
        self.layout.stop.contains(x, y)
    }

    fn paint(&self, dc: HDC) {
        let l = &self.layout;
        unsafe {
            let fill = |r: Rect, c: COLORREF| {
                let brush = CreateSolidBrush(c);
                FillRect(dc, &rect(r), brush);
                let _ = DeleteObject(brush.into());
            };
            let text = |s: &str, r: Rect, font: HFONT, color: COLORREF, flags| {
                SelectObject(dc, font.into());
                SetTextColor(dc, color);
                let mut buf: Vec<u16> = s.encode_utf16().collect();
                let mut r = rect(r);
                DrawTextW(dc, &mut buf, &mut r, flags | DT_NOPREFIX);
            };
            SetBkMode(dc, TRANSPARENT);
            fill(Rect { x: 0, y: 0, w: l.w, h: l.h }, INK);
            fill(Rect { x: l.border, y: l.border, w: l.w - 2 * l.border, h: l.h - 2 * l.border }, BG);
            let one_line = DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS;
            text(&self.status.name, l.name, self.fonts.name, INK, one_line);
            text(&self.status.line(), l.line, self.fonts.text, MUTED, one_line);
            if let Some(n) = &self.status.notice {
                text(n, l.notice, self.fonts.text, ACCENT, DT_WORDBREAK | DT_END_ELLIPSIS);
            }
            fill(l.bar, TRACK);
            let done = (l.bar.w as f32 * self.status.progress()).round() as i32;
            fill(Rect { w: done, ..l.bar }, ACCENT);
            // Stop, or Close once it ended and stays open.
            let b = l.stop;
            let label = if matches!(self.status.phase, Phase::Ended { .. }) { "Close" } else { "Stop" };
            let (face, ink) = if self.hover { (ACCENT, WHITE) } else { (BG, INK) };
            fill(b, if self.hover { ACCENT } else { INK });
            fill(Rect { x: b.x + l.border, y: b.y + l.border, w: b.w - 2 * l.border, h: b.h - 2 * l.border }, face);
            text(label, b, self.fonts.name, ink, one_line | windows::Win32::Graphics::Gdi::DT_CENTER);
        }
    }
}

fn rect(r: Rect) -> RECT {
    RECT { left: r.x, top: r.y, right: r.x + r.w, bottom: r.y + r.h }
}

fn point(lp: LPARAM) -> (i32, i32) {
    ((lp.0 & 0xFFFF) as i16 as i32, ((lp.0 >> 16) & 0xFFFF) as i16 as i32)
}

unsafe extern "system" fn proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    unsafe {
        let handled = match msg {
            WM_PAINT => with_app(|a| {
                let mut ps = PAINTSTRUCT::default();
                let dc = BeginPaint(hwnd, &mut ps);
                let (w, h) = (a.layout.w, a.layout.h);
                // Drawn off screen, then copied: no flicker at 30 updates a second.
                let mem = CreateCompatibleDC(Some(dc));
                let bmp = CreateCompatibleBitmap(dc, w, h);
                let old = SelectObject(mem, bmp.into());
                a.paint(mem);
                let _ = BitBlt(dc, 0, 0, w, h, Some(mem), 0, 0, SRCCOPY);
                SelectObject(mem, old);
                let _ = DeleteObject(bmp.into());
                let _ = DeleteDC(mem);
                let _ = EndPaint(hwnd, &ps);
                LRESULT(0)
            }),
            WM_TIMER if wp.0 == TIMER_COUNTDOWN => with_app(|a| a.countdown()).map(|_| LRESULT(0)),
            WM_TIMER if wp.0 == TIMER_CLOSE => {
                let _ = DestroyWindow(hwnd);
                Some(LRESULT(0))
            }
            WM_APP_TICK => with_app(App::on_tick).map(|_| LRESULT(0)),
            WM_APP_NOTICE => with_app(App::on_notice).map(|_| LRESULT(0)),
            WM_APP_DONE => with_app(App::on_done).map(|_| LRESULT(0)),
            WM_APP_STOP => with_app(|a| a.stop(reason_of(wp.0))).map(|_| LRESULT(0)),
            WM_HOTKEY if wp.0 as i32 == HOTKEY_KILL => with_app(|a| a.stop(FinishReason::Killed)).map(|_| LRESULT(0)),
            // Clicking never takes the focus from the app being automated.
            WM_MOUSEACTIVATE => Some(LRESULT(MA_NOACTIVATE as isize)),
            // Dragged anywhere but the button.
            WM_NCHITTEST => with_app(|a| {
                let mut p = windows::Win32::Foundation::POINT { x: point(lp).0, y: point(lp).1 };
                let _ = ScreenToClient(hwnd, &mut p);
                LRESULT(if a.in_stop(p.x, p.y) { HTCLIENT } else { HTCAPTION } as isize)
            }),
            WM_MOUSEMOVE => with_app(|a| {
                let (x, y) = point(lp);
                let hover = a.in_stop(x, y);
                if !a.tracking {
                    let mut t = TRACKMOUSEEVENT {
                        cbSize: size_of::<TRACKMOUSEEVENT>() as u32,
                        dwFlags: TME_LEAVE,
                        hwndTrack: hwnd,
                        dwHoverTime: 0,
                    };
                    a.tracking = TrackMouseEvent(&mut t).is_ok();
                }
                if hover != a.hover {
                    a.hover = hover;
                    a.redraw();
                }
                LRESULT(0)
            }),
            WM_MOUSELEAVE => with_app(|a| {
                a.tracking = false;
                a.hover = false;
                a.redraw();
                LRESULT(0)
            }),
            WM_LBUTTONUP => {
                let (x, y) = point(lp);
                let ended = with_app(|a| {
                    if !a.in_stop(x, y) {
                        return false;
                    }
                    let ended = matches!(a.status.phase, Phase::Ended { .. });
                    if !ended {
                        a.stop(FinishReason::Stopped);
                    }
                    ended
                });
                if ended == Some(true) {
                    let _ = DestroyWindow(hwnd);
                }
                Some(LRESULT(0))
            }
            WM_DPICHANGED => {
                let dpi = (wp.0 & 0xFFFF) as u32;
                let suggested = *(lp.0 as *const RECT);
                if let Some((w, h)) = with_app(|a| {
                    a.on_dpi(dpi);
                    (a.layout.w, a.layout.h)
                }) {
                    let (x, y) = (suggested.left, suggested.top);
                    let _ = SetWindowPos(hwnd, None, x, y, w, h, SWP_NOZORDER | SWP_NOACTIVATE);
                }
                Some(LRESULT(0))
            }
            WM_DESTROY => {
                PostQuitMessage(0);
                Some(LRESULT(0))
            }
            _ => None,
        };
        handled.unwrap_or_else(|| DefWindowProcW(hwnd, msg, wp, lp))
    }
}
