//! A macro's data file, read from disk each time it's used, so edits made
//! in Excel count on the next run.

use std::path::Path;

use relay_core::data::{self, DataFileInfo, DataTable};
use relay_core::{Event, Macro};

/// The file's name, as messages show it.
pub fn name(path: &Path) -> String {
    path.file_name().map_or_else(|| path.display().to_string(), |n| n.to_string_lossy().into_owned())
}

pub fn read(path: &Path) -> Result<DataTable, String> {
    let name = name(path);
    let bytes = std::fs::read(path).map_err(|e| match e.kind() {
        std::io::ErrorKind::NotFound => format!("{name} isn't there anymore: choose it again in Settings → Playback."),
        _ => format!("Couldn't read {name}: {e}"),
    })?;
    data::parse(&bytes).map_err(|e| format!("{name}: {e}"))
}

/// The rows a macro with `events` plays with (its data file is `path`), or
/// why it can't play.
pub fn for_playing(events: &[Event], path: Option<&Path>) -> Result<Option<DataTable>, String> {
    let table = path.map(read).transpose()?;
    data::check(events, table.as_ref(), &path.map(name).unwrap_or_default())?;
    Ok(table)
}

/// What the Settings tab shows: the file's columns and rows, or why `m`
/// can't play with it.
pub fn info(m: &Macro, path: &Path) -> DataFileInfo {
    let (columns, rows, error) = match read(path) {
        Ok(t) => {
            let error = data::check(&m.events, Some(&t), &name(path)).err();
            (t.columns, t.rows.len() as u32, error)
        }
        Err(e) => (Vec::new(), 0, Some(e)),
    };
    DataFileInfo { path: path.display().to_string(), columns, rows, error }
}

#[cfg(test)]
mod tests {
    use relay_core::model::RecordingMeta;

    use super::*;

    fn typing(text: &str) -> Macro {
        Macro::new("m", RecordingMeta::single_1080p(), vec![Event::Text { t: 0, dur: 100, text: text.into() }])
    }

    #[test]
    fn a_macro_plays_with_the_rows_of_its_file() {
        let dir = tempfile::tempdir().unwrap();
        let csv = dir.path().join("customers.csv");
        std::fs::write(&csv, "Customer,Total\nACME,12\nGlobex,7\n").unwrap();
        let m = typing("{col:customer}");
        assert_eq!(for_playing(&m.events, Some(&csv)).unwrap().unwrap().rows.len(), 2);
        assert_eq!(for_playing(&typing("{n}").events, None), Ok(None));
        assert_eq!(
            info(&m, &csv),
            DataFileInfo {
                path: csv.display().to_string(),
                columns: vec!["Customer".into(), "Total".into()],
                rows: 2,
                error: None
            }
        );
    }

    #[test]
    fn why_it_cant_play_names_the_file() {
        let dir = tempfile::tempdir().unwrap();
        let csv = dir.path().join("customers.csv");
        let gone = for_playing(&typing("{n}").events, Some(&csv)).unwrap_err();
        assert_eq!(gone, "customers.csv isn't there anymore: choose it again in Settings → Playback.");
        assert_eq!(info(&typing("{n}"), &csv).error, Some(gone));

        std::fs::write(&csv, "a,b\n1,2,3").unwrap();
        assert_eq!(
            for_playing(&typing("{n}").events, Some(&csv)).unwrap_err(),
            "customers.csv: Row 1 has 3 values, but there are 2 columns."
        );
        std::fs::write(&csv, "Customer\nACME").unwrap();
        let missing = for_playing(&typing("{col:Total}").events, Some(&csv)).unwrap_err();
        assert_eq!(missing, "customers.csv has no column “Total”.");
        let shown = info(&typing("{col:Total}"), &csv);
        assert_eq!((shown.columns, shown.rows, shown.error), (vec!["Customer".into()], 1, Some(missing)));
        assert_eq!(
            for_playing(&typing("{col:Total}").events, None).unwrap_err(),
            "A Text step types {col:Total}: choose a data file in Settings → Playback."
        );
    }
}
