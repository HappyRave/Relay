//! What an edit changed in a macro's events, for undo: the run of events it
//! replaced, and how far it moved the events after them. Most edits change a
//! few events and shift the rest by one amount (inserting a wait, deleting a
//! step, a pause), so this is far smaller than a copy of every event.

use crate::model::Event;

#[derive(Debug, Clone, PartialEq)]
pub struct Splice {
    /// Where the change starts.
    at: usize,
    /// The events that were there before.
    removed: Vec<Event>,
    /// How many events are there now instead.
    inserted: usize,
    /// How far (ms) the events after them moved.
    shift: i64,
}

impl Splice {
    /// The change from `before` to `after`.
    pub fn between(before: &[Event], after: &[Event]) -> Splice {
        let prefix = before.iter().zip(after).take_while(|(b, a)| b == a).count();
        let (b, a) = (&before[prefix..], &after[prefix..]);
        let shift = match (b.last(), a.last()) {
            (Some(x), Some(y)) => i64::from(y.t()) - i64::from(x.t()),
            _ => 0,
        };
        let mut suffix =
            b.iter().rev().zip(a.iter().rev()).take_while(|(x, y)| shifted(x, shift).as_ref() == Some(y)).count();
        let shift = if suffix == 0 { 0 } else { shift };
        // Without a shift, a plain common suffix may still be longer.
        if suffix == 0 {
            suffix = b.iter().rev().zip(a.iter().rev()).take_while(|(x, y)| x == y).count();
        }
        Splice { at: prefix, removed: b[..b.len() - suffix].to_vec(), inserted: a.len() - suffix, shift }
    }

    /// How many events it keeps a copy of.
    pub fn kept(&self) -> usize {
        self.removed.len()
    }

    /// Whether nothing changed.
    pub fn is_empty(&self) -> bool {
        self.removed.is_empty() && self.inserted == 0 && self.shift == 0
    }

    /// Turns `events` (what the edit left) back into what it had before, and
    /// returns the splice that redoes it.
    pub fn revert(self, events: &mut Vec<Event>) -> Splice {
        let end = self.at + self.inserted;
        let taken: Vec<Event> = events.splice(self.at..end, self.removed.iter().cloned()).collect();
        for e in &mut events[self.at + self.removed.len()..] {
            *e = shifted(e, -self.shift).expect("the suffix moved by this shift");
        }
        Splice { at: self.at, inserted: self.removed.len(), removed: taken, shift: -self.shift }
    }
}

/// `e` moved by `shift` ms, if it stays at or after 0.
fn shifted(e: &Event, shift: i64) -> Option<Event> {
    let t = u32::try_from(i64::from(e.t()) + shift).ok()?;
    let mut e = e.clone();
    *e.t_mut() = t;
    Some(e)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A wait labeled with its time, so no two are alike.
    fn wait(t: u32, dur: u32) -> Event {
        Event::Wait { t, dur, label: format!("at {t}") }
    }

    fn round_trip(before: Vec<Event>, after: Vec<Event>) -> Splice {
        let splice = Splice::between(&before, &after);
        let mut events = after.clone();
        let redo = splice.clone().revert(&mut events);
        assert_eq!(events, before, "undo");
        redo.revert(&mut events);
        assert_eq!(events, after, "redo");
        splice
    }

    #[test]
    fn a_change_in_the_middle_keeps_only_that() {
        let before: Vec<Event> = (0..10).map(|i| wait(i * 1000, 100)).collect();
        let mut after = before.clone();
        after[4] = Event::Wait { t: 4000, dur: 100, label: "x".into() };
        let s = round_trip(before, after);
        assert_eq!((s.at, s.removed.len(), s.inserted, s.shift), (4, 1, 1, 0));
    }

    #[test]
    fn an_insert_that_moves_the_rest_keeps_only_the_shift() {
        let before: Vec<Event> = (0..10).map(|i| wait(i * 1000, 100)).collect();
        let mut after = before[..3].to_vec();
        after.push(wait(3000, 500));
        after.extend(before[3..].iter().map(|e| shifted(e, 500).unwrap()));
        let s = round_trip(before, after);
        assert_eq!((s.at, s.removed.len(), s.inserted, s.shift), (3, 0, 1, 500));
    }

    #[test]
    fn a_delete_that_moves_the_rest_back() {
        let before: Vec<Event> = (0..10).map(|i| wait(i * 1000, 100)).collect();
        let mut after = before[..2].to_vec();
        after.extend(before[3..].iter().map(|e| shifted(e, -1000).unwrap()));
        let s = round_trip(before, after);
        assert_eq!((s.at, s.removed.len(), s.inserted, s.shift), (2, 1, 0, -1000));
    }

    #[test]
    fn nothing_and_everything() {
        let events: Vec<Event> = (0..3).map(|i| wait(i * 1000, 100)).collect();
        assert!(round_trip(events.clone(), events.clone()).is_empty());
        let s = round_trip(events.clone(), Vec::new());
        assert_eq!((s.removed.len(), s.inserted), (3, 0));
        round_trip(Vec::new(), events.clone());
        let other: Vec<Event> = (0..3).map(|i| wait(i * 700, 50)).collect();
        round_trip(events, other);
    }

    #[test]
    fn a_shift_that_would_go_below_zero_is_not_a_shift() {
        let before = vec![wait(0, 100), wait(50, 100)];
        let after = vec![wait(10, 100), wait(0, 100)];
        round_trip(before, after);
    }
}
