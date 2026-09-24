//! Running processes, for the "when app launches" trigger (sysinfo, so it
//! works on every platform).

use std::collections::HashSet;

use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};

/// Polls the process list; keep one alive so each refresh is incremental.
pub struct ProcessWatcher {
    sys: System,
}

impl Default for ProcessWatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl ProcessWatcher {
    pub fn new() -> Self {
        ProcessWatcher { sys: System::new() }
    }

    /// Lower-case executable names of everything running ("excel.exe").
    pub fn running(&mut self) -> HashSet<String> {
        self.sys.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing());
        self.sys.processes().values().map(|p| p.name().to_string_lossy().to_lowercase()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sees_this_test_process() {
        let me = std::env::current_exe().unwrap();
        let name = me.file_name().unwrap().to_string_lossy().to_lowercase();
        assert!(ProcessWatcher::new().running().contains(&name), "{name}");
    }
}
