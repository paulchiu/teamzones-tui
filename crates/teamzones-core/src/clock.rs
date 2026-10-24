//! The time source and the 15-minute grid every evaluation runs on.

use chrono::{DateTime, Utc};

/// Where "now" comes from: the system clock, or one fixed instant for tests and `--now`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Clock {
    System,
    Fixed(DateTime<Utc>),
}

impl Clock {
    /// The current instant according to this clock.
    pub fn now(self) -> DateTime<Utc> {
        match self {
            Self::System => Utc::now(),
            Self::Fixed(instant) => instant,
        }
    }
}

const SLOT_SECONDS: i64 = 15 * 60;

/// A 15-minute slot aligned to UTC quarter hours, held as its index since the Unix epoch.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Slot(i64);

impl Slot {
    /// The slot whose span holds `instant`.
    pub fn containing(instant: DateTime<Utc>) -> Self {
        Self(instant.timestamp().div_euclid(SLOT_SECONDS))
    }

    /// The first slot that starts strictly after `instant`.
    pub fn first_after(instant: DateTime<Utc>) -> Self {
        Self::containing(instant).next()
    }

    /// The first slot that starts at or after `instant`.
    pub fn starting_from(instant: DateTime<Utc>) -> Self {
        let slot = Self::containing(instant);
        if slot.start() == instant {
            slot
        } else {
            slot.next()
        }
    }

    /// The instant this slot starts.
    pub fn start(self) -> DateTime<Utc> {
        DateTime::from_timestamp(self.0 * SLOT_SECONDS, 0)
            .expect("slots are only built from instants chrono can represent, plus a few days")
    }

    /// The slot after this one.
    #[must_use]
    pub fn next(self) -> Self {
        self.offset(1)
    }

    /// The slot `count` slots later (or earlier, when negative).
    #[must_use]
    pub fn offset(self, count: i64) -> Self {
        Self(self.0 + count)
    }

    /// How many slots `self` is after `earlier`.
    pub fn since(self, earlier: Self) -> i64 {
        self.0 - earlier.0
    }
}

/// A non-empty, half-open run of slots, `[start, end)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Range {
    start: Slot,
    end: Slot,
}

impl Range {
    /// The range from `start` up to but not including `end`, or `None` when it would be empty.
    pub fn new(start: Slot, end: Slot) -> Option<Self> {
        (start < end).then_some(Self { start, end })
    }

    /// The range of just this one slot.
    pub fn single(slot: Slot) -> Self {
        Self {
            start: slot,
            end: slot.next(),
        }
    }

    /// The smallest range covering `[start, end)`, snapping the start down and the end up to the grid.
    pub fn covering(start: DateTime<Utc>, end: DateTime<Utc>) -> Option<Self> {
        Self::new(Slot::containing(start), Slot::starting_from(end))
    }

    pub fn start(self) -> Slot {
        self.start
    }

    pub fn end(self) -> Slot {
        self.end
    }

    /// Every slot in the range, in order.
    pub fn slots(self) -> impl Iterator<Item = Slot> {
        (self.start.0..self.end.0).map(Slot)
    }
}

/// What a line or an answer is about: one instant, whose slot sets every state, or a range.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum When {
    At(DateTime<Utc>),
    Over(Range),
}

impl When {
    /// The slots whose states this covers.
    pub fn slots(self) -> Vec<Slot> {
        match self {
            Self::At(instant) => vec![Slot::containing(instant)],
            Self::Over(range) => range.slots().collect(),
        }
    }

    /// The instant labels start from: the exact instant, or the range's first slot.
    pub fn start(self) -> DateTime<Utc> {
        match self {
            Self::At(instant) => instant,
            Self::Over(range) => range.start().start(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn utc(text: &str) -> DateTime<Utc> {
        text.parse().unwrap()
    }

    #[test]
    fn a_slot_is_aligned_to_utc_quarter_hours() {
        assert_eq!(
            Slot::containing(utc("2026-10-23T22:04:00Z")).start(),
            utc("2026-10-23T22:00:00Z")
        );
    }

    #[test]
    fn the_first_slot_after_now_is_the_next_quarter_hour() {
        assert_eq!(
            Slot::first_after(utc("2026-10-23T22:04:00Z")).start(),
            utc("2026-10-23T22:15:00Z")
        );
        assert_eq!(
            Slot::first_after(utc("2026-10-23T22:15:00Z")).start(),
            utc("2026-10-23T22:30:00Z")
        );
    }

    #[test]
    fn a_range_is_half_open_and_never_empty() {
        let start = Slot::containing(utc("2026-10-26T06:00:00Z"));
        let range = Range::new(start, start.offset(2)).unwrap();
        let starts: Vec<_> = range.slots().map(Slot::start).collect();
        assert_eq!(
            starts,
            [utc("2026-10-26T06:00:00Z"), utc("2026-10-26T06:15:00Z")]
        );
        assert_eq!(Range::new(start, start), None);
    }

    #[test]
    fn covering_snaps_the_start_down_and_the_end_up() {
        let range =
            Range::covering(utc("2026-10-26T06:10:00Z"), utc("2026-10-26T06:20:00Z")).unwrap();
        assert_eq!(range.start().start(), utc("2026-10-26T06:00:00Z"));
        assert_eq!(range.end().start(), utc("2026-10-26T06:30:00Z"));
    }

    #[test]
    fn an_instant_takes_the_state_of_the_slot_holding_it() {
        let when = When::At(utc("2026-10-23T22:04:00Z"));
        assert_eq!(
            when.slots(),
            [Slot::containing(utc("2026-10-23T22:00:00Z"))]
        );
        assert_eq!(when.start(), utc("2026-10-23T22:04:00Z"));
    }
}
