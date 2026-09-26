//! The polling triggers: weekly schedules, app launches and pixel changes.
//! Each runs on its own thread and asks the coordinator to run a macro; the
//! coordinator decides whether it can (idle, screen unlocked, not paused).
//! Macro hotkeys live in `hotkeys`.
//!
//! What each poll decides is in a small `*Watch` type, fed the triggers and
//! the world (the time, the running programs, a pixel reader); the threads
//! only sleep and feed them. So the decisions are tested without threads.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use chrono::{DateTime, Local, TimeDelta, TimeZone};
use parking_lot::Mutex;
use relay_core::model::Rgb;
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

/// Every macro's triggers, as the library shares them.
pub type AllTriggers = Arc<[(Uuid, MacroTriggers)]>;

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
pub fn next_scheduled<Tz: TimeZone>(t: &MacroTriggers, now: DateTime<Tz>) -> Option<DateTime<Tz>> {
    t.schedule.enabled.then(|| next_run(&now, &t.schedule.schedule)).flatten()
}

/// Runs a schedule when its next run, as seen from the previous tick, has
/// come. Comparing wall-clock times each tick survives sleep and clock changes.
pub struct ScheduleWatch<Tz: TimeZone> {
    last: DateTime<Tz>,
}

impl<Tz: TimeZone> ScheduleWatch<Tz> {
    pub fn new(now: DateTime<Tz>) -> Self {
        ScheduleWatch { last: now }
    }

    /// The macros due since the last tick. A run more than two minutes late
    /// (the PC slept through it) is skipped, not made up.
    pub fn tick(&mut self, now: DateTime<Tz>, triggers: &[(Uuid, MacroTriggers)]) -> Vec<Uuid> {
        let mut due = Vec::new();
        for (id, t) in triggers {
            let Some(at) = next_scheduled(t, self.last.clone()) else { continue };
            if at <= now {
                if now.clone() - at.clone() <= MISSED_AFTER {
                    due.push(*id);
                } else {
                    tracing::info!(%id, "skipped a missed scheduled run");
                }
            }
        }
        self.last = now;
        due
    }
}

/// Fires when a watched program starts.
#[derive(Default)]
pub struct LaunchWatch {
    edges: HashMap<(Uuid, String), ProcessLaunchEdge>,
}

impl LaunchWatch {
    /// The programs to watch: lower-case executable names of enabled triggers.
    pub fn wanted(triggers: &[(Uuid, MacroTriggers)]) -> Vec<(Uuid, String, u32)> {
        triggers
            .iter()
            .filter(|(_, t)| t.app_launch.enabled && !t.app_launch.exe.trim().is_empty())
            .map(|(id, t)| (*id, t.app_launch.exe.trim().to_lowercase(), t.app_launch.delay_ms))
            .collect()
    }

    /// The macros whose program started since the last tick, with their
    /// delay. `running` holds lower-case executable names.
    pub fn tick(&mut self, wanted: &[(Uuid, String, u32)], running: &HashSet<String>) -> Vec<(Uuid, u32)> {
        self.edges.retain(|(id, exe), _| wanted.iter().any(|(w, e, _)| w == id && e == exe));
        let mut fired = Vec::new();
        for (id, exe, delay) in wanted {
            let edge = self.edges.entry((*id, exe.clone())).or_default();
            if edge.update(running.contains(exe)) {
                fired.push((*id, *delay));
            }
        }
        fired
    }
}

/// Fires when a watched pixel turns its color.
#[derive(Default)]
pub struct PixelWatch {
    edges: HashMap<Uuid, (PixelEdge, (i32, i32))>,
}

