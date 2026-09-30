//! Playback timing: the mapping between wall-clock time and macro time, with
//! speed changes, pause/resume and seeking, and the humanized play times.
//! Pure: callers pass `now` in milliseconds from any monotonic clock.

use crate::model::{Event, Ms};
use crate::steps::{Step, StepKind};

/// Macro time advances at `speed` × wall time from an anchor. Every change
/// (speed, pause, seek) re-anchors, so there is no drift.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlayClock {
    anchor_t: f64,
    anchor_wall: f64,
    speed: f64,
    paused: bool,
}

/// The speeds a clock accepts; anything else is clamped into the range (and
/// NaN is 1×), so macro time always advances and stays finite.
pub const MIN_SPEED: f64 = 0.01;
pub const MAX_SPEED: f64 = 100.0;

fn valid_speed(speed: f64) -> f64 {
    if speed.is_nan() { 1.0 } else { speed.clamp(MIN_SPEED, MAX_SPEED) }
}

impl PlayClock {
    pub fn new(from: Ms, now: f64, speed: f64) -> Self {
        PlayClock { anchor_t: from as f64, anchor_wall: now, speed: valid_speed(speed), paused: false }
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
        self.speed = valid_speed(speed);
    }

    /// Wall time at which macro time reaches `t`, or `None` while paused.
    pub fn deadline(&self, t: f64) -> Option<f64> {
        (!self.paused).then(|| self.anchor_wall + (t - self.anchor_t) / self.speed)
    }
}

