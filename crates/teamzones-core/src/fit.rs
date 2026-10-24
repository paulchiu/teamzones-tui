//! Fits: the slots where every counted row in every asked block qualifies, and the search for the next one.

use chrono::{DateTime, Days, Duration, NaiveDate, NaiveTime, TimeZone, Utc};
use chrono_tz::Tz;
use thiserror::Error;

use crate::clock::{Range, Slot, When};
use crate::config::Roster;
use crate::question::{Marks, Row, Score, person_qualifies, place_qualifies, slot_score};
use crate::time::Moment;

/// Nothing fits within the search window; `best` is the earliest slot with the most qualifying rows.
#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
#[error("no full slot in 7 days, best {best} at {}", crate::copy::at_text(.at.start(), *.axis))]
pub struct NoFit {
    pub best: Score,
    pub at: Slot,
    pub axis: Tz,
}

/// How far the next-fit search looks: 7 days of slots.
const HORIZON_SLOTS: i64 = 7 * 96;
/// The CLI caps a next fit at 1h.
const CAP_SLOTS: i64 = 4;

/// Whether every counted row in every asked block qualifies at `slot`; marks that ask nothing never fit.
pub fn slot_fits(roster: &Roster, marks: &Marks, slot: Slot) -> bool {
    marks.question().is_some()
        && marks
            .counted_people()
            .into_iter()
            .all(|id| person_qualifies(roster, id, slot))
        && marks
            .counted_places()
            .into_iter()
            .all(|id| place_qualifies(roster, id, marks.rule, slot))
}

/// The first instant of `day` in `tz`; a zone that skips midnight starts its day at 1am.
fn day_start(day: NaiveDate, tz: Tz) -> DateTime<Utc> {
    let midnight = day.and_time(NaiveTime::MIN);
    [midnight, midnight + Duration::hours(1)]
        .into_iter()
        .find_map(|local| tz.from_local_datetime(&local).earliest())
        .map_or(midnight.and_utc(), |local| local.with_timezone(&Utc))
}

/// The maximal runs of fitting slots among `slots`, in order.
fn runs(roster: &Roster, marks: &Marks, slots: impl Iterator<Item = Slot>) -> Vec<Range> {
    let mut found: Vec<Range> = Vec::new();
    for slot in slots.filter(|slot| slot_fits(roster, marks, *slot)) {
        match found.last_mut() {
            Some(run) if run.end() == slot => {
                *run = Range::new(run.start(), slot.next()).unwrap_or(*run);
            }
            _ => found.push(Range::single(slot)),
        }
    }
    found
}

/// The maximal fitting runs within one day of the axis zone, from its midnight to the next.
pub fn fits(roster: &Roster, marks: &Marks, day: NaiveDate) -> Vec<Range> {
    let axis = roster.axis();
    let (start, end) = (
        Slot::containing(day_start(day, axis)),
        Slot::containing(day_start(day + Days::new(1), axis)),
    );
    runs(
        roster,
        marks,
        (0..end.since(start)).map(|step| start.offset(step)),
    )
}

/// The CLI's next fit: from the first slot strictly after `now`, the first fitting run, capped at 1h.
///
/// # Errors
///
/// [`NoFit`] when nothing fits within 7 days, carrying the earliest slot with the best score.
pub fn next_fit(roster: &Roster, marks: &Marks, now: DateTime<Utc>) -> Result<Range, NoFit> {
    let first = Slot::first_after(now);
    let slots = || (0..HORIZON_SLOTS).map(move |step| first.offset(step));
    let start = slots().find(|slot| slot_fits(roster, marks, *slot));
    match start {
        Some(start) => {
            let end = (1..CAP_SLOTS)
                .map(|step| start.offset(step))
                .find(|slot| !slot_fits(roster, marks, *slot));
            Ok(Range::new(start, end.unwrap_or(start.offset(CAP_SLOTS)))
                .unwrap_or(Range::single(start)))
        }
        None => Err(best_of(roster, marks, slots())),
    }
}

/// The TUI's next fit: the first fitting run after `from`, skipping the run `from` already sits in.
///
/// # Errors
///
/// [`NoFit`] when nothing fits within 7 days, carrying the earliest slot with the best score.
pub fn next_run(roster: &Roster, marks: &Marks, from: Slot) -> Result<Range, NoFit> {
    let fits = |slot: Slot| slot_fits(roster, marks, slot);
    let after = (1..HORIZON_SLOTS)
        .map(|step| from.offset(step))
        .find(|slot| !fits(from) || !fits(*slot))
        .unwrap_or(from.offset(HORIZON_SLOTS));
    let window = || (0..HORIZON_SLOTS).map(move |step| after.offset(step));
    match window().find(|slot| fits(*slot)) {
        Some(start) => {
            let end = (1..HORIZON_SLOTS)
                .map(|step| start.offset(step))
                .find(|slot| !fits(*slot))
                .unwrap_or(start.offset(HORIZON_SLOTS));
            Ok(Range::new(start, end).unwrap_or(Range::single(start)))
        }
        None => Err(best_of(roster, marks, window())),
    }
}

