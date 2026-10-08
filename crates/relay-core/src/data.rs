//! Data files: a CSV whose rows drive a macro's repeats, one run per row.
//! Text steps type a column of the current row with `{col:Name}`.
//!
//! The first row names the columns. Values are separated by commas,
//! semicolons or tabs (whichever the header uses most), may be quoted (`""`
//! is a quote), and may span lines inside quotes. The file is UTF-8 (with or
//! without a BOM) or, failing that, Windows-1252, as Excel saves "CSV".

use serde::{Deserialize, Serialize};
use thiserror::Error;
use ts_rs::TS;

use crate::model::Event;
use crate::text::{self, Part};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DataTable {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum DataError {
    #[error("The file is empty: its first row should name the columns.")]
    Empty,
    #[error("A quote isn't closed in row {0}.")]
    Unclosed(usize),
    #[error("Row {row} has {values} values, but there are {columns} columns.")]
    TooManyValues { row: usize, values: usize, columns: usize },
    #[error("Two columns are named “{0}”.")]
    SameName(String),
}

/// What the UI shows of a macro's data file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct DataFileInfo {
    pub path: String,
    pub columns: Vec<String>,
    pub rows: u32,
    /// Why the file can't be used (missing, unreadable, invalid).
    pub error: Option<String>,
}

impl DataTable {
    /// The value of column `name` (any case) in row `row` (from 0); empty if
    /// that row ends before it.
    pub fn value(&self, row: usize, name: &str) -> Option<&str> {
        let col = self.column(name)?;
        Some(self.rows.get(row)?.get(col).map_or("", String::as_str))
    }

    fn column(&self, name: &str) -> Option<usize> {
        let name = name.trim();
        self.columns.iter().position(|c| same_name(c, name))
    }
}

/// Reads a data file's bytes.
pub fn parse(bytes: &[u8]) -> Result<DataTable, DataError> {
    let text = decode(bytes);
    let sep = separator(&text);
    let mut records = records(&text, sep)?.into_iter().filter(|r| r.iter().any(|v| !v.is_empty()));
    let header = records.next().ok_or(DataError::Empty)?;
    let columns: Vec<String> = header.into_iter().map(|c| c.trim().to_string()).collect();
    for (i, c) in columns.iter().enumerate() {
        if !c.is_empty() && columns[..i].iter().any(|d| same_name(d, c)) {
            return Err(DataError::SameName(c.clone()));
        }
    }
    let rows = records
        .enumerate()
        .map(|(i, values)| match values.len() > columns.len() {
            true => Err(DataError::TooManyValues { row: i + 1, values: values.len(), columns: columns.len() }),
            false => Ok(values),
        })
        .collect::<Result<_, _>>()?;
    Ok(DataTable { columns, rows })
}

fn decode(bytes: &[u8]) -> String {
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
    match std::str::from_utf8(bytes) {
        Ok(s) => s.to_string(),
        Err(_) => bytes.iter().map(|&b| windows_1252(b)).collect(),
    }
}

fn windows_1252(b: u8) -> char {
    const HIGH: [char; 32] = [
        '€', '\u{81}', '‚', 'ƒ', '„', '…', '†', '‡', 'ˆ', '‰', 'Š', '‹', 'Œ', '\u{8D}', 'Ž', '\u{8F}', '\u{90}', '‘',
        '’', '“', '”', '•', '–', '—', '˜', '™', 'š', '›', 'œ', '\u{9D}', 'ž', 'Ÿ',
    ];
    match b {
        0x80..=0x9F => HIGH[(b - 0x80) as usize],
        _ => b as char,
    }
}

/// The separator the header uses most: comma, semicolon or tab.
fn separator(text: &str) -> char {
    let mut counts = [(',', 0), (';', 0), ('\t', 0)];
    let mut quoted = false;
    for c in text.chars() {
        match c {
            '"' => quoted = !quoted,
            '\n' if !quoted => break,
            _ if !quoted => counts.iter_mut().filter(|(s, _)| *s == c).for_each(|(_, n)| *n += 1),
            _ => {}
        }
    }
    counts.iter().fold((',', 0), |best, &(s, n)| if n > best.1 { (s, n) } else { best }).0
}

