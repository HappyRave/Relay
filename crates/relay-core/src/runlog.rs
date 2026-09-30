//! The run history: when each macro ran, what started it, how it ended and
//! what its pixel checks and Find image steps found. Kept machine-local by the
//! app (`runs.json`), the newest [`MAX_RUNS`] entries.

use std::collections::VecDeque;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

use crate::model::Ms;
use crate::session::{FinishReason, RunSource};

/// How many entries the history keeps.
pub const MAX_RUNS: usize = 200;
/// How many check results one run keeps (the latest ones).
pub const MAX_CHECKS: usize = 50;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct RunEntry {
    /// When the run started, or when the skipped trigger fired.
    pub at: DateTime<Utc>,
    pub macro_id: Uuid,
    /// The name when it ran (the macro may be renamed or deleted since).
    pub macro_name: String,
    pub source: RunSource,
    pub outcome: RunOutcome,
    /// Wall time from start to end, pauses included; 0 for a skip.
    pub duration_ms: Ms,
    /// Where on the timeline playback started.
    pub from_ms: Ms,
    /// Loops played, the last one included even if it didn't finish.
    pub loops: u32,
    pub speed: f32,
    pub humanize: bool,
    /// The latest pixel checks and Find image steps, oldest first.
    pub checks: Vec<CheckResult>,
    /// Earlier checks that weren't kept.
    pub checks_dropped: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "type", content = "reason", rename_all = "snake_case")]
#[ts(export)]
pub enum RunOutcome {
    Finished(FinishReason),
    Skipped(SkipReason),
}

/// Why a trigger that fired didn't run its macro.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum SkipReason {
    /// Something else was recording or playing.
    Busy,
    /// The screen was locked (or a UAC prompt was up).
    Locked,
    /// A scheduled run the PC slept through.
    Missed,
}

/// What one pixel check or Find image step did.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CheckResult {
    /// The 1-based step number.
    pub step: u32,
    /// The 0-based loop it happened in.
    pub loop_idx: u32,
    /// A Find image step, rather than a pixel check.
    pub image: bool,
    /// How long it waited (pauses not counted).
    pub after_ms: Ms,
    pub outcome: CheckOutcome,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
#[ts(export)]
pub enum CheckOutcome {
    /// The pixel matched.
    Matched,
    /// The image was found, its top-left corner at `x, y`, with a score in percent.
    Found { x: i32, y: i32, score: u8 },
    /// Nothing before the timeout: the run stopped there.
    TimedOut,
    /// The run was stopped while it waited.
    Interrupted,
}

/// A run's check results as they come: only the latest [`MAX_CHECKS`] are kept,
/// so a macro looping forever records in constant memory.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CheckLog {
    checks: VecDeque<CheckResult>,
    dropped: u32,
}

impl CheckLog {
    pub fn push(&mut self, c: CheckResult) {
        if self.checks.len() == MAX_CHECKS {
            self.checks.pop_front();
            self.dropped = self.dropped.saturating_add(1);
        }
        self.checks.push_back(c);
    }

    /// The kept results, oldest first, and how many earlier ones were dropped.
    pub fn into_parts(self) -> (Vec<CheckResult>, u32) {
        (self.checks.into(), self.dropped)
    }
}

/// The history, oldest first in memory and on disk.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RunLog {
    entries: VecDeque<RunEntry>,
}

impl RunLog {
    /// Adds an entry, dropping the oldest beyond [`MAX_RUNS`].
    pub fn push(&mut self, e: RunEntry) {
        self.entries.push_back(e);
        while self.entries.len() > MAX_RUNS {
            self.entries.pop_front();
        }
    }

    /// Newest first, as the UI shows them.
    pub fn list(&self) -> Vec<RunEntry> {
        self.entries.iter().rev().cloned().collect()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl From<Vec<RunEntry>> for RunLog {
    fn from(entries: Vec<RunEntry>) -> Self {
        let mut log = RunLog::default();
        for e in entries {
            log.push(e);
        }
        log
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(n: u32) -> RunEntry {
        RunEntry {
            at: DateTime::from_timestamp(1_790_000_000 + n as i64, 0).unwrap(),
            macro_id: Uuid::from_u128(7),
            macro_name: format!("Run {n}"),
            source: RunSource::Schedule,
            outcome: RunOutcome::Finished(FinishReason::Completed),
            duration_ms: 1200,
            from_ms: 0,
            loops: 1,
            speed: 1.0,
            humanize: false,
            checks: vec![],
            checks_dropped: 0,
        }
    }

    fn check(step: u32) -> CheckResult {
        CheckResult { step, loop_idx: 0, image: false, after_ms: 30, outcome: CheckOutcome::Matched }
    }

    #[test]
    fn keeps_the_newest_runs_and_lists_them_newest_first() {
        let mut log = RunLog::default();
        for n in 0..MAX_RUNS as u32 + 5 {
            log.push(entry(n));
        }
        assert_eq!(log.len(), MAX_RUNS);
        let list = log.list();
        assert_eq!(list[0].macro_name, format!("Run {}", MAX_RUNS + 4));
        assert_eq!(list.last().unwrap().macro_name, "Run 5");
    }

    #[test]
    fn a_long_list_read_from_disk_is_trimmed_too() {
        let log = RunLog::from((0..MAX_RUNS as u32 + 1).map(entry).collect::<Vec<_>>());
        assert_eq!(log.len(), MAX_RUNS);
        assert_eq!(log.list().last().unwrap().macro_name, "Run 1");
    }

    #[test]
    fn keeps_the_latest_checks_and_counts_the_rest() {
        let mut checks = CheckLog::default();
        for n in 1..=MAX_CHECKS as u32 + 3 {
            checks.push(check(n));
        }
        let (kept, dropped) = checks.into_parts();
        assert_eq!(kept.len(), MAX_CHECKS);
        assert_eq!(kept[0].step, 4);
        assert_eq!(kept.last().unwrap().step, MAX_CHECKS as u32 + 3);
        assert_eq!(dropped, 3);
    }

    #[test]
    fn serializes_as_the_ui_and_runs_json_expect() {
        let mut e = entry(0);
        e.outcome = RunOutcome::Skipped(SkipReason::Busy);
        e.checks = vec![
            CheckResult {
                step: 3,
                loop_idx: 1,
                image: true,
                after_ms: 250,
                outcome: CheckOutcome::Found { x: 10, y: 20, score: 97 },
            },
            CheckResult { outcome: CheckOutcome::TimedOut, ..check(4) },
        ];
        let json = serde_json::to_value(&e).unwrap();
        assert_eq!(json["outcome"], serde_json::json!({ "type": "skipped", "reason": "busy" }));
        assert_eq!(json["source"], "schedule");
        assert_eq!(json["checks"][0]["outcome"], serde_json::json!({ "type": "found", "x": 10, "y": 20, "score": 97 }));
        assert_eq!(json["checks"][1]["outcome"], serde_json::json!({ "type": "timed_out" }));
        let back: RunEntry = serde_json::from_value(json).unwrap();
        assert_eq!(back, e);
        let finished = serde_json::to_value(RunOutcome::Finished(FinishReason::PixelTimeout)).unwrap();
        assert_eq!(finished, serde_json::json!({ "type": "finished", "reason": "pixel_timeout" }));
    }
}