impl PixelWatch {
    /// The macros whose pixel just became its color, reading pixels with `read`.
    pub fn tick(
        &mut self,
        triggers: &[(Uuid, MacroTriggers)],
        mut read: impl FnMut(i32, i32) -> Option<Rgb>,
    ) -> Vec<Uuid> {
        let wanted: Vec<_> = triggers.iter().filter(|(_, t)| t.pixel.enabled).collect();
        self.edges.retain(|id, _| wanted.iter().any(|(w, _)| w == id));
        let mut fired = Vec::new();
        for (id, t) in wanted {
            let p = &t.pixel;
            let entry = self.edges.entry(*id).or_insert_with(|| (PixelEdge::new(), (p.x, p.y)));
            if entry.1 != (p.x, p.y) {
                // Moved to another pixel: start over rather than fire on a stale edge.
                *entry = (PixelEdge::new(), (p.x, p.y));
            }
            // An unreadable screen (locked, a UAC prompt) is no sample at all:
            // counting it as "doesn't match" would re-arm the edge.
            let Some(color) = read(p.x, p.y) else { continue };
            if entry.0.update(color.within(p.color, p.tolerance)) {
                fired.push(*id);
            }
        }
        fired
    }
}

fn snapshot(app: &AppHandle) -> AllTriggers {
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

fn schedule_loop(app: AppHandle) {
    let mut watch = ScheduleWatch::new(Local::now());
    loop {
        std::thread::sleep(SCHEDULE_TICK);
        for id in watch.tick(Local::now(), &snapshot(&app)) {
            fire(&app, id, RunSource::Schedule);
        }
    }
}

fn app_launch_loop(app: AppHandle) {
    let mut processes = ProcessWatcher::new();
    let mut watch = LaunchWatch::default();
    loop {
        let wanted = LaunchWatch::wanted(&snapshot(&app));
        // Listing processes isn't free: only when something is watched.
        let running = if wanted.is_empty() { HashSet::new() } else { processes.running() };
        for (id, delay) in watch.tick(&wanted, &running) {
            let app = app.clone();
            // Give the app's window time to appear.
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(delay as u64));
                fire(&app, id, RunSource::AppLaunch);
            });
        }
        std::thread::sleep(APP_POLL);
    }
}

