//! Where Relay keeps its files, and crash-safe writes.
//!
//! ```text
//! %APPDATA%\Relay\
//!   settings.json
//!   library.json        order, runs, last run, triggers (machine-local)
//!   window.json         where the widget sits
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

/// Writes via a uniquely named temporary file in the same folder and a
/// rename, so a crash never leaves a half-written file and two writers of
/// the same file never share a temporary one.
pub fn write_atomic(path: &Path, contents: &str) -> io::Result<()> {
    let dir = path.parent().filter(|d| !d.as_os_str().is_empty()).unwrap_or(Path::new("."));
    fs::create_dir_all(dir)?;
    let mut tmp = tempfile::NamedTempFile::new_in(dir)?;
    tmp.write_all(contents.as_bytes())?;
    tmp.as_file().sync_all()?;
    tmp.persist(path).map_err(|e| e.error)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atomic_write_replaces_existing_files_and_leaves_nothing_behind() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("a").join("b.json");
        // A user's own file with the old temporary name is left alone.
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p.with_extension("tmp"), "mine").unwrap();
        write_atomic(&p, "one").unwrap();
        write_atomic(&p, "two").unwrap();
        assert_eq!(fs::read_to_string(&p).unwrap(), "two");
        assert_eq!(fs::read_to_string(p.with_extension("tmp")).unwrap(), "mine");
        assert_eq!(fs::read_dir(p.parent().unwrap()).unwrap().count(), 2);
    }
}
