//! Weekly schedules on a local clock, and the band or level they put a row in at a slot.

use chrono::{DateTime, Datelike, Timelike, Utc, Weekday};
use chrono_tz::Tz;
use thiserror::Error;

use crate::clock::Slot;
use crate::time::{TimeError, parse_local_range, parse_weekday};

const DAY_MINUTES: u32 = 24 * 60;

/// Where a row is on Earth: its zone, plus the coordinates sunrise and the map use.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Location {
    pub tz: Tz,
    pub lat: f64,
    pub lon: f64,
}

/// A span of one local day in minutes from midnight, `[start, end)`, which never crosses midnight.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LocalSpan {
    pub start: u32,
    pub end: u32,
}

/// Why a schedule value in config did not parse.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum ScheduleError {
    #[error(transparent)]
    Time(#[from] TimeError),
    #[error("\"{text}\" is not a range such as 9am-5pm")]
    NotARange { text: String },
    #[error(
        "\"{text}\" crosses midnight, which a schedule range cannot (end it at midnight instead)"
    )]
    CrossesMidnight { text: String },
    #[error("\"{text}\" is not a list of days such as Mon-Fri or Mon,Wed,Fri")]
    NotDays { text: String },
}

impl LocalSpan {
    /// Parses a schedule range such as `9am-5pm` or `5:30-9pm`, where `midnight` may end it.
    ///
    /// # Errors
    ///
    /// [`ScheduleError::NotARange`] for anything that is not a range, [`ScheduleError::Time`]
    /// for a range the time grammar rejects, and [`ScheduleError::CrossesMidnight`] for one
    /// whose end comes before its start.
    pub fn parse(text: &str) -> Result<Self, ScheduleError> {
        let (start, end) = parse_local_range(text)?.ok_or_else(|| ScheduleError::NotARange {
            text: text.to_owned(),
        })?;
        let end = if end == 0 { DAY_MINUTES } else { end };
        if end <= start {
            return Err(ScheduleError::CrossesMidnight {
                text: text.to_owned(),
            });
        }
        Ok(Self { start, end })
    }

    pub fn contains(self, minute: u32) -> bool {
        (self.start..self.end).contains(&minute)
    }
}

/// A set of weekdays.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Days(u8);

impl Days {
    pub const MON_FRI: Self = Self(0b001_1111);

    /// Parses `Mon-Fri`, a wrapping range such as `Fri-Mon`, a comma list or one day.
    ///
    /// # Errors
    ///
    /// [`ScheduleError::NotDays`] when any part is not a weekday or a range of them.
    pub fn parse(text: &str) -> Result<Self, ScheduleError> {
        let not_days = || ScheduleError::NotDays {
            text: text.to_owned(),
        };
        text.split(',').try_fold(Self(0), |days, part| {
            let part = part.trim();
            let bits = match part.split_once('-') {
                Some((from, to)) => {
                    let (from, to) = (
                        parse_weekday(from).ok_or_else(not_days)?,
                        parse_weekday(to).ok_or_else(not_days)?,
                    );
                    let len = (to.num_days_from_monday() + 7 - from.num_days_from_monday()) % 7 + 1;
                    (0..len).fold(0, |bits, step| {
                        bits | bit(from.num_days_from_monday() + step)
                    })
                }
                None => bit(parse_weekday(part)
                    .ok_or_else(not_days)?
                    .num_days_from_monday()),
            };
            Ok(Self(days.0 | bits))
        })
    }

    pub fn contains(self, day: Weekday) -> bool {
        self.0 & bit(day.num_days_from_monday()) != 0
    }

    pub fn count(self) -> u32 {
        self.0.count_ones()
    }
}

/// Writes each run of consecutive days as `Mon-Fri` (wrapping past Sunday) and joins runs with commas.
impl std::fmt::Display for Days {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        const NAMES: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
        let set = |index: u32| self.0 & bit(index) != 0;
        if self.count() == 7 {
            return f.write_str("Mon-Sun");
        }
        let runs: Vec<String> = (0..7)
            .filter(|&start| set(start) && !set(start + 6))
            .map(|start| {
                let last = (start..start + 6)
                    .take_while(|&day| set(day + 1))
                    .last()
                    .map_or(start, |day| day + 1);
                let (from, to) = (NAMES[start as usize], NAMES[last as usize % 7]);
                if start == last {
                    from.to_owned()
                } else {
                    format!("{from}-{to}")
                }
            })
            .collect();
        f.write_str(&runs.join(","))
    }
}

fn bit(index: u32) -> u8 {
    1 << (index % 7)
}

/// Office hours: a local span on a set of work days.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hours {
    pub span: LocalSpan,
    pub days: Days,
}

/// One local day of a trading profile: open hours (`None` when closed) and the peaks within.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TradingDay {
    pub open: Option<LocalSpan>,
    pub peaks: Vec<LocalSpan>,
}

/// A trading profile resolved to each weekday, Monday first.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Trading {
    pub days: [TradingDay; 7],
}