fn same_name(a: &str, b: &str) -> bool {
    a.to_lowercase() == b.to_lowercase()
}

/// Every record; a quote left open is reported with its row (0 is the header).
fn records(text: &str, sep: char) -> Result<Vec<Vec<String>>, DataError> {
    let mut out = Vec::new();
    let mut record = Vec::new();
    let mut value = String::new();
    let mut chars = text.chars().peekable();
    let (mut quoted, mut started) = (false, false);
    while let Some(c) = chars.next() {
        started = true;
        if quoted {
            match c {
                '"' if chars.peek() == Some(&'"') => {
                    chars.next();
                    value.push('"');
                }
                '"' => quoted = false,
                c => value.push(c),
            }
            continue;
        }
        match c {
            '"' => quoted = true,
            '\r' if chars.peek() == Some(&'\n') => {}
            '\n' | '\r' => {
                record.push(std::mem::take(&mut value));
                out.push(std::mem::take(&mut record));
                started = false;
            }
            c if c == sep => record.push(std::mem::take(&mut value)),
            c => value.push(c),
        }
    }
    if quoted {
        return Err(DataError::Unclosed(out.len()));
    }
    if started {
        record.push(value);
        out.push(record);
    }
    Ok(out)
}

/// The columns a macro's Text steps type, in order, each once.
pub fn columns_used(events: &[Event]) -> Vec<String> {
    let mut used: Vec<String> = Vec::new();
    for e in events {
        let Event::Text { text, .. } = e else { continue };
        for part in text::parse(text).unwrap_or_default() {
            if let Part::Column(name) = part
                && !used.iter().any(|u| same_name(u, &name))
            {
                used.push(name);
            }
        }
    }
    used
}

