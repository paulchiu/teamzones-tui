//! Headless status lines: one per marked row, saying where it is now and what comes next.

use chrono::{DateTime, Datelike, Days, NaiveDate, NaiveTime, Timelike, Utc};
use chrono_tz::Tz;

use crate::clock::Slot;
use crate::config::{PersonId, Roster};
use crate::copy::{clock_text, span_text};
use crate::question::{Marks, person_band, place_level};
use crate::schedule::{Level, LocalSpan, Trading, band};

/// How many days ahead "what comes next" looks for an opening or a peak.
const LOOKAHEAD_DAYS: u64 = 7;

/// `Sat 9:04am AEDT`: the row's local weekday, time and zone at `now`.
fn local_now(now: DateTime<Utc>, tz: Tz) -> String {
    let local = now.with_timezone(&tz);
    format!(
        "{} {} {}",
        local.format("%a"),
        clock_text(&local),
        local.format("%Z")
    )
}

/// A wall-clock instant on `date` at `minute` (1440 is the next midnight), for formatting only.
fn wall(date: NaiveDate, minute: u32) -> DateTime<Utc> {
    let (date, minute) = if minute >= 24 * 60 {
        (date + Days::new(1), minute - 24 * 60)
    } else {
        (date, minute)
    };
    date.and_time(
        NaiveTime::from_num_seconds_from_midnight_opt(minute * 60, 0).unwrap_or(NaiveTime::MIN),
    )
    .and_utc()
}

/// The weekday prefix for a date other than the row's local today.
fn day_prefix(date: NaiveDate, today: NaiveDate) -> String {
    if date == today {
        String::new()
    } else {
        format!("{} ", date.format("%a"))
    }
}

fn clock_on(date: NaiveDate, minute: u32, today: NaiveDate) -> String {
    format!(
        "{}{}",
        day_prefix(date, today),
        clock_text(&wall(date, minute))
    )
}

fn span_on(date: NaiveDate, span: LocalSpan, today: NaiveDate) -> String {
    format!(
        "{}{}",
        day_prefix(date, today),
        span_text(&wall(date, span.start), &wall(date, span.end))
    )
}

/// The first peak that starts at or after `minute` on `date`, or on an open day after it.
fn next_peak(trading: &Trading, date: NaiveDate, minute: u32) -> Option<(NaiveDate, LocalSpan)> {
    (0..=LOOKAHEAD_DAYS)
        .map(|ahead| date + Days::new(ahead))
        .find_map(|day| {
            let from = if day == date { minute } else { 0 };
            let trading_day = trading.on(day.weekday());
            trading_day.open.and(
                trading_day
                    .peaks
                    .iter()
                    .find(|peak| peak.start >= from)
                    .map(|peak| (day, *peak)),
            )
        })
}

/// The first opening at or after `minute` on `date`, or on a later day.
fn next_opening(trading: &Trading, date: NaiveDate, minute: u32) -> Option<(NaiveDate, u32)> {
    (0..=LOOKAHEAD_DAYS)
        .map(|ahead| date + Days::new(ahead))
        .find_map(|day| {
            let open = trading.on(day.weekday()).open?;
            (day != date || open.start >= minute).then_some((day, open.start))
        })
}

/// What comes next for a place: the peak in progress or coming up today, or when it opens.
fn place_next(trading: &Trading, tz: Tz, level: Level, now: DateTime<Utc>) -> String {
    let local = now.with_timezone(&tz);
    let (today, minute) = (local.date_naive(), local.hour() * 60 + local.minute());
    let trading_day = trading.on(local.weekday());
    let later_peak = |date, from| {
        next_peak(trading, date, from)
            .map(|(day, peak)| format!("next peak {}", span_on(day, peak, today)))
    };
    let pieces: Vec<String> = match level {
        Level::Peak => trading_day
            .peaks
            .iter()
            .find(|peak| peak.contains(minute))
            .map(|peak| format!("peak {}", span_on(today, *peak, today)))
            .into_iter()
            .collect(),
        Level::OffPeak => match trading_day.peaks.iter().find(|peak| peak.start > minute) {
            Some(peak) => vec![format!("peak {}", span_on(today, *peak, today))],
            None => {
                let closes = trading_day
                    .open
                    .map(|open| format!("closes {}", clock_on(today, open.end, today)));
                closes
                    .into_iter()
                    .chain(later_peak(today + Days::new(1), 0))
                    .collect()
            }
        },
        Level::Quiet => match next_opening(trading, today, minute) {
            Some((day, opens)) => std::iter::once(format!("opens {}", clock_on(day, opens, today)))
                .chain(later_peak(day, opens))
                .collect(),
            None => vec!["closed all week".to_owned()],
        },
    };
    pieces.join(", ")
}

