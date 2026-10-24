//! The say panel's text: the summary after the say line, and the focused row's detail.

use chrono::{DateTime, NaiveDate, Offset, TimeZone, Utc};
use chrono_tz::Tz;
use teamzones_core::clock::{Slot, When};
use teamzones_core::config::PersonId;
use teamzones_core::copy::{Clause, Variant, clock_text, say_line, span_text};
use teamzones_core::question::{Row, person_band, place_level, place_state};
use teamzones_core::schedule::{Band, Hours};
use teamzones_core::status::place_next;

use super::app::App;

/// A length in grid slots as the track label and notices write it: `1h`, `30m` or `1h15m`.
pub fn duration_text(slots: i64) -> String {
    let minutes = slots * 15;
    match (minutes / 60, minutes % 60) {
        (0, m) => format!("{m}m"),
        (h, 0) => format!("{h}h"),
        (h, m) => format!("{h}h{m}m"),
    }
}

fn plural(count: usize, one: &str, many: &str) -> String {
    format!("{count} {}", if count == 1 { one } else { many })
}

/// The dim summary after the say line, such as `3 zones for 5 people, 2 places`.
pub fn summary(app: &App) -> String {
    let roster = app.roster();
    let marks = &app.marks;
    let who = if marks.asks_who() {
        let short = Variant {
            clause: Clause::Drop,
            ..Variant::default()
        };
        let zones = say_line(roster, marks, app.when(), short)
            .split(" / ")
            .count();
        format!(
            "{} for {}",
            plural(zones, "zone", "zones"),
            plural(marks.counted_people().len(), "person", "people")
        )
    } else {
        "home zone".to_owned()
    };
    let places = marks.counted_places().len();
    if places == 0 {
        who
    } else {
        format!("{who}, {}", plural(places, "place", "places"))
    }
}

/// A local time or range in `tz`, such as `4-5pm`.
pub fn local_text(when: When, tz: Tz) -> String {
    match when {
        When::At(instant) => clock_text(&instant.with_timezone(&tz)),
        When::Over(range) => span_text(
            &range.start().start().with_timezone(&tz),
            &range.end().start().with_timezone(&tz),
        ),
    }
}

/// `UTC+10`, `UTC+5:30` or `UTC-5`.
fn offset_text(tz: Tz, instant: DateTime<Utc>) -> String {
    let seconds = instant.with_timezone(&tz).offset().fix().local_minus_utc();
    let sign = if seconds < 0 { '-' } else { '+' };
    let minutes = seconds.unsigned_abs() / 60;
    match minutes % 60 {
        0 => format!("UTC{sign}{}", minutes / 60),
        rest => format!("UTC{sign}{}:{rest:02}", minutes / 60),
    }
}

fn hours_text(Hours { span, days }: Hours) -> String {
    let day = NaiveDate::default();
    let wall = |minute: u32| {
        Utc.from_utc_datetime(&day.and_time(chrono::NaiveTime::MIN))
            + chrono::TimeDelta::minutes(i64::from(minute))
    };
    format!(
        "hours {} {days}",
        span_text(&wall(span.start), &wall(span.end))
    )
}

fn groups_text(app: &App, row: Row) -> Option<String> {
    let names: Vec<&str> = app
        .roster()
        .groups
        .iter()
        .filter(|group| match row {
            Row::Person(id) => id == PersonId::HOME || group.people.contains(&id),
            Row::Place(id) => group.places.contains(&id),
        })
        .map(|group| group.name.as_str())
        .collect();
    (!names.is_empty()).then(|| format!("groups {}", names.join(", ")))
}

/// The focused row's name and its detail pieces, which the panel drops whole from the right.
pub fn detail(app: &App) -> (String, Vec<String>) {
    let roster = app.roster();
    let when = app.when();
    let start = when.start();
    let slot = Slot::containing(start);
    match app.focus {
        Row::Person(id) => {
            let person = roster.person(id);
            let tz = person.location.tz;
            let local = start.with_timezone(&tz);
            let state = match person_band(roster, id, slot) {
                Band::Working => "in hours",
                band @ (Band::Night | Band::Day | Band::Morning | Band::Evening | Band::Off) => {
                    band.word()
                }
            };
            let pieces = [
                Some(person.city.clone()),
                Some(format!("{} {}", local.format("%Z"), offset_text(tz, start))),
                Some(
                    person
                        .hours
                        .map_or_else(|| "no hours set".to_owned(), hours_text),
                ),
                groups_text(app, app.focus),
                Some(format!(
                    "{} {} {state}",
                    local.format("%a %-d %b"),
                    local_text(when, tz)
                )),
            ];
            (person.name.clone(), pieces.into_iter().flatten().collect())
        }
        Row::Place(id) => {
            let place = roster.place(id);
            let tz = place.location.tz;
            let local = start.with_timezone(&tz);
            let state = place_state(roster, id, when).map_or("clock", |level| level.word());
            let next: Vec<String> = match (&place.trading, place_level(roster, id, slot)) {
                (Some(trading), Some(level)) => place_next(trading, tz, level, start)
                    .split(", ")
                    .filter(|piece| !piece.is_empty())
                    .map(str::to_owned)
                    .collect(),
                (None, _) | (_, None) => Vec::new(),
            };
            let pieces = std::iter::once(format!(
                "{} {} {state}",
                local.format("%a"),
                local_text(when, tz)
            ))
            .chain(next)
            .chain([
                format!("{} {}", local.format("%Z"), offset_text(tz, start)),
                tz.name().to_owned(),
            ])
            .chain(place.profile.as_ref().map(|name| format!("{name} profile")))
            .chain(groups_text(app, app.focus))
            .collect();
            (place.name.clone(), pieces)
        }
    }
}
