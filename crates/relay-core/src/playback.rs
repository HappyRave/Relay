//! Playback timing: the mapping between wall-clock time and macro time, with
//! speed changes, pause/resume and seeking, and the humanized play times.
//! Pure: callers pass `now` in milliseconds from any monotonic clock.

use crate::model::{Event, Ms};
use crate::steps::Step;

/// Macro time advances at `speed` × wall time from an anchor. Every change
/// (speed, pause, seek) re-anchors, so there is no drift.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlayClock {
    anchor_t: f64,
    anchor_wall: f64,
    speed: f64,
    paused: bool,
}

impl PlayClock {
    pub fn new(from: Ms, now: f64, speed: f64) -> Self {
        PlayClock { anchor_t: from as f64, anchor_wall: now, speed: speed.max(0.01), paused: false }
    }

    /// Macro time (ms) at wall time `now`.
    pub fn at(&self, now: f64) -> f64 {
        if self.paused { self.anchor_t } else { self.anchor_t + (now - self.anchor_wall) * self.speed }
    }

    pub fn paused(&self) -> bool {
        self.paused
    }

    pub fn speed(&self) -> f64 {
        self.speed
    }

    fn reanchor(&mut self, now: f64) {
        self.anchor_t = self.at(now);
        self.anchor_wall = now;
    }

    pub fn pause(&mut self, now: f64) {
        self.reanchor(now);
        self.paused = true;
    }

    pub fn resume(&mut self, now: f64) {
        if self.paused {
            self.anchor_wall = now;
            self.paused = false;
        }
    }

    /// Jumps to macro time `t`, staying paused or running as it was.
    pub fn seek(&mut self, t: f64, now: f64) {
        self.anchor_t = t.max(0.0);
        self.anchor_wall = now;
    }

    pub fn set_speed(&mut self, speed: f64, now: f64) {
        self.reanchor(now);
        self.speed = speed.max(0.01);
    }

    /// Wall time at which macro time reaches `t`, or `None` while paused.
    pub fn deadline(&self, t: f64) -> Option<f64> {
        (!self.paused).then(|| self.anchor_wall + (t - self.anchor_t) / self.speed)
    }
}

/// When each event plays, in macro milliseconds. With `jitter_ms > 0`
/// ("Humanize") every step moves by one random offset in ±jitter, so a step's
/// presses and releases shift together; events outside steps (the cursor
/// path) follow the step before them. Times are kept in order, so a release
/// never comes before its press, and never go below zero.
pub fn plan_times(events: &[Event], steps: &[Step], jitter_ms: u32, seed: u64) -> Vec<f64> {
    let mut offsets = vec![None; events.len()];
    if jitter_ms > 0 {
        let mut rng = fastrand::Rng::with_seed(seed);
        let j = jitter_ms as f64;
        for s in steps {
            let o = rng.f64() * 2.0 * j - j;
            for &i in &s.items {
                if let Some(slot) = offsets.get_mut(i as usize) {
                    *slot = Some(o);
                }
            }
        }
    }
    let mut carry = 0.0;
    let mut last = 0.0f64;
    events
        .iter()
        .zip(offsets)
        .map(|(e, o)| {
            if let Some(o) = o {
                carry = o;
            }
            last = (e.t() as f64 + carry).max(last).max(0.0);
            last
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::keys::KeyStroke;
    use crate::steps::{GroupOptions, group_steps};

    fn key(t: Ms, code: &str, down: bool) -> Event {
        Event::Key { t, down, key: KeyStroke::code(code), ch: None }
    }

    #[test]
    fn plan_without_jitter_is_the_recording() {
        let ev = [key(0, "KeyA", true), key(40, "KeyA", false), Event::Move { t: 90, x: 0, y: 0 }];
        let steps = group_steps(&ev, GroupOptions::default());
        assert_eq!(plan_times(&ev, &steps, 0, 1), vec![0.0, 40.0, 90.0]);
    }

    #[test]
    fn jitter_moves_whole_steps_within_bounds_and_keeps_order() {
        let mut ev = Vec::new();
        for i in 0..50u32 {
            ev.push(key(i * 100, "KeyA", true));
            ev.push(key(i * 100 + 30, "KeyA", false));
            ev.push(Event::Move { t: i * 100 + 60, x: 0, y: 0 });
        }
        let steps = group_steps(&ev, GroupOptions::default());
        let plan = plan_times(&ev, &steps, 40, 7);
        assert!(plan.windows(2).all(|w| w[0] <= w[1]), "ordered");
        for (i, (p, e)) in plan.iter().zip(&ev).enumerate() {
            // Ordering clamps can only delay an event, never beyond one step's jitter.
            assert!((p - e.t() as f64).abs() <= 40.0 + 1e-9 || *p > e.t() as f64, "event {i}: {p}");
        }
        assert_ne!(plan, plan_times(&ev, &steps, 40, 8), "the seed changes the run");
        assert_eq!(plan, plan_times(&ev, &steps, 40, 7), "and is deterministic");
        // Both halves of a key press move by the same offset.
        let d0 = plan[0] - 0.0;
        assert!((plan[1] - 30.0 - d0).abs() < 1e-9 || plan[1] == plan[0]);
    }

    #[test]
    fn speed_scales_macro_time() {
        let c = PlayClock::new(0, 1000.0, 2.0);
        assert_eq!(c.at(1500.0), 1000.0);
        assert_eq!(c.deadline(1000.0), Some(1500.0));
    }

    #[test]
    fn pause_freezes_and_resume_continues() {
        let mut c = PlayClock::new(0, 0.0, 1.0);
        c.pause(300.0);
        assert_eq!(c.at(5000.0), 300.0);
        assert_eq!(c.deadline(400.0), None);
        c.resume(6000.0);
        assert_eq!(c.at(6100.0), 400.0);
    }

    #[test]
    fn resume_while_running_and_seek_while_paused_keep_time() {
        let mut c = PlayClock::new(0, 0.0, 1.0);
        c.resume(500.0);
        assert_eq!(c.at(600.0), 600.0, "resuming a running clock changes nothing");
        c.pause(700.0);
        c.seek(100.0, 800.0);
        assert_eq!(c.at(5000.0), 100.0, "a seek while paused stays paused");
    }

    #[test]
    fn speed_change_and_seek_reanchor() {
        let mut c = PlayClock::new(0, 0.0, 1.0);
        c.set_speed(4.0, 100.0);
        assert_eq!(c.at(200.0), 500.0);
        c.seek(50.0, 1000.0);
        assert_eq!(c.at(1010.0), 90.0);
    }
}