/// Pads every column except the last to its longest value and joins them with two spaces.
fn columns(rows: &[Vec<String>]) -> Vec<String> {
    let widths: Vec<usize> = (0..rows.iter().map(Vec::len).max().unwrap_or(0))
        .map(|column| {
            rows.iter()
                .filter_map(|row| row.get(column))
                .map(|cell| cell.chars().count())
                .max()
                .unwrap_or(0)
        })
        .collect();
    rows.iter()
        .map(|row| {
            let last = row.len().saturating_sub(1);
            let cells = row.iter().enumerate().map(|(i, cell)| {
                if i == last {
                    cell.clone()
                } else {
                    format!("{cell:<width$}", width = widths[i])
                }
            });
            cells.collect::<Vec<_>>().join("  ").trim_end().to_owned()
        })
        .collect()
}

/// One line per marked row (people first, home left out), with a blank line before the places.
pub fn status_lines(roster: &Roster, marks: &Marks, now: DateTime<Utc>) -> Vec<String> {
    let slot = Slot::containing(now);
    let people: Vec<Vec<String>> = marks
        .people
        .iter()
        .filter(|id| **id != PersonId::HOME)
        .map(|id| {
            let person = roster.person(*id);
            vec![
                person.name.clone(),
                local_now(now, person.location.tz),
                person_band(roster, *id, slot).word().to_owned(),
            ]
        })
        .collect();
    let places: Vec<Vec<String>> = marks
        .places
        .iter()
        .map(|id| {
            let place = roster.place(*id);
            let tz = place.location.tz;
            let (state, next) = match (&place.trading, place_level(roster, *id, slot)) {
                (Some(trading), Some(level)) => {
                    (level.word().to_owned(), place_next(trading, tz, level, now))
                }
                (None, _) | (_, None) => (
                    band(place.location, None, roster.daylight, slot)
                        .word()
                        .to_owned(),
                    String::new(),
                ),
            };
            vec![place.name.clone(), state, local_now(now, tz), next]
        })
        .collect();
    let separator = (!people.is_empty() && !places.is_empty()).then(String::new);
    columns(&people)
        .into_iter()
        .chain(separator)
        .chain(columns(&places))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    const FIXTURE: &str = include_str!("../tests/fixtures/sample.toml");

    #[test]
    fn people_come_first_then_a_blank_line_then_places() {
        let roster = Config::parse(FIXTURE).unwrap().roster;
        let marks = Marks::of_group(roster.group_named("deploy").unwrap());
        let lines = status_lines(&roster, &marks, "2026-10-23T22:04:00Z".parse().unwrap());
        assert_eq!(lines.len(), 5);
        assert!(
            lines[0].starts_with("Priya ") && lines[2].is_empty() && lines[3].starts_with("AU "),
            "{lines:#?}"
        );
    }

    #[test]
    fn a_place_at_peak_shows_the_peak_in_progress() {
        let roster = Config::parse(FIXTURE).unwrap().roster;
        let marks = Marks::of_group(roster.group_named("regions").unwrap());
        // Sat 24 Oct 12pm AEDT: AU is in its Saturday lunch peak, UK quiet and US peak, so the state column is five wide.
        let lines = status_lines(&roster, &marks, "2026-10-24T01:00:00Z".parse().unwrap());
        assert_eq!(lines[0], "AU  peak   Sat 12pm AEDT  peak 11:30am-2:30pm");
    }

    #[test]
    fn off_peak_after_the_last_peak_shows_closing_and_the_next_peak() {
        let roster = Config::parse(FIXTURE).unwrap().roster;
        let marks = Marks::of_group(roster.group_named("regions").unwrap());
        // Sat 24 Oct 10:30pm AEDT: AU is open until 11pm with no peak left today.
        let lines = status_lines(&roster, &marks, "2026-10-24T11:30:00Z".parse().unwrap());
        assert!(
            lines[0].ends_with("closes 11pm, next peak Sun 11:30am-2pm"),
            "{}",
            lines[0]
        );
    }
}