/// Why a macro can't play with `table` (its data file, `None` if it has
/// none, named `file`): a column its Text steps type is missing, or there
/// are no rows to play.
pub fn check(events: &[Event], table: Option<&DataTable>, file: &str) -> Result<(), String> {
    let used = columns_used(events);
    let Some(table) = table else {
        return match used.first() {
            Some(name) => Err(format!("A Text step types {{col:{name}}}: choose a data file in Settings → Playback.")),
            None => Ok(()),
        };
    };
    if let Some(name) = used.iter().find(|name| table.column(name).is_none()) {
        return Err(format!("{file} has no column “{name}”."));
    }
    if table.rows.is_empty() {
        return Err(format!("{file} has no rows to play."));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table(columns: &[&str], rows: &[&[&str]]) -> DataTable {
        let strings = |v: &[&str]| v.iter().map(|s| s.to_string()).collect();
        DataTable { columns: strings(columns), rows: rows.iter().map(|r| strings(r)).collect() }
    }

    #[test]
    fn a_csv_is_read_with_its_header() {
        let t = parse(b"Customer,Amount\nACME,12.50\nGlobex,7\n").unwrap();
        assert_eq!(t, table(&["Customer", "Amount"], &[&["ACME", "12.50"], &["Globex", "7"]]));
        assert_eq!(parse(b"a,b\r\n1,2\r\n3,4").unwrap(), table(&["a", "b"], &[&["1", "2"], &["3", "4"]]));
    }

    #[test]
    fn the_separator_is_the_one_the_header_uses() {
        assert_eq!(parse(b"a;b\n1,5;2").unwrap(), table(&["a", "b"], &[&["1,5", "2"]]), "Excel in French");
        assert_eq!(parse(b"a\tb\n1\t2").unwrap(), table(&["a", "b"], &[&["1", "2"]]));
        assert_eq!(parse(b"\"x;y\",b\n1,2").unwrap(), table(&["x;y", "b"], &[&["1", "2"]]), "quoted ones don't count");
        assert_eq!(parse(b"only\nvalue").unwrap(), table(&["only"], &[&["value"]]));
    }

    #[test]
    fn quotes_hold_separators_quotes_and_lines() {
        let t = parse(b"name,note\n\"Smith, J.\",\"said \"\"hi\"\"\nthen left\"\n").unwrap();
        assert_eq!(t.rows, [["Smith, J.", "said \"hi\"\nthen left"]]);
        assert_eq!(parse(b"a\n\"open"), Err(DataError::Unclosed(1)));
    }

    #[test]
    fn blank_lines_are_skipped_and_short_rows_are_empty_at_the_end() {
        let t = parse(b"\na,b,c\n\n1,2\n,,\n4,5,6\n\n").unwrap();
        assert_eq!(t.rows, [vec!["1", "2"], vec!["4", "5", "6"]]);
        assert_eq!(t.value(0, "c"), Some(""));
        assert_eq!(t.value(1, "C "), Some("6"), "any case, trimmed");
        assert_eq!(t.value(0, "d"), None);
        assert_eq!(t.value(2, "a"), None);
    }

    #[test]
    fn the_encoding_is_utf8_or_windows_1252() {
        assert_eq!(parse("\u{FEFF}Prénom\nZoë".as_bytes()).unwrap(), table(&["Prénom"], &[&["Zoë"]]));
        assert_eq!(parse(b"Pr\xE9nom\n\x80 5").unwrap(), table(&["Prénom"], &[&["€ 5"]]));
    }

    #[test]
    fn mistakes_are_refused_with_where_they_are() {
        assert_eq!(parse(b""), Err(DataError::Empty));
        assert_eq!(parse(b"\n\n"), Err(DataError::Empty));
        assert_eq!(parse(b"a,b\n1,2\n1,2,3"), Err(DataError::TooManyValues { row: 2, values: 3, columns: 2 }));
        assert_eq!(parse(b"Name,name"), Err(DataError::SameName("name".into())));
        assert_eq!(parse(b"a,\n1,2").unwrap().columns, ["a", ""], "unnamed columns are allowed");
        assert_eq!(
            DataError::TooManyValues { row: 2, values: 3, columns: 2 }.to_string(),
            "Row 2 has 3 values, but there are 2 columns."
        );
        assert_eq!(DataError::Unclosed(4).to_string(), "A quote isn't closed in row 4.");
    }

    fn typing(texts: &[&str]) -> Vec<Event> {
        texts.iter().enumerate().map(|(i, s)| Event::Text { t: i as u32 * 100, dur: 50, text: s.to_string() }).collect()
    }

    #[test]
    fn the_columns_used_are_listed_once() {
        let events = typing(&["{col:Customer} {n}", "{col:customer}{col:Amount}", "{col:Broken", "plain"]);
        assert_eq!(columns_used(&events), ["Customer", "Amount"]);
    }

    #[test]
    fn a_macro_plays_only_with_the_columns_it_types() {
        let t = table(&["Customer"], &[&["ACME"]]);
        assert_eq!(check(&typing(&["{n}"]), None, ""), Ok(()), "no columns, no file needed");
        assert_eq!(check(&typing(&["{col:CUSTOMER}"]), Some(&t), "data.csv"), Ok(()));
        assert_eq!(
            check(&typing(&["{col:Customer}"]), None, ""),
            Err("A Text step types {col:Customer}: choose a data file in Settings → Playback.".into())
        );
        assert_eq!(
            check(&typing(&["{col:Customer} {col:Total}"]), Some(&t), "data.csv"),
            Err("data.csv has no column “Total”.".into())
        );
        let empty = table(&["Customer"], &[]);
        assert_eq!(check(&typing(&["{n}"]), Some(&empty), "data.csv"), Err("data.csv has no rows to play.".into()));
    }
}
