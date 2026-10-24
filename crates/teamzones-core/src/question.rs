//! Marks, the question they ask, and whether each counted row qualifies.

use std::collections::BTreeSet;

use chrono_tz::Tz;

use crate::clock::{Range, Slot, When};
use crate::config::{Group, PersonId, PlaceId, Roster, Rule};
use crate::schedule::{Band, Level, band, level};

/// A focusable row: a person or a place, by stable id.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Row {
    Person(PersonId),
    Place(PlaceId),
}

/// A `+@city` clock: its zone joins the say line, but it is never counted.
#[derive(Clone, Debug, PartialEq)]
pub struct CityClock {
    pub name: String,
    pub tz: Tz,
}

/// The working set of counted rows, plus the places' rule and any extra clocks.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Marks {
    pub people: BTreeSet<PersonId>,
    pub places: BTreeSet<PlaceId>,
    pub clocks: Vec<CityClock>,
    pub rule: Rule,
}

/// What the marks ask.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Question {
    Who,
    Where,
    Both,
}

impl Marks {
    /// The marks a group sets: its members and its rule.
    pub fn of_group(group: &Group) -> Self {
        Self {
            people: group.people.iter().copied().collect(),
            places: group.places.iter().copied().collect(),
            clocks: Vec::new(),
            rule: group.rule,
        }
    }

    /// Every person and place, as `0 all` marks them.
    pub fn all(roster: &Roster) -> Self {
        Self {
            people: roster.people().map(|(id, _)| id).collect(),
            places: roster.places().map(|(id, _)| id).collect(),
            clocks: Vec::new(),
            rule: Rule::OffPeak,
        }
    }

    /// The question these marks ask, or `None` when they ask nothing.
    pub fn question(&self) -> Option<Question> {
        match (self.asks_who(), self.asks_where()) {
            (true, true) => Some(Question::Both),
            (true, false) => Some(Question::Who),
            (false, true) => Some(Question::Where),
            (false, false) => None,
        }
    }

    pub fn asks_who(&self) -> bool {
        !self.people.is_empty()
    }

    pub fn asks_where(&self) -> bool {
        !self.places.is_empty() && self.rule != Rule::Context
    }

    /// The counted people: home whenever the marks ask who, then the rest in config order.
    pub fn counted_people(&self) -> Vec<PersonId> {
        if !self.asks_who() {
            return Vec::new();
        }
        std::iter::once(PersonId::HOME)
            .chain(
                self.people
                    .iter()
                    .copied()
                    .filter(|id| *id != PersonId::HOME),
            )
            .collect()
    }

    /// The counted places, in config order: none when the rule is context.
    pub fn counted_places(&self) -> Vec<PlaceId> {
        if self.asks_where() {
            self.places.iter().copied().collect()
        } else {
            Vec::new()
        }
    }
}

/// A person's band at a slot.
pub fn person_band(roster: &Roster, id: PersonId, slot: Slot) -> Band {
    let person = roster.person(id);
    band(person.location, person.hours, roster.daylight, slot)
}

/// A place's level at a slot, or `None` for a place without a trading profile.
pub fn place_level(roster: &Roster, id: PlaceId, slot: Slot) -> Option<Level> {
    let place = roster.place(id);
    place
        .trading
        .as_ref()
        .map(|trading| level(trading, place.location.tz, slot))
}

/// A place's range state: its worst level over the slots of `when`.
pub fn place_state(roster: &Roster, id: PlaceId, when: When) -> Option<Level> {
    when.slots()
        .into_iter()
        .map(|slot| place_level(roster, id, slot))
        .try_fold(Level::Quiet, |worst, level| level.map(|l| worst.max(l)))
}

/// Whether a counted person qualifies at a slot: working, or in daylight without hours.
pub fn person_qualifies(roster: &Roster, id: PersonId, slot: Slot) -> bool {
    match person_band(roster, id, slot) {
        Band::Working | Band::Day => true,
        Band::Night | Band::Morning | Band::Evening | Band::Off => false,
    }
}

/// Whether a counted place qualifies at a slot under `rule`; a place without trading never does.
pub fn place_qualifies(roster: &Roster, id: PlaceId, rule: Rule, slot: Slot) -> bool {
    place_level(roster, id, slot).is_some_and(|level| rule.admits(level))
}

/// How many counted rows qualify, out of how many are counted.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Score {
    pub qualifying: usize,
    pub counted: usize,
}

impl std::fmt::Display for Score {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.qualifying, self.counted)
    }
}

/// The who block's rule, named by whether the counted people have hours.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WhoRule {
    InHours,
    Daytime,
    HoursOrDay,
}

impl WhoRule {
    pub fn label(self) -> &'static str {
        match self {
            Self::InHours => "in hours",
            Self::Daytime => "daytime",
            Self::HoursOrDay => "hours or day",
        }
    }
}

/// One asked block's answer over a range: its rule and how many counted rows qualify throughout.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BlockAnswer<R> {
    pub rule: R,
    pub score: Score,
}

