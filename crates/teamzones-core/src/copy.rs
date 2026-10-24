//! The say line: zone entries in copy order, plus the places' clause.

use chrono::{DateTime, NaiveDate, Offset, Timelike, Utc};
use chrono_tz::Tz;

use crate::clock::When;
use crate::config::{Roster, Rule};
use crate::question::{Marks, Row, place_state};
use crate::schedule::Level;

/// What happens to the places' clause.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Clause {
    #[default]
    States,
    Zones,
    Drop,
}

/// How to shape a say line: `c d`, `c w` and one clause handling, which the CLI may combine.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Variant {
    pub date: bool,
    pub names: bool,
    pub clause: Clause,
}

/// A local time as `h[:mm]am|pm`, with minutes only when non-zero, so midnight is `12am`.
pub fn clock_text<T: chrono::TimeZone>(local: &DateTime<T>) -> String {
    format!("{}{}", hour_minute(local), suffix(local))
}

fn hour_minute<T: chrono::TimeZone>(local: &DateTime<T>) -> String {
    let (_, hour) = local.hour12();
    match local.minute() {
        0 => hour.to_string(),
        minute => format!("{hour}:{minute:02}"),
    }
}

fn suffix<T: chrono::TimeZone>(local: &DateTime<T>) -> &'static str {
    if local.hour12().0 { "pm" } else { "am" }
}

/// An instant as the axis reads it in a notice, such as `Mon 5pm AEDT`.
pub fn at_text(instant: DateTime<Utc>, axis: Tz) -> String {
    let local = instant.with_timezone(&axis);
    format!(
        "{} {} {}",
        local.format("%a"),
        clock_text(&local),
        local.format("%Z")
    )
}

/// A local range as `5-6pm` when both ends share a suffix, and `11am-1pm` when they do not.
pub fn span_text<T: chrono::TimeZone>(start: &DateTime<T>, end: &DateTime<T>) -> String {
    if suffix(start) == suffix(end) {
        format!("{}-{}", hour_minute(start), clock_text(end))
    } else {
        format!("{}-{}", clock_text(start), clock_text(end))
    }
}

/// The local start of `when` in `tz`, and its time or range text.
fn local_text(when: When, tz: Tz) -> (DateTime<Tz>, String) {
    match when {
        When::At(instant) => {
            let local = instant.with_timezone(&tz);
            (local, clock_text(&local))
        }
        When::Over(range) => {
            let (start, end) = (
                range.start().start().with_timezone(&tz),
                range.end().start().with_timezone(&tz),
            );
            (start, span_text(&start, &end))
        }
    }
}

/// One zone entry, such as `Sun 10pm-12am CDT`: a weekday only when its date is not the axis date.
fn entry(when: When, tz: Tz, axis_date: NaiveDate) -> String {
    let (start, text) = local_text(when, tz);
    let weekday = if start.date_naive() == axis_date {
        String::new()
    } else {
        format!("{} ", start.format("%a"))
    };
    format!("{weekday}{text} {}", start.format("%Z"))
}

fn offset_seconds(tz: Tz, instant: DateTime<Utc>) -> i32 {
    instant.with_timezone(&tz).offset().fix().local_minus_utc()
}

/// The zones a line names, axis first, each with the names of whoever brought it.
fn zone_sources(roster: &Roster, marks: &Marks, clause: Clause) -> Vec<(Tz, Vec<String>)> {
    let home = marks
        .asks_who()
        .then(|| roster.person(crate::config::PersonId::HOME).name.clone());
    let axis = (roster.axis(), home.into_iter().collect());
    let people = marks.counted_people().into_iter().skip(1).map(|id| {
        let person = roster.person(id);
        (person.location.tz, vec![person.name.clone()])
    });
    let clocks = marks
        .clocks
        .iter()
        .map(|clock| (clock.tz, vec![clock.name.clone()]));
    let places = match clause {
        Clause::Zones => marks.counted_places(),
        Clause::States | Clause::Drop => Vec::new(),
    };
    let places = places.into_iter().map(|id| {
        let place = roster.place(id);
        (place.location.tz, vec![place.name.clone()])
    });
    std::iter::once(axis)
        .chain(people)
        .chain(clocks)
        .chain(places)
        .collect()
}

