//! Which of Relay's windows show, and where the player goes. The main window
//! steps aside while Relay records (and, if asked, while it plays) and the
//! player shows instead; in player mode the player takes the window's place;
//! a run a trigger starts while the window is open brings the player up in a
//! free corner, so a Stop is always on top.

use parking_lot::Mutex;
use tauri::{AppHandle, Manager, Runtime, WebviewWindow};

use crate::ipc::{Emitter, EngineMsg};
use crate::settings::{PlayerSize, Settings, SettingsStore};
use crate::window_ctl::{self, Rect, WindowState};

/// The player's size at 100%, in CSS px.
pub fn player_css(size: PlayerSize) -> (f64, f64) {
    match size {
        PlayerSize::Compact => (600.0, 64.0),
        PlayerSize::Large => (760.0, 88.0),
    }
}

/// The player grows with the text size up to this, so it stays small.
const MAX_PLAYER_ZOOM: f64 = 1.5;

/// Space kept between the player and the edges of the work area, and between
/// it and the window in a corner (CSS px).
const MARGIN: f64 = 16.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Session {
    #[default]
    Idle,
    /// The countdown included.
    Recording,
    /// Paused included; `triggered` when a macro's trigger started it.
    Playing { triggered: bool },
}

/// What the user chose.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Presence {
    /// The player instead of the window.
    pub player_mode: bool,
    /// Relay put away in the tray.
    pub tucked: bool,
    /// Relay steps aside while the user picks from the screen.
    pub aside: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Shown {
    pub main: bool,
    pub player: bool,
}

/// Which windows show. The window steps aside while recording (and playing,
/// if the settings say so); the player shows whenever a session runs without
/// the window, when a trigger started the run, and in player mode.
pub fn shown(p: Presence, session: Session, settings: &Settings) -> Shown {
    if p.aside {
        return Shown { main: false, player: false };
    }
    let aside = match session {
        Session::Idle => false,
        Session::Recording => settings.hide_while_recording,
        Session::Playing { .. } => settings.hide_while_playing,
    };
    let main = !p.player_mode && !p.tucked && !aside;
    let player = match session {
        Session::Idle => p.player_mode && !p.tucked,
        Session::Recording => !main,
        Session::Playing { triggered } => !main || triggered,
    };
    Shown { main, player }
}

/// The zoom for a player of `css` size in a `work` area at `sf`: the text
/// size's, up to [`MAX_PLAYER_ZOOM`], and less if the work area is too narrow.
pub fn player_zoom(css: (f64, f64), work: Rect, sf: f64, text: f64) -> f64 {
    let room = (work.w as f64 / sf - 2.0 * MARGIN) / css.0;
    text.min(MAX_PLAYER_ZOOM).min(room).max(0.4)
}

/// Where the player goes when it shows beside the window: the first corner
/// of the `work` area (from the top right, then top left, bottom right,
/// bottom left) where a player of `size`, `margin` in from the edges, is
/// clear of the `window` and isn't where the macro `clicks`; the top right if
/// none is.
pub fn free_corner(
    work: Rect,
    size: (i32, i32),
    margin: i32,
    window: Option<Rect>,
    clicks: impl Fn(Rect) -> bool,
) -> Rect {
    let (w, h) = size;
    let (left, right) = (work.x + margin, work.x + work.w - w - margin);
    let (top, bottom) = (work.y + margin, work.y + work.h - h - margin);
    let corners = [(right, top), (left, top), (right, bottom), (left, bottom)].map(|(x, y)| Rect { x, y, w, h });
    let clear = |r: &Rect| !window.is_some_and(|win| win.overlaps(r, margin)) && !clicks(*r);
    corners.iter().copied().find(clear).unwrap_or(corners[0])
}

/// The windows' state: what the user chose besides player mode (saved with
/// the window), the session they were last shown for, and where the player
/// is while it shows.
#[derive(Default)]
pub struct PresenceState(Mutex<Inner>);

