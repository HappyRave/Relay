//! Weekly schedules: "Mon–Fri at 09:00" in the user's local time zone.

use chrono::{DateTime, Datelike, Days, LocalResult, NaiveDateTime, NaiveTime, TimeDelta, TimeZone};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct WeeklySchedule {
    /// Monday first.
    pub days: [bool; 7],
    /// Local wall-clock time, serialized as "HH:MM".
    #[serde(serialize_with = "ser_hm", deserialize_with = "de_hm")]
    #[ts(type = "string")]
    pub time: NaiveTime,
}

impl Default for WeeklySchedule {
    /// Weekdays at 09:00.
    fn default() -> Self {
        WeeklySchedule { days: [true, true, true, true, true, false, false], time: NaiveTime::from_hms_opt(9, 0, 0).unwrap() }
    }
}

fn ser_hm<S: Serializer>(t: &NaiveTime, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(&t.format("%H:%M").to_string())
}

fn de_hm<'de, D: Deserializer<'de>>(d: D) -> Result<NaiveTime, D::Error> {
    let s = String::deserialize(d)?;
    NaiveTime::parse_from_str(&s, "%H:%M").map_err(serde::de::Error::custom)
}

/// The first scheduled instant strictly after `now`, or `None` when no day is
/// selected. A time skipped by a DST jump runs at the first valid instant after
/// the gap; a time that occurs twice runs at the earlier one.
pub fn next_run<Tz: TimeZone>(now: &DateTime<Tz>, schedule: &WeeklySchedule) -> Option<DateTime<Tz>> {
    let tz = now.timezone();
    let today = now.date_naive();
    (0..=7u64).find_map(|add| {
        let date = today.checked_add_days(Days::new(add))?;
        if !schedule.days[date.weekday().num_days_from_monday() as usize] {
            return None;
        }
        resolve(&tz, date.and_time(schedule.time)).filter(|t| t > now)
    })
}

fn resolve<Tz: TimeZone>(tz: &Tz, local: NaiveDateTime) -> Option<DateTime<Tz>> {
    match tz.from_local_datetime(&local) {
        LocalResult::Single(t) => Some(t),
        LocalResult::Ambiguous(earliest, _) => Some(earliest),
        LocalResult::None => {
            // Inside a DST gap: walk forward to the first local minute that exists.
            (1..=240).find_map(|m| tz.from_local_datetime(&(local + TimeDelta::minutes(m))).earliest())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use chrono_tz::Europe::Brussels;

    fn sched(days: [bool; 7], hm: &str) -> WeeklySchedule {
        WeeklySchedule { days, time: NaiveTime::parse_from_str(hm, "%H:%M").unwrap() }
    }
    fn at(y: i32, m: u32, d: u32, h: u32, min: u32) -> DateTime<chrono_tz::Tz> {
        Brussels.from_local_datetime(&NaiveDate::from_ymd_opt(y, m, d).unwrap().and_hms_opt(h, min, 0).unwrap()).unwrap()
    }
    const WEEKDAYS: [bool; 7] = [true, true, true, true, true, false, false];

    #[test]
    fn later_today_or_next_selected_day() {
        // 2026-09-24 is a Thursday.
        let s = sched(WEEKDAYS, "09:00");
        assert_eq!(next_run(&at(2026, 9, 24, 8, 0), &s), Some(at(2026, 9, 24, 9, 0)));
        assert_eq!(next_run(&at(2026, 9, 24, 9, 0), &s), Some(at(2026, 9, 25, 9, 0)));
        // Friday evening skips the weekend.
        assert_eq!(next_run(&at(2026, 9, 25, 18, 0), &s), Some(at(2026, 9, 28, 9, 0)));
        // A single day a week, already passed today: same weekday next week.
        let thu = sched([false, false, false, true, false, false, false], "07:30");
        assert_eq!(next_run(&at(2026, 9, 24, 8, 0), &thu), Some(at(2026, 10, 1, 7, 30)));
        assert_eq!(next_run(&at(2026, 9, 24, 8, 0), &sched([false; 7], "07:30")), None);
    }

    #[test]
    fn dst_gap_runs_right_after_the_jump() {
        // Brussels skips 02:00–03:00 on Sunday 2026-03-29.
        let sun = sched([false, false, false, false, false, false, true], "02:30");
        let next = next_run(&at(2026, 3, 28, 12, 0), &sun).unwrap();
        assert_eq!(next, at(2026, 3, 29, 3, 0));
    }

    #[test]
    fn dst_overlap_runs_once_at_the_earlier_time() {
        // 02:30 happens twice on Sunday 2026-10-25.
        let sun = sched([false, false, false, false, false, false, true], "02:30");
        let first = next_run(&at(2026, 10, 24, 12, 0), &sun).unwrap();
        assert_eq!(first.to_rfc3339(), "2026-10-25T02:30:00+02:00");
        let after = next_run(&first, &sun).unwrap();
        assert_eq!(after.date_naive(), NaiveDate::from_ymd_opt(2026, 11, 1).unwrap());
    }

    #[test]
    fn serializes_time_as_hours_and_minutes() {
        let s = sched(WEEKDAYS, "09:05");
        let json = serde_json::to_string(&s).unwrap();
        assert!(json.contains(r#""time":"09:05""#), "{json}");
        assert_eq!(serde_json::from_str::<WeeklySchedule>(&json).unwrap(), s);
    }
}
