//! The run history on disk: `runs.json`, machine-local like `library.json`.
//! A file that exists but can't be read is left alone this run; one that
//! doesn't parse is set aside as `runs.json.bad`.

use std::fs;
use std::path::{Path, PathBuf};

use relay_core::runlog::{RunEntry, RunLog};
use serde::{Deserialize, Serialize};

use crate::storage::write_atomic;

#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
struct File {
    version: u32,
    entries: Vec<RunEntry>,
}

pub struct RunHistory {
    path: PathBuf,
    log: RunLog,
    /// Why `runs.json` isn't written this run (see the module docs).
    unreadable: Option<String>,
}

impl RunHistory {
    pub fn open(dir: &Path) -> (Self, Vec<String>) {
        let path = dir.join("runs.json");
        let mut problems = Vec::new();
        let mut unreadable = None;
        let log = match fs::read_to_string(&path) {
            Ok(text) => match serde_json::from_str::<File>(&text) {
                Ok(file) => file.entries.into(),
                Err(e) => {
                    let bad = path.with_extension("json.bad");
                    let _ = fs::rename(&path, &bad);
                    problems.push(format!("Couldn't read {}: {e} (moved to {})", path.display(), bad.display()));
                    RunLog::default()
                }
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => RunLog::default(),
            Err(e) => {
                let why = format!("{} couldn't be read when Relay started ({e})", path.display());
                problems.push(format!("{why}. The run history won't be saved until you restart Relay."));
                unreadable = Some(format!("{why}, so Relay leaves it alone until restarted"));
                RunLog::default()
            }
        };
        (RunHistory { path, log, unreadable }, problems)
    }

    /// Newest first.
    pub fn list(&self) -> Vec<RunEntry> {
        self.log.list()
    }

    /// Adds an entry and saves the file. The entry is kept even if saving fails.
    pub fn add(&mut self, entry: RunEntry) -> std::io::Result<()> {
        self.log.push(entry);
        if let Some(why) = &self.unreadable {
            return Err(std::io::Error::other(why.clone()));
        }
        let mut entries = self.log.list();
        entries.reverse();
        let file = File { version: 1, entries };
        write_atomic(&self.path, &serde_json::to_string_pretty(&file).expect("run history serializes"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use relay_core::runlog::{MAX_RUNS, RunOutcome};
    use relay_core::session::{FinishReason, RunSource};
    use uuid::Uuid;

    fn entry(name: &str) -> RunEntry {
        RunEntry {
            at: chrono::DateTime::from_timestamp(1_790_000_000, 0).unwrap(),
            macro_id: Uuid::from_u128(1),
            macro_name: name.into(),
            source: RunSource::Manual,
            outcome: RunOutcome::Finished(FinishReason::Completed),
            duration_ms: 900,
            from_ms: 0,
            loops: 1,
            speed: 1.0,
            humanize: false,
            checks: vec![],
            checks_dropped: 0,
            note: None,
        }
    }

    fn names(h: &RunHistory) -> Vec<String> {
        h.list().into_iter().map(|e| e.macro_name).collect()
    }

    #[test]
    fn starts_empty_then_saves_each_run_oldest_first() {
        let dir = tempfile::tempdir().unwrap();
        let (mut h, problems) = RunHistory::open(dir.path());
        assert!(problems.is_empty() && h.list().is_empty());
        h.add(entry("a")).unwrap();
        h.add(entry("b")).unwrap();
        let file: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(dir.path().join("runs.json")).unwrap()).unwrap();
        assert_eq!(file["version"], 1);
        assert_eq!(file["entries"][0]["macro_name"], "a");
        let (h, _) = RunHistory::open(dir.path());
        assert_eq!(names(&h), ["b", "a"]);
    }

    #[test]
    fn keeps_the_newest_runs_on_disk() {
        let dir = tempfile::tempdir().unwrap();
        let (mut h, _) = RunHistory::open(dir.path());
        for n in 0..=MAX_RUNS {
            h.add(entry(&n.to_string())).unwrap();
        }
        let (h, _) = RunHistory::open(dir.path());
        let list = names(&h);
        assert_eq!(list.len(), MAX_RUNS);
        assert_eq!((list[0].as_str(), list.last().unwrap().as_str()), ("200", "1"));
    }

    #[test]
    fn an_invalid_file_is_set_aside_and_reported() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("runs.json"), "{ not json").unwrap();
        let (mut h, problems) = RunHistory::open(dir.path());
        assert_eq!(problems.len(), 1);
        assert!(problems[0].contains("runs.json.bad"), "{problems:?}");
        assert_eq!(fs::read_to_string(dir.path().join("runs.json.bad")).unwrap(), "{ not json");
        h.add(entry("a")).unwrap();
        assert!(dir.path().join("runs.json").exists());
    }

    #[test]
    fn a_file_that_cant_be_read_is_never_overwritten() {
        let dir = tempfile::tempdir().unwrap();
        // A folder in its place: it exists, but reading it fails.
        fs::create_dir(dir.path().join("runs.json")).unwrap();
        let (mut h, problems) = RunHistory::open(dir.path());
        assert!(problems[0].contains("won't be saved until you restart Relay"), "{problems:?}");
        let err = h.add(entry("a")).unwrap_err();
        assert!(err.to_string().contains("leaves it alone"), "{err}");
        assert_eq!(names(&h), ["a"], "kept in memory");
        assert!(dir.path().join("runs.json").is_dir());
    }
}