impl Trading {
    pub fn on(&self, day: Weekday) -> &TradingDay {
        let index = match day {
            Weekday::Mon => 0,
            Weekday::Tue => 1,
            Weekday::Wed => 2,
            Weekday::Thu => 3,
            Weekday::Fri => 4,
            Weekday::Sat => 5,
            Weekday::Sun => 6,
        };
        &self.days[index]
    }
}

/// Where day and night come from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Daylight {
    Sun,
    Clock,
}

/// A person's band at a slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Band {
    Night,
    Day,
    Morning,
    Working,
    Evening,
    Off,
}

impl Band {
    /// The word status lines and labels use.
    pub fn word(self) -> &'static str {
        match self {
            Self::Night => "night",
            Self::Day => "day",
            Self::Morning => "morning",
            Self::Working => "working",
            Self::Evening => "evening",
            Self::Off => "day off",
        }
    }
}

/// A place's trading load at a slot, ordered so the worst level is the greatest.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    Quiet,
    OffPeak,
    Peak,
}

impl Level {
    pub fn word(self) -> &'static str {
        match self {
            Self::Quiet => "quiet",
            Self::OffPeak => "off-peak",
            Self::Peak => "peak",
        }
    }
}

fn local_minute(local: DateTime<Tz>) -> u32 {
    local.hour() * 60 + local.minute()
}

/// Whether it is day at `location` when `slot` starts, by the sun or by a fixed 6am-6pm.
pub fn is_day(location: Location, daylight: Daylight, slot: Slot) -> bool {
    match daylight {
        Daylight::Sun => sun_up(location.lat, location.lon, slot.start()),
        Daylight::Clock => {
            (6 * 60..18 * 60).contains(&local_minute(slot.start().with_timezone(&location.tz)))
        }
    }
}

/// The band of someone at `location` with these `hours` (or none) when `slot` starts.
pub fn band(location: Location, hours: Option<Hours>, daylight: Daylight, slot: Slot) -> Band {
    let day = is_day(location, daylight, slot);
    let local = slot.start().with_timezone(&location.tz);
    let minute = local_minute(local);
    match hours {
        None if day => Band::Day,
        None => Band::Night,
        Some(hours) if !hours.days.contains(local.weekday()) => {
            if day {
                Band::Off
            } else {
                Band::Night
            }
        }
        Some(hours) if hours.span.contains(minute) => Band::Working,
        Some(_) if !day => Band::Night,
        Some(hours) if minute < hours.span.start => Band::Morning,
        Some(_) => Band::Evening,
    }
}

/// The trading level of a place in zone `tz` when `slot` starts.
pub fn level(trading: &Trading, tz: Tz, slot: Slot) -> Level {
    let local = slot.start().with_timezone(&tz);
    let minute = local_minute(local);
    let day = trading.on(local.weekday());
    match day.open {
        None => Level::Quiet,
        Some(_) if day.peaks.iter().any(|peak| peak.contains(minute)) => Level::Peak,
        Some(open) if open.contains(minute) => Level::OffPeak,
        Some(_) => Level::Quiet,
    }
}

/// NOAA solar position at `instant`: the subsolar longitude and the declination, in degrees.
pub fn subsolar(instant: DateTime<Utc>) -> (f64, f64) {
    let hours = f64::from(instant.hour()) + f64::from(instant.minute()) / 60.0;
    let year_angle = std::f64::consts::TAU / 365.0
        * (f64::from(instant.ordinal()) - 1.0 + (hours - 12.0) / 24.0);
    let (g, g2, g3) = (year_angle, 2.0 * year_angle, 3.0 * year_angle);
    let equation_of_time = 229.18
        * (0.000_075 + 0.001_868 * g.cos()
            - 0.032_077 * g.sin()
            - 0.014_615 * g2.cos()
            - 0.040_849 * g2.sin());
    let declination = 0.006_918 - 0.399_912 * g.cos() + 0.070_257 * g.sin() - 0.006_758 * g2.cos()
        + 0.000_907 * g2.sin()
        - 0.002_697 * g3.cos()
        + 0.001_48 * g3.sin();
    let lon = (-(hours * 60.0 + equation_of_time - 720.0) / 4.0 + 540.0) % 360.0 - 180.0;
    (lon, declination.to_degrees())
}

/// Whether the sun is above the horizon, allowing the standard -0.833 degrees for refraction.
pub fn sun_up(lat: f64, lon: f64, instant: DateTime<Utc>) -> bool {
    let (sun_lon, declination) = subsolar(instant);
    let (lat, declination) = (lat.to_radians(), declination.to_radians());
    let cos_zenith = lat.sin() * declination.sin()
        + lat.cos() * declination.cos() * (lon - sun_lon).to_radians().cos();
    cos_zenith.asin().to_degrees() > -0.833
}

#[cfg(test)]
mod tests {
    use super::*;

    fn slot(text: &str) -> Slot {
        Slot::containing(text.parse().unwrap())
    }

    const BRISBANE: Location = Location {
        tz: chrono_tz::Australia::Brisbane,
        lat: -27.47,
        lon: 153.03,
    };
    const NINE_TO_FIVE: Hours = Hours {
        span: LocalSpan {
            start: 9 * 60,
            end: 17 * 60,
        },
        days: Days::MON_FRI,
    };