/// When each event plays, in macro milliseconds. With `jitter_ms > 0`
/// ("Humanize") every step moves by one random offset in ±jitter, so the gaps
/// between its presses and releases stay exact. Steps that overlap (a key held
/// across a click) or touch move together, by the first one's offset. A step
/// is held back so it starts no earlier than 0 and than the previous step's
/// (moved) end, so steps never swap. The cursor path between steps (MOVE
/// steps) follows the step before it, kept between its neighbours.
pub fn plan_times(events: &[Event], steps: &[Step], jitter_ms: u32, seed: u64) -> Vec<f64> {
    struct Group {
        t: Ms,
        end: Ms,
        offset: f64,
    }
    let mut rng = fastrand::Rng::with_seed(seed);
    let j = jitter_ms as f64;
    let mut groups: Vec<Group> = Vec::new();
    let mut group_of = vec![None; events.len()];
    for s in steps.iter().filter(|s| !matches!(s.kind, StepKind::Move { .. })) {
        let o = if jitter_ms > 0 { rng.f64() * 2.0 * j - j } else { 0.0 };
        match groups.last_mut() {
            Some(g) if s.t <= g.end => g.end = g.end.max(s.end),
            _ => groups.push(Group { t: s.t, end: s.end, offset: o }),
        }
        for &i in &s.items {
            if let Some(slot) = group_of.get_mut(i as usize) {
                *slot = Some(groups.len() - 1);
            }
        }
    }
    let mut prev_end = 0.0f64;
    for g in &mut groups {
        g.offset = g.offset.max(prev_end - g.t as f64);
        prev_end = g.end as f64 + g.offset;
    }
    let offset = |i: usize| group_of[i].map(|g: usize| groups[g].offset);
    // An event outside the steps plays no later than the next step event.
    let mut next = vec![f64::INFINITY; events.len()];
    let mut upper = f64::INFINITY;
    for i in (0..events.len()).rev() {
        next[i] = upper;
        if let Some(o) = offset(i) {
            upper = events[i].t() as f64 + o;
        }
    }
    let mut carry = 0.0;
    let mut last = 0.0f64;
    (0..events.len())
        .map(|i| {
            let t = events[i].t() as f64;
            last = match offset(i) {
                Some(o) => {
                    carry = o;
                    (t + o).max(last) // only ever by a rounding error
                }
                None => (t + carry).min(next[i]).max(last),
            };
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

    /// The plan is ordered, never negative, and moves each step's events together.
    fn assert_plan(ev: &[Event], steps: &[Step], plan: &[f64], seed: u64) {
        assert!(plan.windows(2).all(|w| w[0] <= w[1]), "seed {seed}: not ordered: {plan:?}");
        assert!(plan.iter().all(|&p| p >= 0.0), "seed {seed}: negative");
        // MOVE steps follow the steps around them instead.
        for s in steps.iter().filter(|s| !matches!(s.kind, StepKind::Move { .. })) {
            let first = s.items[0] as usize;
            let d = plan[first] - ev[first].t() as f64;
            for &i in &s.items {
                let i = i as usize;
                assert!((plan[i] - ev[i].t() as f64 - d).abs() < 1e-9, "seed {seed}: event {i} of {s:?}");
            }
        }
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
        for seed in 0..500 {
            let plan = plan_times(&ev, &steps, 40, seed);
            assert_plan(&ev, &steps, &plan, seed);
            for (i, (p, e)) in plan.iter().zip(&ev).enumerate() {
                assert!((p - e.t() as f64).abs() <= 40.0 + 1e-9, "seed {seed}, event {i}: {p}");
            }
            // The press is held exactly as long as recorded (seed 2 used to hold it 6.2 ms).
            for k in 0..50 {
                assert!((plan[3 * k + 1] - plan[3 * k] - 30.0).abs() < 1e-9, "seed {seed}, press {k}");
            }
        }
        let plan = plan_times(&ev, &steps, 40, 7);
        assert_ne!(plan, plan_times(&ev, &steps, 40, 8), "the seed changes the run");
        assert_eq!(plan, plan_times(&ev, &steps, 40, 7), "and is deterministic");
    }

    #[test]
    fn close_and_overlapping_steps_keep_their_gaps() {
        let b =
            |t, down| Event::Button { t, x: 1, y: 1, btn: crate::model::MouseBtn::Left, down, label: String::new() };
        let ev = [
            // Two taps 5 ms apart: the second can't be pulled before the first.
            key(0, "KeyA", true),
            key(30, "KeyA", false),
            Event::Move { t: 32, x: 0, y: 0 },
            key(35, "KeyB", true),
            key(60, "KeyB", false),
            // A key held across a click: they move together.
            key(1000, "KeyC", true),
            b(1100, true),
            Event::Move { t: 1120, x: 2, y: 2 },
            b(1160, false),
            key(1500, "KeyC", false),
            // A shared Ctrl, outside the steps.
            key(2000, "ControlLeft", true),
            key(2010, "KeyS", true),
            key(2040, "KeyS", false),
            key(2060, "KeyV", true),
            key(2090, "KeyV", false),
            key(2100, "ControlLeft", false),
        ];
        let steps = group_steps(&ev, GroupOptions::default());
        // The move at 32 is a MOVE step; the one at 1120 is the click's.
        assert_eq!(steps.len(), 7, "{steps:#?}");
        for seed in 0..500 {
            let plan = plan_times(&ev, &steps, 40, seed);
            assert_plan(&ev, &steps, &plan, seed);
            assert!((plan[6] - plan[5] - 100.0).abs() < 1e-9, "seed {seed}: the key and the click move together");
        }
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
    fn a_speed_change_while_paused_applies_on_resume() {
        let mut c = PlayClock::new(0, 0.0, 1.0);
        c.pause(100.0);
        c.set_speed(2.0, 500.0);
        assert!(c.paused());
        assert_eq!((c.at(900.0), c.deadline(200.0)), (100.0, None), "still frozen");
        c.resume(1000.0);
        assert_eq!(c.at(1050.0), 200.0);
        assert_eq!(c.deadline(300.0), Some(1100.0));
    }

    #[test]
    fn seeks_before_the_start_clamp_to_zero() {
        let mut c = PlayClock::new(500, 0.0, 1.0);
        c.seek(-250.0, 100.0);
        assert_eq!(c.at(100.0), 0.0);
        assert_eq!(c.at(150.0), 50.0);
    }

    #[test]
    fn unusable_speeds_are_clamped() {
        for (speed, want) in [
            (0.0, MIN_SPEED),
            (-3.0, MIN_SPEED),
            (f64::NEG_INFINITY, MIN_SPEED),
            (f64::INFINITY, MAX_SPEED),
            (1e9, MAX_SPEED),
            (f64::NAN, 1.0),
            (0.5, 0.5),
        ] {
            let mut c = PlayClock::new(0, 0.0, speed);
            assert_eq!(c.speed(), want, "{speed}");
            c.set_speed(speed, 10.0);
            assert_eq!(c.speed(), want, "{speed}");
            assert!(c.at(1000.0).is_finite() && c.deadline(5000.0).is_some_and(f64::is_finite), "{speed}");
        }
    }

    #[test]
    fn a_time_already_passed_has_a_deadline_in_the_past() {
        let c = PlayClock::new(1000, 5000.0, 2.0);
        assert_eq!(c.deadline(800.0), Some(4900.0));
        assert_eq!(c.deadline(1000.0), Some(5000.0));
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