/// The TUI's previous fit: the start of the fitting run before `from`, skipping the run `from` sits in.
///
/// # Errors
///
/// [`NoFit`] when nothing fits within 7 days back, carrying the earliest slot with the best score.
pub fn previous_run(roster: &Roster, marks: &Marks, from: Slot) -> Result<Range, NoFit> {
    let fits = |slot: Slot| slot_fits(roster, marks, slot);
    let before = (1..HORIZON_SLOTS)
        .map(|step| from.offset(-step))
        .find(|slot| !fits(from) || !fits(*slot))
        .unwrap_or(from.offset(-HORIZON_SLOTS));
    let window = || (0..HORIZON_SLOTS).map(move |step| before.offset(-step));
    match window().find(|slot| fits(*slot)) {
        Some(last) => {
            let start = (1..HORIZON_SLOTS)
                .map(|step| last.offset(-step))
                .find(|slot| !fits(*slot))
                .map_or(last.offset(1 - HORIZON_SLOTS), Slot::next);
            Ok(Range::new(start, last.next()).unwrap_or(Range::single(last)))
        }
        None => {
            let mut earliest_first: Vec<Slot> = window().collect();
            earliest_first.reverse();
            Err(best_of(roster, marks, earliest_first.into_iter()))
        }
    }
}

/// The first counted row that does not qualify at `slot`: people first, then places.
pub fn blocker(roster: &Roster, marks: &Marks, slot: Slot) -> Option<Row> {
    let person = marks
        .counted_people()
        .into_iter()
        .find(|id| !person_qualifies(roster, *id, slot))
        .map(Row::Person);
    person.or_else(|| {
        marks
            .counted_places()
            .into_iter()
            .find(|id| !place_qualifies(roster, *id, marks.rule, slot))
            .map(Row::Place)
    })
}

/// The earliest of `slots` with the most qualifying counted rows, as a [`NoFit`].
fn best_of(roster: &Roster, marks: &Marks, slots: impl Iterator<Item = Slot>) -> NoFit {
    let (best, at) = slots
        .map(|slot| (slot_score(roster, marks, slot), slot))
        .reduce(|best, candidate| {
            if candidate.0.qualifying > best.0.qualifying {
                candidate
            } else {
                best
            }
        })
        .expect("every search window holds at least one slot");
    NoFit {
        best,
        at,
        axis: roster.axis(),
    }
}

