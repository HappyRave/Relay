//! What a Text step types: text with placeholders, filled in when it plays.
//! `{date}`, `{time}`, `{clipboard}` and `{n}` (the repeat number); `{{` and
//! `}}` type a brace.

use chrono::NaiveDateTime;
use thiserror::Error;

use crate::model::Ms;

/// How long typing one character takes.
pub const CHAR_MS: Ms = 10;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Part {
    Literal(String),
    Date,
    Time,
    Clipboard,
    Repeat,
}

#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum TemplateError {
    #[error("{{{0}}} isn't a placeholder: use {{date}}, {{time}}, {{clipboard}} or {{n}}.")]
    Unknown(String),
    #[error("A {{ isn't closed: type {{{{ for a brace.")]
    Unclosed,
    #[error("A }} has no {{ before it: type }}}} for a brace.")]
    Unopened,
}

/// Splits a template into text and placeholders.
pub fn parse(template: &str) -> Result<Vec<Part>, TemplateError> {
    let mut parts = Vec::new();
    let mut literal = String::new();
    let mut chars = template.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '{' if chars.peek() == Some(&'{') => {
                chars.next();
                literal.push('{');
            }
            '}' if chars.peek() == Some(&'}') => {
                chars.next();
                literal.push('}');
            }
            '}' => return Err(TemplateError::Unopened),
            '{' => {
                let mut name = String::new();
                loop {
                    match chars.next() {
                        Some('}') => break,
                        Some('{') | None => return Err(TemplateError::Unclosed),
                        Some(c) => name.push(c),
                    }
                }
                let part = match name.as_str() {
                    "date" => Part::Date,
                    "time" => Part::Time,
                    "clipboard" => Part::Clipboard,
                    "n" => Part::Repeat,
                    _ => return Err(TemplateError::Unknown(name)),
                };
                if !literal.is_empty() {
                    parts.push(Part::Literal(std::mem::take(&mut literal)));
                }
                parts.push(part);
            }
            c => literal.push(c),
        }
    }
    if !literal.is_empty() {
        parts.push(Part::Literal(literal));
    }
    Ok(parts)
}

/// Checks a template, for edits.
pub fn validate(template: &str) -> Result<(), TemplateError> {
    parse(template).map(|_| ())
}

/// The text `template` types on repeat `n` (from 1) at local time `now`.
/// The clipboard is read only if the template uses it (`None`: no text on it,
/// which types nothing). A template that doesn't parse (a hand-edited file)
/// is typed as it is.
pub fn fill(template: &str, n: u32, now: NaiveDateTime, clipboard: impl FnOnce() -> Option<String>) -> String {
    let Ok(parts) = parse(template) else { return template.to_string() };
    let mut clipboard = Some(clipboard);
    let mut copied = String::new();
    let mut out = String::new();
    for part in parts {
        match part {
            Part::Literal(s) => out.push_str(&s),
            Part::Date => out.push_str(&now.format("%Y-%m-%d").to_string()),
            Part::Time => out.push_str(&now.format("%H:%M:%S").to_string()),
            Part::Repeat => out.push_str(&n.to_string()),
            Part::Clipboard => {
                if let Some(read) = clipboard.take() {
                    copied = read().unwrap_or_default();
                }
                out.push_str(&copied);
            }
        }
    }
    out
}

/// About how long typing `template` takes: what's known of it (the
/// clipboard counts as empty). A Text step lasts at least this long.
pub fn typing_ms(template: &str) -> Ms {
    let chars = match parse(template) {
        Ok(parts) => parts
            .iter()
            .map(|p| match p {
                Part::Literal(s) => s.chars().count(),
                Part::Date => 10,
                Part::Time => 8,
                Part::Repeat => 1,
                Part::Clipboard => 0,
            })
            .sum(),
        Err(_) => template.chars().count(),
    };
    (chars as Ms).saturating_mul(CHAR_MS)
}

/// A template that types `text` as it is.
pub fn escape(text: &str) -> String {
    text.replace('{', "{{").replace('}', "}}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn at() -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2026, 10, 1).unwrap().and_hms_opt(9, 5, 7).unwrap()
    }

    #[test]
    fn placeholders_are_filled_in() {
        let filled = fill("Invoice {date} {time} #{n}: {clipboard}.", 3, at(), || Some("ACME".into()));
        assert_eq!(filled, "Invoice 2026-10-01 09:05:07 #3: ACME.");
    }

    #[test]
    fn double_braces_type_a_brace() {
        assert_eq!(fill("{{n}} = {n}, }}{{", 12, at(), || None), "{n} = 12, }{");
        assert_eq!(parse("{{}}").unwrap(), [Part::Literal("{}".into())]);
        assert_eq!(fill(&escape("a {b} }}"), 1, at(), || None), "a {b} }}");
    }

    #[test]
    fn the_clipboard_is_read_once_and_only_when_used() {
        let mut reads = 0;
        assert_eq!(
            fill("{clipboard}-{clipboard}", 1, at(), || {
                reads += 1;
                Some("x".into())
            }),
            "x-x"
        );
        assert_eq!(reads, 1);
        assert_eq!(fill("{date}", 1, at(), || panic!("not read")), "2026-10-01");
        assert_eq!(fill("[{clipboard}]", 1, at(), || None), "[]", "no text on the clipboard");
    }

    #[test]
    fn mistakes_are_refused_with_what_to_do() {
        assert_eq!(parse("{name}"), Err(TemplateError::Unknown("name".into())));
        assert_eq!(parse("{Date}"), Err(TemplateError::Unknown("Date".into())), "names are lowercase");
        assert_eq!(parse("{}"), Err(TemplateError::Unknown(String::new())));
        assert_eq!(parse("a {date"), Err(TemplateError::Unclosed));
        assert_eq!(parse("{da{te}"), Err(TemplateError::Unclosed));
        assert_eq!(parse("a } b"), Err(TemplateError::Unopened));
        assert_eq!(
            TemplateError::Unknown("name".into()).to_string(),
            "{name} isn't a placeholder: use {date}, {time}, {clipboard} or {n}."
        );
        assert_eq!(TemplateError::Unclosed.to_string(), "A { isn't closed: type {{ for a brace.");
        assert_eq!(TemplateError::Unopened.to_string(), "A } has no { before it: type }} for a brace.");
    }

    #[test]
    fn a_template_that_does_not_parse_is_typed_as_it_is() {
        assert_eq!(fill("50% {off", 1, at(), || None), "50% {off");
    }

    #[test]
    fn typing_time_counts_what_is_known() {
        assert_eq!(typing_ms(""), 0);
        assert_eq!(typing_ms("héllo ✓"), 70);
        assert_eq!(typing_ms("{date} {time}#{n}{clipboard}"), (10 + 1 + 8 + 1 + 1) * CHAR_MS);
        assert_eq!(typing_ms("{{x}}"), 3 * CHAR_MS);
        assert_eq!(typing_ms("{oops"), 5 * CHAR_MS);
    }
}
