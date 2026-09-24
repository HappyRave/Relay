//! Where Relay keeps its files, and crash-safe writes.
//!
//! ```text
//! %APPDATA%\Relay\
//!   settings.json
//!   library.json        order, runs, last run, hotkeys (machine-local)
//!   macros\<id>.rly     one file per macro (portable)
//! ```
//! `RELAY_DATA_DIR` overrides the root, for tests and throwaway runs.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use tauri::{AppHandle, Manager};

pub fn data_dir(app: &AppHandle) -> PathBuf {
    if let Some(dir) = std::env::var_os("RELAY_DATA_DIR") {
        return PathBuf::from(dir);
    }
    app.path().data_dir().map(|d| d.join("Relay")).unwrap_or_else(|_| PathBuf::from("Relay"))
}

/// Writes via a temporary file and a rename, so a crash never leaves a half-written file.
pub fn write_atomic(path: &Path, contents: &str) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("tmp");
    {
        let mut f = fs::File::create(&tmp)?;
        f.write_all(contents.as_bytes())?;
        f.sync_all()?;
    }
    fs::rename(&tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atomic_write_replaces_existing_files() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("a").join("b.json");
        write_atomic(&p, "one").unwrap();
        write_atomic(&p, "two").unwrap();
        assert_eq!(fs::read_to_string(&p).unwrap(), "two");
        assert!(!p.with_extension("tmp").exists());
    }
}