/// The instant or range a [`Moment`] names: now, the next fit for the marks, or what was typed.
///
/// # Errors
///
/// [`NoFit`] when the moment is next and nothing fits within 7 days.
pub fn resolve(
    moment: Moment,
    roster: &Roster,
    marks: &Marks,
    now: DateTime<Utc>,
) -> Result<When, NoFit> {
    match moment {
        Moment::Now => Ok(When::At(now)),
        Moment::Next => next_fit(roster, marks, now).map(When::Over),
        Moment::Fixed(when) => Ok(when),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, Rule};
    use crate::copy::span_text;

    const FIXTURE: &str = include_str!("../tests/fixtures/sample.toml");

    fn roster() -> Roster {
        Config::parse(FIXTURE).unwrap().roster
    }

    fn marks(roster: &Roster, group: &str) -> Marks {
        Marks::of_group(roster.group_named(group).unwrap())
    }

    /// Each range as `Sat 9:15-10am` in AEDT, so expected values read like the spec.
    fn aedt(ranges: &[Range]) -> Vec<String> {
        let tz = chrono_tz::Australia::Melbourne;
        ranges
            .iter()
            .map(|r| {
                let (start, end) = (
                    r.start().start().with_timezone(&tz),
                    r.end().start().with_timezone(&tz),
                );
                format!("{} {}", start.format("%a"), span_text(&start, &end))
            })
            .collect()
    }

    fn monday() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, 26).unwrap()
    }

    fn now() -> DateTime<Utc> {
        "2026-10-23T22:04:00Z".parse().unwrap()
    }

    #[test]
    fn regions_fit_four_windows_on_monday() {
        let roster = roster();
        assert_eq!(
            aedt(&fits(&roster, &marks(&roster, "regions"), monday())),
            [
                "Mon 1-3:30am",
                "Mon 8-9:30am",
                "Mon 2-5:30pm",
                "Mon 9-10:30pm"
            ]
        );
    }

    #[test]
    fn deploy_fits_only_5_to_5_30pm_on_monday() {
        let roster = roster();
        assert_eq!(
            aedt(&fits(&roster, &marks(&roster, "deploy"), monday())),
            ["Mon 5-5:30pm"]
        );
    }

    #[test]
    fn team_fits_only_5_to_6pm_on_monday() {
        let roster = roster();
        assert_eq!(
            aedt(&fits(&roster, &marks(&roster, "team"), monday())),
            ["Mon 5-6pm"]
        );
    }

    #[test]
    fn the_next_regions_fit_starts_at_the_first_slot_after_now() {
        let roster = roster();
        assert_eq!(
            aedt(&[next_fit(&roster, &marks(&roster, "regions"), now()).unwrap()]),
            ["Sat 9:15-10am"]
        );
    }

    #[test]
    fn the_next_fit_is_capped_at_one_hour() {
        let roster = roster();
        // Regions fit Mon 2-5:30pm AEDT, so a search from Mon 1:50pm takes 2-3pm.
        let from = "2026-10-26T02:50:00Z".parse().unwrap();
        assert_eq!(
            aedt(&[next_fit(&roster, &marks(&roster, "regions"), from).unwrap()]),
            ["Mon 2-3pm"]
        );
    }

    #[test]
    fn the_next_deploy_fit_skips_the_weekend() {
        let roster = roster();
        assert_eq!(
            aedt(&[next_fit(&roster, &marks(&roster, "deploy"), now()).unwrap()]),
            ["Mon 5-5:30pm"]
        );
    }

    #[test]
    fn quiet_regions_never_fit_and_report_the_best_two_of_three() {
        let roster = roster();
        let quiet = Marks {
            rule: Rule::Quiet,
            ..marks(&roster, "regions")
        };
        let no_fit = next_fit(&roster, &quiet, now()).unwrap_err();
        assert_eq!(
            no_fit.best,
            Score {
                qualifying: 2,
                counted: 3
            }
        );
        // US closes at 11pm Fri CDT, which is 3pm Sat AEDT, while UK is still closed.
        assert_eq!(
            no_fit.to_string(),
            "no full slot in 7 days, best 2/3 at Sat 3pm AEDT"
        );
    }

    fn aedt_slot(text: &str) -> Slot {
        Slot::containing(text.parse().unwrap())
    }

    #[test]
    fn next_run_from_saturday_morning_lands_on_monday_for_deploy() {
        let roster = roster();
        let run = next_run(&roster, &marks(&roster, "deploy"), Slot::containing(now())).unwrap();
        assert_eq!(aedt(&[run]), ["Mon 5-5:30pm"]);
    }

    #[test]
    fn next_run_steps_past_the_run_it_starts_in() {
        let roster = roster();
        // Mon 2pm AEDT sits in the 2-5:30pm regions run, so the next run is 9-10:30pm.
        let run = next_run(
            &roster,
            &marks(&roster, "regions"),
            aedt_slot("2026-10-26T03:00:00Z"),
        )
        .unwrap();
        assert_eq!(aedt(&[run]), ["Mon 9-10:30pm"]);
    }

    #[test]
    fn previous_run_steps_back_past_the_run_it_starts_in() {
        let roster = roster();
        // From Mon 9pm AEDT, inside the 9-10:30pm run, the previous run is 2-5:30pm.
        let run = previous_run(
            &roster,
            &marks(&roster, "regions"),
            aedt_slot("2026-10-26T10:00:00Z"),
        )
        .unwrap();
        assert_eq!(aedt(&[run]), ["Mon 2-5:30pm"]);
    }

    #[test]
    fn previous_run_from_outside_a_run_finds_the_one_before() {
        let roster = roster();
        // Mon 7pm AEDT is between runs; the one before is 2-5:30pm.
        let run = previous_run(
            &roster,
            &marks(&roster, "regions"),
            aedt_slot("2026-10-26T08:00:00Z"),
        )
        .unwrap();
        assert_eq!(aedt(&[run]), ["Mon 2-5:30pm"]);
    }

    #[test]
    fn next_run_reports_the_best_slot_when_nothing_fits() {
        let roster = roster();
        let quiet = Marks {
            rule: Rule::Quiet,
            ..marks(&roster, "regions")
        };
        let no_fit = next_run(&roster, &quiet, Slot::containing(now())).unwrap_err();
        assert_eq!(
            no_fit.to_string(),
            "no full slot in 7 days, best 2/3 at Sat 3pm AEDT"
        );
    }

    #[test]
    fn the_blocker_at_5_30pm_monday_on_deploy_is_au() {
        let roster = roster();
        let blocked = blocker(
            &roster,
            &marks(&roster, "deploy"),
            aedt_slot("2026-10-26T06:30:00Z"),
        );
        assert_eq!(blocked, Some(Row::Place(roster.place_named("AU").unwrap())));
    }

    #[test]
    fn nothing_blocks_a_fitting_slot() {
        let roster = roster();
        assert_eq!(
            blocker(
                &roster,
                &marks(&roster, "deploy"),
                aedt_slot("2026-10-26T06:00:00Z")
            ),
            None
        );
    }
}