/// `A`, `A and B`, or `A, B and C`.
fn and_list(names: &[&str]) -> String {
    match names {
        [] => String::new(),
        [only] => (*only).to_owned(),
        [init @ .., last] => format!("{} and {last}", init.join(", ")),
    }
}

/// The places' states, counting places first, as in `off-peak in UK, peak in AU`.
fn clause(roster: &Roster, marks: &Marks, when: When) -> Option<String> {
    let states: Vec<(&str, Level)> = marks
        .counted_places()
        .into_iter()
        .filter_map(|id| {
            place_state(roster, id, when).map(|level| (roster.place(id).name.as_str(), level))
        })
        .collect();
    let named = |wanted: &dyn Fn(Level) -> bool| {
        states
            .iter()
            .filter(|(_, level)| wanted(*level))
            .map(|(name, _)| *name)
            .collect::<Vec<_>>()
    };
    let counts = |level| marks.rule.admits(level);
    let groups = match marks.rule {
        Rule::OffPeak => vec![
            ("off-peak", named(&counts)),
            ("peak", named(&|l| !counts(l))),
        ],
        Rule::Quiet => vec![
            ("quiet", named(&counts)),
            ("peak", named(&|l| !counts(l) && l == Level::Peak)),
            ("open", named(&|l| !counts(l) && l == Level::OffPeak)),
        ],
        Rule::Context => Vec::new(),
    };
    let parts: Vec<String> = groups
        .into_iter()
        .filter(|(_, names)| !names.is_empty())
        .map(|(word, names)| format!("{word} in {}", and_list(&names)))
        .collect();
    (!parts.is_empty()).then(|| parts.join(", "))
}

/// The say line for the marks over `when`, shaped by `variant`.
pub fn say_line(roster: &Roster, marks: &Marks, when: When, variant: Variant) -> String {
    let Variant {
        date,
        names,
        clause: clause_mode,
    } = variant;
    let start = when.start();
    let axis_date = start.with_timezone(&roster.axis()).date_naive();
    let mut sources = zone_sources(roster, marks, clause_mode);
    sources[1..].sort_by_key(|(tz, _)| -offset_seconds(*tz, start));
    let mut entries: Vec<(String, Vec<String>)> = Vec::new();
    for (tz, who) in sources {
        let text = entry(when, tz, axis_date);
        match entries.iter_mut().find(|(known, _)| *known == text) {
            Some((_, merged)) => merged.extend(who),
            None => entries.push((text, who)),
        }
    }
    let zones = entries
        .into_iter()
        .map(|(text, who)| {
            if names && !who.is_empty() {
                format!("{text} ({})", who.join(", "))
            } else {
                text
            }
        })
        .collect::<Vec<_>>()
        .join(" / ");
    let prefix = if date {
        format!(
            "{}, ",
            start.with_timezone(&roster.axis()).format("%a %-d %b")
        )
    } else {
        String::new()
    };
    let states = match clause_mode {
        Clause::States => clause(roster, marks, when).map(|states| format!(", {states}")),
        Clause::Zones | Clause::Drop => None,
    };
    format!("{prefix}{zones}{}", states.unwrap_or_default())
}

/// The `c t` line: the focused row's entry, then the axis entry in parentheses, with no clause.
pub fn focus_line(roster: &Roster, row: Row, when: When) -> String {
    let tz = match row {
        Row::Person(id) => roster.person(id).location.tz,
        Row::Place(id) => roster.place(id).location.tz,
    };
    let axis_date = when.start().with_timezone(&roster.axis()).date_naive();
    format!(
        "{} ({})",
        entry(when, tz, axis_date),
        entry(when, roster.axis(), axis_date)
    )
}