#[derive(Default)]
struct Inner {
    tucked: bool,
    aside: bool,
    session: Session,
    player: Option<Rect>,
}

/// Shows and hides the windows for `session`, placing the player when it
/// comes up (in a free corner, clear of the macro's `clicks`, when it shows
/// beside the window). `focus_main` brings the window to the front when it
/// shows. Returns where the player is, if it shows.
pub fn update<R: Runtime>(
    app: &AppHandle<R>,
    session: Session,
    clicks: Option<&dyn Fn(Rect) -> bool>,
    focus_main: bool,
) -> Option<Rect> {
    let settings = app.state::<Mutex<SettingsStore>>().lock().current.clone();
    let player_mode = app.state::<WindowState>().prefs().player_mode;
    let (shown, placed) = {
        let state = app.state::<PresenceState>();
        let mut s = state.0.lock();
        s.session = session;
        (shown(Presence { player_mode, tucked: s.tucked, aside: s.aside }, session, &settings), s.player)
    };
    // Not under the lock: these call into the main thread.
    if let Some(main) = app.get_webview_window("main") {
        if shown.main {
            let _ = main.show();
            if focus_main {
                let _ = main.unminimize();
                let _ = main.set_focus();
            }
        } else {
            let _ = main.hide();
        }
    }
    let player = app.get_webview_window("player")?;
    let at = match (shown.player, placed) {
        (false, _) => {
            window_ctl::hide_inactive(&player);
            None
        }
        (true, Some(r)) => Some(r),
        (true, None) => {
            let beside = shown.main.then(|| app.get_webview_window("main")).flatten();
            let r = place_player(&player, &settings, beside.as_ref(), clicks);
            window_ctl::show_inactive(&player);
            r
        }
    };
    let _ = player.set_always_on_top(settings.keep_on_top.on_top(session != Session::Idle));
    app.state::<PresenceState>().0.lock().player = at;
    at
}

/// Shows the windows again for the last session, after a choice changed.
pub fn refresh<R: Runtime>(app: &AppHandle<R>, focus_main: bool) -> Option<Rect> {
    let session = app.state::<PresenceState>().0.lock().session;
    update(app, session, None, focus_main)
}

/// The player's size or the text size changed: the player is placed again.
pub fn resize_player<R: Runtime>(app: &AppHandle<R>) {
    let showing = app.state::<PresenceState>().0.lock().player.is_some();
    if let (true, Some(player)) = (showing, app.get_webview_window("player")) {
        let settings = app.state::<Mutex<SettingsStore>>().lock().current.clone();
        let at = place_player(&player, &settings, None, None);
        app.state::<PresenceState>().0.lock().player = at;
    }
}

/// Player mode on or off, saved with the window, and both windows told.
pub fn set_player_mode<R: Runtime>(app: &AppHandle<R>, on: bool) {
    app.state::<WindowState>().set_player_mode(on);
    app.state::<PresenceState>().0.lock().tucked = false;
    app.state::<std::sync::Arc<Emitter>>().send(EngineMsg::PlayerMode { on });
    refresh(app, !on);
}

/// Relay put away in the tray, or brought back.
pub fn set_tucked<R: Runtime>(app: &AppHandle<R>, tucked: bool) {
    app.state::<PresenceState>().0.lock().tucked = tucked;
    refresh(app, !tucked);
}

/// Relay's windows step aside while the user picks from the screen, and come back after.
pub fn step_aside<R: Runtime>(app: &AppHandle<R>, on: bool) {
    app.state::<PresenceState>().0.lock().aside = on;
    refresh(app, !on);
}

/// Whether Relay is in the tray.
pub fn tucked<R: Runtime>(app: &AppHandle<R>) -> bool {
    app.state::<PresenceState>().0.lock().tucked
}

/// The window comes back after a recording (unless the player stands in for it).
pub fn untuck<R: Runtime>(app: &AppHandle<R>) {
    app.state::<PresenceState>().0.lock().tucked = false;
}