/// The answer rows: each block's answer, or `None` when that block is not asked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Answer {
    pub who: Option<BlockAnswer<WhoRule>>,
    pub place: Option<BlockAnswer<Rule>>,
}

fn score<T: Copy>(rows: &[T], slots: &[Slot], qualifies: impl Fn(T, Slot) -> bool) -> Score {
    let qualifying = rows
        .iter()
        .filter(|row| slots.iter().all(|slot| qualifies(**row, *slot)))
        .count();
    Score {
        qualifying,
        counted: rows.len(),
    }
}

/// Answers the marks' question over `when`: per block, the rows that qualify in every slot.
pub fn evaluate(roster: &Roster, marks: &Marks, when: When) -> Answer {
    let slots = when.slots();
    let people = marks.counted_people();
    let places = marks.counted_places();
    let with_hours = people
        .iter()
        .filter(|id| roster.person(**id).hours.is_some())
        .count();
    let who_rule = match with_hours {
        0 => WhoRule::Daytime,
        n if n == people.len() => WhoRule::InHours,
        _ => WhoRule::HoursOrDay,
    };
    Answer {
        who: marks.asks_who().then(|| BlockAnswer {
            rule: who_rule,
            score: score(&people, &slots, |id, slot| {
                person_qualifies(roster, id, slot)
            }),
        }),
        place: marks.asks_where().then(|| BlockAnswer {
            rule: marks.rule,
            score: score(&places, &slots, |id, slot| {
                place_qualifies(roster, id, marks.rule, slot)
            }),
        }),
    }
}

/// How many counted rows across both blocks qualify at one slot.
pub fn slot_score(roster: &Roster, marks: &Marks, slot: Slot) -> Score {
    let answer = evaluate(roster, marks, When::Over(Range::single(slot)));
    let blocks = [answer.who.map(|b| b.score), answer.place.map(|b| b.score)];
    blocks.into_iter().flatten().fold(
        Score {
            qualifying: 0,
            counted: 0,
        },
        |total, s| Score {
            qualifying: total.qualifying + s.qualifying,
            counted: total.counted + s.counted,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    const FIXTURE: &str = include_str!("../tests/fixtures/sample.toml");

    fn roster() -> Roster {
        Config::parse(FIXTURE).unwrap().roster
    }

    fn marks(roster: &Roster, group: &str) -> Marks {
        Marks::of_group(roster.group_named(group).unwrap())
    }

    fn range(from: &str, to: &str) -> When {
        When::Over(
            Range::new(
                Slot::containing(from.parse().unwrap()),
                Slot::containing(to.parse().unwrap()),
            )
            .unwrap(),
        )
    }

    #[test]
    fn a_group_asks_who_for_people_and_where_for_places() {
        let roster = roster();
        assert_eq!(marks(&roster, "team").question(), Some(Question::Who));
        assert_eq!(marks(&roster, "regions").question(), Some(Question::Where));
        assert_eq!(marks(&roster, "deploy").question(), Some(Question::Both));
    }

    #[test]
    fn home_is_counted_only_when_the_marks_ask_who() {
        let roster = roster();
        assert_eq!(marks(&roster, "au").counted_people()[0], PersonId::HOME);
        assert!(marks(&roster, "regions").counted_people().is_empty());
    }

    #[test]
    fn deploy_over_mon_5_to_6pm_has_everyone_in_hours_and_only_uk_off_peak() {
        let roster = roster();
        // Mon 26 Oct 5-6pm AEDT is 06:00-07:00 UTC; AU's dinner peak starts at 5:30pm.
        let answer = evaluate(
            &roster,
            &marks(&roster, "deploy"),
            range("2026-10-26T06:00:00Z", "2026-10-26T07:00:00Z"),
        );
        assert_eq!(
            answer.who,
            Some(BlockAnswer {
                rule: WhoRule::InHours,
                score: Score {
                    qualifying: 3,
                    counted: 3
                }
            })
        );
        assert_eq!(
            answer.place,
            Some(BlockAnswer {
                rule: Rule::OffPeak,
                score: Score {
                    qualifying: 1,
                    counted: 2
                }
            })
        );
    }

    #[test]
    fn the_who_rule_is_daytime_without_hours_and_mixed_otherwise() {
        let roster = roster();
        let ellie = roster.person_named("Ellie").unwrap();
        let mixed = Marks {
            people: [ellie].into(),
            ..Marks::default()
        };
        let when = range("2026-10-26T06:00:00Z", "2026-10-26T07:00:00Z");
        assert_eq!(
            evaluate(&roster, &mixed, when).who.unwrap().rule,
            WhoRule::HoursOrDay
        );
        let home_without_hours =
            FIXTURE.replacen("hours = \"9am-6pm\"\ndays  = \"Mon-Fri\"\n", "", 1);
        let roster = Config::parse(&home_without_hours).unwrap().roster;
        assert_eq!(roster.person(PersonId::HOME).hours, None);
        assert_eq!(
            evaluate(&roster, &mixed, when).who.unwrap().rule,
            WhoRule::Daytime
        );
    }
}