    #[test]
    fn schedule_ranges_take_one_suffix_for_both_ends() {
        assert_eq!(
            LocalSpan::parse("5:30-9pm").unwrap(),
            LocalSpan {
                start: 17 * 60 + 30,
                end: 21 * 60
            }
        );
        assert_eq!(
            LocalSpan::parse("11:30am-2pm").unwrap(),
            LocalSpan {
                start: 11 * 60 + 30,
                end: 14 * 60
            }
        );
    }

    #[test]
    fn a_schedule_range_may_end_at_midnight_but_not_cross_it() {
        assert_eq!(
            LocalSpan::parse("6pm-midnight").unwrap(),
            LocalSpan {
                start: 18 * 60,
                end: 24 * 60
            }
        );
        assert!(matches!(
            LocalSpan::parse("10pm-2am"),
            Err(ScheduleError::CrossesMidnight { .. })
        ));
    }

    #[test]
    fn days_write_back_as_the_shortest_runs_they_parse_from() {
        let text = |days: &str| Days::parse(days).unwrap().to_string();
        assert_eq!(
            [
                text("Mon-Fri"),
                text("Fri-Mon"),
                text("Mon,Wed,Fri"),
                text("Sat"),
                text("Mon-Sun")
            ],
            ["Mon-Fri", "Fri-Mon", "Mon,Wed,Fri", "Sat", "Mon-Sun"]
        );
    }

    #[test]
    fn days_parse_ranges_wrapping_ranges_lists_and_single_days() {
        assert_eq!(Days::parse("Mon-Fri").unwrap(), Days::MON_FRI);
        let wrap = Days::parse("Fri-Mon").unwrap();
        assert_eq!(wrap.count(), 4);
        assert!(wrap.contains(Weekday::Sun) && !wrap.contains(Weekday::Tue));
        let list = Days::parse("Mon,Wed,Fri").unwrap();
        assert!(list.contains(Weekday::Wed) && !list.contains(Weekday::Thu));
        assert_eq!(Days::parse("Sat").unwrap().count(), 1);
        assert!(Days::parse("Funday").is_err());
    }

    #[test]
    fn a_work_day_runs_morning_working_evening_and_night() {
        let band_at = |utc| band(BRISBANE, Some(NINE_TO_FIVE), Daylight::Sun, slot(utc));
        // Mon 26 Oct 2026 in Brisbane (UTC+10): sunrise near 5am, sunset near 6pm.
        assert_eq!(band_at("2026-10-25T21:00:00Z"), Band::Morning);
        assert_eq!(band_at("2026-10-25T23:00:00Z"), Band::Working);
        assert_eq!(band_at("2026-10-26T07:15:00Z"), Band::Evening);
        assert_eq!(band_at("2026-10-26T12:00:00Z"), Band::Night);
    }

    #[test]
    fn a_day_off_is_off_while_the_sun_is_up_and_night_otherwise() {
        let band_at = |utc| band(BRISBANE, Some(NINE_TO_FIVE), Daylight::Sun, slot(utc));
        // Sat 24 Oct 2026, 8:04am and 11pm in Brisbane.
        assert_eq!(band_at("2026-10-23T22:04:00Z"), Band::Off);
        assert_eq!(band_at("2026-10-24T13:00:00Z"), Band::Night);
    }

    #[test]
    fn without_hours_a_person_is_in_day_or_night() {
        assert_eq!(
            band(BRISBANE, None, Daylight::Sun, slot("2026-10-26T02:00:00Z")),
            Band::Day
        );
        assert_eq!(
            band(BRISBANE, None, Daylight::Sun, slot("2026-10-26T14:00:00Z")),
            Band::Night
        );
    }

    #[test]
    fn clock_bands_put_day_at_six_to_six_local() {
        // 5:45am and 6am in Brisbane, both after a real sunrise near 5am.
        assert_eq!(
            band(
                BRISBANE,
                None,
                Daylight::Clock,
                slot("2026-10-25T19:45:00Z")
            ),
            Band::Night
        );
        assert_eq!(
            band(
                BRISBANE,
                None,
                Daylight::Clock,
                slot("2026-10-25T20:00:00Z")
            ),
            Band::Day
        );
    }

    #[test]
    fn a_place_is_peak_then_off_peak_then_quiet() {
        let day = TradingDay {
            open: Some(LocalSpan {
                start: 7 * 60,
                end: 23 * 60,
            }),
            peaks: vec![LocalSpan {
                start: 11 * 60 + 30,
                end: 14 * 60,
            }],
        };
        let trading = Trading {
            days: std::array::from_fn(|_| day.clone()),
        };
        let level_at = |utc| level(&trading, chrono_tz::Europe::London, slot(utc));
        // Mon 26 Oct 2026 in London (GMT).
        assert_eq!(level_at("2026-10-26T12:00:00Z"), Level::Peak);
        assert_eq!(level_at("2026-10-26T15:00:00Z"), Level::OffPeak);
        assert_eq!(level_at("2026-10-26T03:00:00Z"), Level::Quiet);
        assert!(Level::Peak > Level::OffPeak && Level::OffPeak > Level::Quiet);
    }
}