/// Sizes the player and puts it where it goes: beside the window, in a free
/// corner of the window's monitor; otherwise at the user's anchor. Returns
/// where it is (its client area).
fn place_player<R: Runtime>(
    player: &WebviewWindow<R>,
    settings: &Settings,
    beside: Option<&WebviewWindow<R>>,
    clicks: Option<&dyn Fn(Rect) -> bool>,
) -> Option<Rect> {
    let state = player.app_handle().state::<WindowState>();
    let css = player_css(settings.player_size);
    let text = settings.text_size.zoom();
    let r = match beside.and_then(|w| Some((w.current_monitor().ok()??, window_ctl::outer_rect(w)?))) {
        Some((monitor, window)) => {
            let (work, sf) = (Rect::work_area(&monitor), monitor.scale_factor());
            let zoom = player_zoom(css, work, sf, text);
            let size = ((css.0 * zoom * sf).round() as i32, (css.1 * zoom * sf).round() as i32);
            let _ = player.set_zoom(zoom);
            free_corner(work, size, (MARGIN * sf) as i32, Some(window), |r| clicks.is_some_and(|f| f(r)))
        }
        None => {
            let anchor = state.prefs().player_anchor;
            let monitor = window_ctl::monitor_for(player, anchor)?;
            let (work, sf) = (Rect::work_area(&monitor), monitor.scale_factor());
            let zoom = player_zoom(css, work, sf, text);
            let _ = player.set_zoom(zoom);
            window_ctl::layout(work, sf, zoom, css, anchor)
        }
    };
    state.player_placed(r.bottom_center());
    window_ctl::set_client_rect(player, r.x, r.y, r.w, r.h);
    Some(r)
}