fn pixel_loop(app: AppHandle, platform: Arc<Platform>) {
    let mut watch = PixelWatch::default();
    loop {
        std::thread::sleep(PIXEL_POLL);
        for id in watch.tick(&snapshot(&app), |x, y| platform.screen.pixel(x, y)) {
            fire(&app, id, RunSource::Pixel);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{FixedOffset, TimeZone};
    use relay_core::schedule::WeeklySchedule;
    use relay_core::triggers::{AppLaunchTrigger, PixelTrigger, ScheduleTrigger};

    fn tz() -> FixedOffset {
        FixedOffset::east_opt(2 * 3600).unwrap()
    }
    fn at(d: u32, h: u32, m: u32, s: u32) -> DateTime<FixedOffset> {
        // September 2026: the 24th is a Thursday.
        tz().with_ymd_and_hms(2026, 9, d, h, m, s).unwrap()
    }
    fn id(n: u128) -> Uuid {
        Uuid::from_u128(n)
    }
    fn scheduled(time: &str, days: [bool; 7]) -> MacroTriggers {
        let schedule = WeeklySchedule { days, time: chrono::NaiveTime::parse_from_str(time, "%H:%M").unwrap() };
        MacroTriggers { schedule: ScheduleTrigger { enabled: true, schedule }, ..Default::default() }
    }
    const WEEKDAYS: [bool; 7] = [true, true, true, true, true, false, false];

    #[test]
    fn trigger_state_pauses_and_resumes() {
        let s = TriggerState::default();
        assert!(!s.paused());
        s.set_paused(true);
        assert!(s.paused());
        s.set_paused(false);
        assert!(!s.paused());
    }

    #[test]
    fn next_scheduled_needs_the_schedule_on() {
        let mut t = MacroTriggers::default();
        assert_eq!(next_scheduled(&t, at(24, 8, 0, 0)), None);
        t.schedule.enabled = true;
        assert_eq!(next_scheduled(&t, at(24, 8, 0, 0)), Some(at(24, 9, 0, 0)));
    }

    #[test]
    fn a_schedule_fires_once_when_its_time_passes() {
        let triggers = vec![(id(1), scheduled("09:00", WEEKDAYS))];
        let mut w = ScheduleWatch::new(at(24, 8, 59, 50));
        assert!(w.tick(at(24, 8, 59, 55), &triggers).is_empty(), "not yet");
        assert_eq!(w.tick(at(24, 9, 0, 0), &triggers), [id(1)], "exactly on time");
        assert!(w.tick(at(24, 9, 0, 5), &triggers).is_empty(), "only once");
        assert!(w.tick(at(24, 9, 0, 10), &triggers).is_empty());
    }

    #[test]
    fn a_schedule_fires_a_few_seconds_late_but_skips_a_run_missed_by_sleep() {
        let triggers = vec![(id(1), scheduled("09:00", WEEKDAYS))];
        let mut w = ScheduleWatch::new(at(24, 8, 59, 58));
        assert_eq!(w.tick(at(24, 9, 0, 3), &triggers), [id(1)], "a tick 3 s after the minute still runs it");

        // The PC slept from 08:59 to 09:30: the run is skipped, not made up.
        let mut w = ScheduleWatch::new(at(25, 8, 59, 0));
        assert!(w.tick(at(25, 9, 30, 0), &triggers).is_empty());
        // Up to two minutes late is still run.
        let mut w = ScheduleWatch::new(at(25, 8, 59, 0));
        assert_eq!(w.tick(at(25, 9, 2, 0), &triggers), [id(1)]);
    }

    #[test]
    fn schedules_respect_their_days_and_disabled_ones_never_fire() {
        let saturday_only = [false, false, false, false, false, true, false];
        let mut off = scheduled("09:00", WEEKDAYS);
        off.schedule.enabled = false;
        let triggers = vec![(id(1), scheduled("09:00", saturday_only)), (id(2), off)];
        // Thursday 09:00: neither.
        let mut w = ScheduleWatch::new(at(24, 8, 59, 58));
        assert!(w.tick(at(24, 9, 0, 1), &triggers).is_empty());
        // Saturday 09:00: the Saturday one.
        let mut w = ScheduleWatch::new(at(26, 8, 59, 58));
        assert_eq!(w.tick(at(26, 9, 0, 1), &triggers), [id(1)]);
    }

    #[test]
    fn several_macros_can_share_a_time() {
        let triggers = vec![(id(1), scheduled("09:00", WEEKDAYS)), (id(2), scheduled("09:00", WEEKDAYS))];
        let mut w = ScheduleWatch::new(at(24, 8, 59, 58));
        assert_eq!(w.tick(at(24, 9, 0, 1), &triggers), [id(1), id(2)]);
    }

    fn launching(exe: &str, delay_ms: u32) -> MacroTriggers {
        MacroTriggers {
            app_launch: AppLaunchTrigger { enabled: true, exe: exe.into(), delay_ms },
            ..Default::default()
        }
    }
    fn running(names: &[&str]) -> HashSet<String> {
        names.iter().map(|n| n.to_string()).collect()
    }

    #[test]
    fn launch_watch_wants_enabled_triggers_by_lowercase_name() {
        let mut blank = launching("  ", 0);
        blank.app_launch.enabled = true;
        let mut off = launching("notepad.exe", 0);
        off.app_launch.enabled = false;
        let triggers = vec![(id(1), launching(" EXCEL.EXE ", 1500)), (id(2), blank), (id(3), off)];
        assert_eq!(LaunchWatch::wanted(&triggers), vec![(id(1), "excel.exe".to_string(), 1500)]);
    }

    #[test]
    fn a_launch_fires_on_start_not_while_running_or_at_baseline() {
        let wanted = LaunchWatch::wanted(&[(id(1), launching("excel.exe", 2000))]);
        let mut w = LaunchWatch::default();
        assert!(w.tick(&wanted, &running(&["excel.exe"])).is_empty(), "already running at the first look");
        assert!(w.tick(&wanted, &running(&["excel.exe"])).is_empty());
        assert!(w.tick(&wanted, &running(&[])).is_empty(), "closed");
        assert_eq!(w.tick(&wanted, &running(&["excel.exe", "other.exe"])), [(id(1), 2000)], "started again");
        assert!(w.tick(&wanted, &running(&["excel.exe"])).is_empty(), "once per start");
    }

    #[test]
    fn a_launch_watch_forgets_triggers_that_are_turned_off() {
        let on = LaunchWatch::wanted(&[(id(1), launching("excel.exe", 0))]);
        let mut w = LaunchWatch::default();
        w.tick(&on, &running(&[]));
        // Turned off while Excel starts, then on again while it runs: the
        // fresh edge takes that as its baseline instead of firing.
        w.tick(&[], &running(&["excel.exe"]));
        assert!(w.tick(&on, &running(&["excel.exe"])).is_empty());
    }

    fn watching(x: i32, y: i32, color: Rgb) -> MacroTriggers {
        MacroTriggers { pixel: PixelTrigger { enabled: true, x, y, color, tolerance: 8 }, ..Default::default() }
    }
    const RED: Rgb = Rgb(0xEC, 0x30, 0x13);
    const WHITE: Rgb = Rgb(255, 255, 255);

    #[test]
    fn a_pixel_fires_when_it_becomes_the_color_and_holds_for_two_samples() {
        let triggers = vec![(id(1), watching(5, 6, RED))];
        let mut w = PixelWatch::default();
        let mut tick = |c: Rgb| {
            w.tick(&triggers, |x, y| {
                assert_eq!((x, y), (5, 6));
                Some(c)
            })
        };
        assert!(tick(WHITE).is_empty());
        assert!(tick(RED).is_empty(), "one sample could be a flicker");
        assert_eq!(tick(Rgb(0xE8, 0x34, 0x10)), [id(1)], "within the tolerance");
        assert!(tick(RED).is_empty(), "staying red fires once");
        assert!(tick(WHITE).is_empty());
        assert!(tick(RED).is_empty());
        assert_eq!(tick(RED), [id(1)], "and again after a change");
    }

    #[test]
    fn a_pixel_that_starts_red_waits_for_a_change() {
        let triggers = vec![(id(1), watching(0, 0, RED))];
        let mut w = PixelWatch::default();
        for _ in 0..5 {
            assert!(w.tick(&triggers, |_, _| Some(RED)).is_empty());
        }
    }

    #[test]
    fn unreadable_pixels_are_no_sample() {
        let triggers = vec![(id(1), watching(0, 0, RED))];
        let mut w = PixelWatch::default();
        w.tick(&triggers, |_, _| Some(RED));
        w.tick(&triggers, |_, _| Some(RED));
        // The screen locks (unreadable) and unlocks, still red: no fire.
        assert!(w.tick(&triggers, |_, _| None).is_empty());
        assert!(w.tick(&triggers, |_, _| Some(RED)).is_empty());
        assert!(w.tick(&triggers, |_, _| Some(RED)).is_empty());
    }

    #[test]
    fn moving_the_watched_pixel_starts_over() {
        let mut w = PixelWatch::default();
        let here = vec![(id(1), watching(0, 0, RED))];
        w.tick(&here, |_, _| Some(WHITE));
        w.tick(&here, |_, _| Some(RED));
        // Moved to a pixel that's red already: that's a baseline, not a change.
        let there = vec![(id(1), watching(9, 9, RED))];
        assert!(w.tick(&there, |_, _| Some(RED)).is_empty());
        assert!(w.tick(&there, |_, _| Some(RED)).is_empty());
    }

    #[test]
    fn only_enabled_pixel_triggers_are_read() {
        let mut off = watching(0, 0, RED);
        off.pixel.enabled = false;
        let mut w = PixelWatch::default();
        let mut reads = 0;
        w.tick(&[(id(1), off), (id(2), MacroTriggers::default())], |_, _| {
            reads += 1;
            Some(RED)
        });
        assert_eq!(reads, 0);
    }
}
