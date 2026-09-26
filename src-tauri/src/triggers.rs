//! The polling triggers: weekly schedules, app launches and pixel changes.
//! Each runs on its own thread and asks the coordinator to run a macro; the
//! coordinator decides whether it can (idle, screen unlocked, not paused).
//! Macro hotkeys live in `hotkeys`.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use parking_lot::Mutex;
use std::time::Duration;

use chrono::{DateTime, Local, TimeDelta};
use relay_core::schedule::next_run;
use relay_core::session::RunSource;
use relay_core::triggers::{MacroTriggers, PixelEdge, ProcessLaunchEdge};
use relay_platform::Platform;
use relay_platform::processes::ProcessWatcher;
use tauri::{AppHandle, Manager};
use uuid::Uuid;

use crate::coordinator::{Cmd, CoordinatorHandle};
use crate::library::Library;

const SCHEDULE_TICK: Duration = Duration::from_secs(5);
const APP_POLL: Duration = Duration::from_secs(2);
const PIXEL_POLL: Duration = Duration::from_millis(250);
/// A scheduled run found later than this (e.g. after sleep) is skipped, not run late.
const MISSED_AFTER: TimeDelta = TimeDelta::minutes(2);

/// Whether triggers may fire. Paused by the kill switch, resumed from the tray or the Triggers tab.
#[derive(Default)]
pub struct TriggerState {
    paused: AtomicBool,
}

impl TriggerState {
    pub fn paused(&self) -> bool {
        self.paused.load(Ordering::SeqCst)
    }

    pub fn set_paused(&self, paused: bool) {
        self.paused.store(paused, Ordering::SeqCst);
    }
}

/// The next scheduled run of a macro's triggers, if it has one.
pub fn next_scheduled(t: &MacroTriggers, now: DateTime<Local>) -> Option<DateTime<Local>> {
    t.schedule.enabled.then(|| next_run(&now, &t.schedule.schedule)).flatten()
}

/// Every macro's triggers (shared, not copied: the library rebuilds it on change).
fn snapshot(app: &AppHandle) -> Arc<[(Uuid, MacroTriggers)]> {
    app.state::<Mutex<Library>>().lock().all_triggers()
}

/// Asks the coordinator to run the macro; it decides whether it can.
fn fire(app: &AppHandle, id: Uuid, source: RunSource) {
    tracing::info!(%id, ?source, "trigger due");
    app.state::<CoordinatorHandle>().send(Cmd::RunMacro { id, source });
}

pub fn spawn(app: AppHandle, platform: Arc<Platform>) {
    let a = app.clone();
    std::thread::Builder::new().name("relay-schedule".into()).spawn(move || schedule_loop(a)).expect("spawn scheduler");
    let a = app.clone();
    std::thread::Builder::new()
        .name("relay-app-launch".into())
        .spawn(move || app_launch_loop(a))
        .expect("spawn app watcher");
    std::thread::Builder::new()
        .name("relay-pixel-trigger".into())
        .spawn(move || pixel_loop(app, platform))
        .expect("spawn pixel watcher");
}

/// Runs a schedule when its next run (as seen from the previous tick) has
/// come. Comparing wall-clock times each tick survives sleep and clock changes.
fn schedule_loop(app: AppHandle) {
    let mut last = Local::now();
    loop {
        std::thread::sleep(SCHEDULE_TICK);
        let now = Local::now();
        for (id, t) in snapshot(&app).iter() {
            let Some(due) = next_scheduled(t, last) else { continue };
            if due <= now {
                if now - due <= MISSED_AFTER {
                    fire(&app, *id, RunSource::Schedule);
                } else {
                    tracing::info!(%id, %due, "skipped a missed scheduled run");
                }
            }
        }
        last = now;
    }
}

fn app_launch_loop(app: AppHandle) {
    let mut watcher = ProcessWatcher::new();
    let mut edges: HashMap<(Uuid, String), ProcessLaunchEdge> = HashMap::new();
    loop {
        let wanted: Vec<(Uuid, String, u32)> = snapshot(&app)
            .iter()
            .filter(|(_, t)| t.app_launch.enabled && !t.app_launch.exe.trim().is_empty())
            .map(|(id, t)| (*id, t.app_launch.exe.trim().to_lowercase(), t.app_launch.delay_ms))
            .collect();
        if !wanted.is_empty() {
            let running = watcher.running();
            edges.retain(|(id, exe), _| wanted.iter().any(|(w, e, _)| w == id && e == exe));
            for (id, exe, delay) in wanted {
                let edge = edges.entry((id, exe.clone())).or_default();
                if edge.update(running.contains(&exe)) {
                    let app = app.clone();
                    // Give the app's window time to appear.
                    std::thread::spawn(move || {
                        std::thread::sleep(Duration::from_millis(delay as u64));
                        fire(&app, id, RunSource::AppLaunch);
                    });
                }
            }
        } else {
            edges.clear();
        }
        std::thread::sleep(APP_POLL);
    }
}

fn pixel_loop(app: AppHandle, platform: Arc<Platform>) {
    let mut edges: HashMap<Uuid, (PixelEdge, (i32, i32))> = HashMap::new();
    loop {
        std::thread::sleep(PIXEL_POLL);
        let all = snapshot(&app);
        let wanted: Vec<_> = all.iter().filter(|(_, t)| t.pixel.enabled).collect();
        edges.retain(|id, _| wanted.iter().any(|(w, _)| w == id));
        for (id, t) in wanted {
            let p = &t.pixel;
            let entry = edges.entry(*id).or_insert_with(|| (PixelEdge::new(), (p.x, p.y)));
            if entry.1 != (p.x, p.y) {
                // Moved to another pixel: start over rather than fire on a stale edge.
                *entry = (PixelEdge::new(), (p.x, p.y));
            }
            // An unreadable screen (locked, a UAC prompt) is no sample at all:
            // counting it as "doesn't match" would re-arm the edge.
            let Some(color) = platform.screen.pixel(p.x, p.y) else { continue };
            if entry.0.update(color.within(p.color, p.tolerance)) {
                fire(&app, *id, RunSource::Pixel);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn next_scheduled_needs_the_schedule_on() {
        let now = Local.with_ymd_and_hms(2026, 9, 24, 8, 0, 0).unwrap();
        let mut t = MacroTriggers::default();
        assert_eq!(next_scheduled(&t, now), None);
        t.schedule.enabled = true;
        assert_eq!(next_scheduled(&t, now), Some(Local.with_ymd_and_hms(2026, 9, 24, 9, 0, 0).unwrap()));
    }
}