/// The player moved: a drag is the new anchor (see [`WindowState::player_moved`]).
pub fn on_player_moved<R: Runtime>(player: &WebviewWindow<R>) {
    let (Ok(pos), Ok(size)) = (player.inner_position(), player.inner_size()) else { return };
    let at = Rect { x: pos.x, y: pos.y, w: size.width as i32, h: size.height as i32 };
    let app = player.app_handle();
    if app.state::<WindowState>().player_moved(at.bottom_center()) {
        let state = app.state::<PresenceState>();
        let mut s = state.0.lock();
        if s.player.is_some() {
            s.player = Some(at);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const IDLE: Session = Session::Idle;
    const REC: Session = Session::Recording;
    const PLAY: Session = Session::Playing { triggered: false };
    const TRIGGERED: Session = Session::Playing { triggered: true };
    const WINDOW: Presence = Presence { player_mode: false, tucked: false, aside: false };
    const PLAYER: Presence = Presence { player_mode: true, tucked: false, aside: false };
    const TRAY: Presence = Presence { player_mode: false, tucked: true, aside: false };

    fn show(p: Presence, s: Session) -> (bool, bool) {
        let r = shown(p, s, &Settings::default());
        (r.main, r.player)
    }

    #[test]
    fn the_window_steps_aside_while_recording_and_the_player_shows() {
        assert_eq!(show(WINDOW, IDLE), (true, false));
        assert_eq!(show(WINDOW, REC), (false, true));
        let keep = Settings { hide_while_recording: false, ..Settings::default() };
        let r = shown(WINDOW, REC, &keep);
        assert_eq!((r.main, r.player), (true, false), "unless asked not to");
    }

    #[test]
    fn playing_keeps_the_window_unless_asked() {
        assert_eq!(show(WINDOW, PLAY), (true, false));
        let hide = Settings { hide_while_playing: true, ..Settings::default() };
        let r = shown(WINDOW, PLAY, &hide);
        assert_eq!((r.main, r.player), (false, true));
    }

    #[test]
    fn a_triggered_run_brings_the_player_up_beside_the_window() {
        assert_eq!(show(WINDOW, TRIGGERED), (true, true));
        assert_eq!(show(PLAYER, TRIGGERED), (false, true));
        assert_eq!(show(TRAY, TRIGGERED), (false, true));
    }

    #[test]
    fn player_mode_shows_the_player_only() {
        for s in [IDLE, REC, PLAY, TRIGGERED] {
            assert_eq!(show(PLAYER, s), (false, true), "{s:?}");
        }
    }

    #[test]
    fn in_the_tray_only_a_session_shows_the_player() {
        assert_eq!(show(TRAY, IDLE), (false, false));
        assert_eq!(show(TRAY, REC), (false, true));
        assert_eq!(show(TRAY, PLAY), (false, true));
        let tucked_player = Presence { player_mode: true, tucked: true, aside: false };
        assert_eq!(show(tucked_player, IDLE), (false, false));
        assert_eq!(show(tucked_player, PLAY), (false, true));
    }

    #[test]
    fn picking_from_the_screen_steps_every_window_aside() {
        for p in [WINDOW, PLAYER] {
            assert_eq!(show(Presence { aside: true, ..p }, IDLE), (false, false));
        }
    }

    #[test]
    fn the_player_grows_with_the_text_up_to_150_percent() {
        let work = Rect { x: 0, y: 0, w: 1920, h: 1032 };
        assert_eq!(player_zoom((600.0, 64.0), work, 1.0, 1.0), 1.0);
        assert_eq!(player_zoom((600.0, 64.0), work, 1.0, 1.25), 1.25);
        assert_eq!(player_zoom((600.0, 64.0), work, 1.0, 2.0), 1.5);
        // 1366 px at 200% scaling is 683 CSS px: the player fits in it.
        let narrow = Rect { x: 0, y: 0, w: 1366, h: 728 };
        let z = player_zoom((600.0, 64.0), narrow, 2.0, 1.5);
        assert!((z - (683.0 - 32.0) / 600.0).abs() < 1e-9, "{z}");
    }

    #[test]
    fn the_free_corner_avoids_the_window_and_the_clicks() {
        let work = Rect { x: 0, y: 0, w: 1920, h: 1032 };
        let size = (600, 64);
        let none = |_: Rect| false;
        // The window at the bottom center, as by default: the top right is free.
        let window = Rect { x: 488, y: 400, w: 944, h: 612 };
        assert_eq!(free_corner(work, size, 16, Some(window), none), Rect { x: 1304, y: 16, w: 600, h: 64 });
        // A click in the top right: the top left.
        let top_right = |r: Rect| r.x > 1000 && r.y < 100;
        assert_eq!(free_corner(work, size, 16, Some(window), top_right), Rect { x: 16, y: 16, w: 600, h: 64 });
        // A window across the top: the bottom right.
        let top = Rect { x: 0, y: 0, w: 1920, h: 500 };
        assert_eq!(free_corner(work, size, 16, Some(top), none), Rect { x: 1304, y: 952, w: 600, h: 64 });
        // Nowhere is clear: the top right.
        let all = Rect { x: 0, y: 0, w: 1920, h: 1032 };
        assert_eq!(free_corner(work, size, 16, Some(all), none), Rect { x: 1304, y: 16, w: 600, h: 64 });
    }

    #[test]
    fn the_free_corner_keeps_a_margin_from_the_window() {
        let work = Rect { x: 0, y: 0, w: 1920, h: 1032 };
        // The window ends 10 px left of where the top right corner starts: too close.
        let window = Rect { x: 0, y: 0, w: 1294, h: 400 };
        assert_eq!(free_corner(work, (600, 64), 16, Some(window), |_| false), Rect { x: 1304, y: 952, w: 600, h: 64 });
    }

    #[test]
    fn the_free_corner_is_on_the_windows_monitor() {
        // A second monitor to the left, at negative coordinates.
        let work = Rect { x: -1920, y: 0, w: 1920, h: 1040 };
        let r = free_corner(work, (600, 64), 16, None, |_| false);
        assert_eq!(r, Rect { x: -616, y: 16, w: 600, h: 64 });
    }
}
