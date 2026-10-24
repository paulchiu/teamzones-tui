//! The typed-time grammar shared by the `t` prompt, the CLI's TIME and schedule ranges in config.

use chrono::offset::LocalResult;
use chrono::{
    DateTime, Datelike, Days as ChronoDays, NaiveDate, NaiveTime, Offset, TimeZone, Utc, Weekday,
};
use chrono_tz::{OffsetName, TZ_VARIANTS, Tz};
use thiserror::Error;

use crate::cities::{self, CityError};
use crate::clock::{Range, When};
use crate::config::Roster;

/// Why typed time text did not parse.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum TimeError {
    #[error("{text} needs am/pm or HH:MM, such as 9am-5pm or 09:00-17:00")]
    NeedsSuffix { text: String },
    #[error("{token} is not a time, a day or a zone")]
    Unknown { token: String },
    #[error("{second}: only one {kind} per time, and {first} is already one")]
    Repeated {
        kind: &'static str,
        first: String,
        second: String,
    },
    #[error("{anchor} stands alone, so drop {other}")]
    Standalone { anchor: String, other: String },
    #[error("{abbreviation} is ambiguous: {}", .candidates.join(", "))]
    AmbiguousAbbreviation {
        abbreviation: String,
        candidates: Vec<String>,
    },
    #[error("{abbreviation} is not in use on {}", .date.format("%a %-d %b"))]
    AbbreviationNotInUse {
        abbreviation: String,
        date: NaiveDate,
    },
    #[error(transparent)]
    City(#[from] CityError),
    #[error("{text} is not a date")]
    BadDate { text: String },
    #[error("{time} does not exist in {zone} on {date}, because the clocks change")]
    Skipped {
        time: String,
        zone: String,
        date: NaiveDate,
    },
}

/// What typed time text means, before any search: now, the next fit, or a fixed instant or range.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Moment {
    Now,
    Next,
    Fixed(When),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Suffix {
    Am,
    Pm,
}

impl Suffix {
    fn other(self) -> Self {
        match self {
            Self::Am => Self::Pm,
            Self::Pm => Self::Am,
        }
    }
}

/// One end of a clock, as written: with a suffix, absolute (`HH:MM`, `HHMM`, noon, midnight), or bare.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Written {
    Suffixed {
        hour: u32,
        minute: u32,
        suffix: Suffix,
    },
    Absolute {
        minutes: u32,
    },
    /// A number such as `9` or `5:30` that needs a suffix from the other end of a range.
    Bare {
        hour: u32,
        minute: u32,
    },
}

fn with_suffix(hour: u32, minute: u32, suffix: Suffix) -> Option<u32> {
    (1..=12)
        .contains(&hour)
        .then_some((hour % 12 + if suffix == Suffix::Pm { 12 } else { 0 }) * 60 + minute)
}

fn hour_minute(text: &str) -> Option<(u32, u32)> {
    let (hour, minute) = text.split_once(':').unwrap_or((text, "00"));
    let digits = |part: &str, max_len| {
        !part.is_empty() && part.len() <= max_len && part.bytes().all(|b| b.is_ascii_digit())
    };
    if !(digits(hour, 2) && minute.len() == 2 && digits(minute, 2)) {
        return None;
    }
    let (hour, minute) = (hour.parse().ok()?, minute.parse().ok()?);
    (hour < 24 && minute < 60).then_some((hour, minute))
}

fn parse_written(text: &str) -> Option<Written> {
    let lower = text.to_ascii_lowercase();
    match lower.as_str() {
        "noon" => return Some(Written::Absolute { minutes: 12 * 60 }),
        "midnight" => return Some(Written::Absolute { minutes: 0 }),
        _ => {}
    }
    let suffixed = [("am", Suffix::Am), ("pm", Suffix::Pm)]
        .into_iter()
        .find_map(|(end, suffix)| {
            let (hour, minute) = hour_minute(lower.strip_suffix(end)?)?;
            with_suffix(hour, minute, suffix).map(|_| Written::Suffixed {
                hour,
                minute,
                suffix,
            })
        });
    if suffixed.is_some() {
        return suffixed;
    }
    if lower.len() == 4 && lower.bytes().all(|b| b.is_ascii_digit()) {
        let (hour, minute) = hour_minute(&format!("{}:{}", &lower[..2], &lower[2..]))?;
        return Some(Written::Absolute {
            minutes: hour * 60 + minute,
        });
    }
    let (hour, minute) = hour_minute(&lower)?;
    let two_digit_hour = lower
        .split_once(':')
        .is_some_and(|(hour, _)| hour.len() == 2);
    Some(if two_digit_hour {
        Written::Absolute {
            minutes: hour * 60 + minute,
        }
    } else {
        Written::Bare { hour, minute }
    })
}

/// Parses one clock time (`3pm`, `3:30pm`, `15:00`, `1500`, `noon`, `midnight`) to minutes after midnight.
pub fn parse_clock(text: &str) -> Option<u32> {
    match parse_written(text)? {
        Written::Suffixed {
            hour,
            minute,
            suffix,
        } => with_suffix(hour, minute, suffix),
        Written::Absolute { minutes } => Some(minutes),
        Written::Bare { .. } => None,
    }
}

/// Parses a range such as `3-5pm` or `10pm-2am` into start and end minutes after midnight.
///
/// One suffix applies to both ends unless that puts the end at or before the start, and then
/// the bare end takes the other suffix. An end at or before the start crosses midnight.
///
/// # Errors
///
/// [`TimeError::NeedsSuffix`] for a bare range such as `9-5` or `15-17`. Text that is not a
/// range at all is `Ok(None)`.
pub fn parse_local_range(text: &str) -> Result<Option<(u32, u32)>, TimeError> {
    let Some((from, to)) = text
        .split_once('-')
        .filter(|(from, to)| !from.is_empty() && !to.contains('-'))
    else {
        return Ok(None);
    };
    let (Some(from), Some(to)) = (parse_written(from), parse_written(to)) else {
        return Ok(None);
    };
    let needs_suffix = || TimeError::NeedsSuffix {
        text: text.to_owned(),
    };
    let absolute = |end: Written| match end {
        Written::Suffixed {
            hour,
            minute,
            suffix,
        } => with_suffix(hour, minute, suffix),
        Written::Absolute { minutes } => Some(minutes),
        Written::Bare { .. } => None,
    };
    let pair = match (from, to) {
        (Written::Bare { hour, minute }, Written::Suffixed { suffix, .. }) => {
            let end = absolute(to).ok_or_else(needs_suffix)?;
            let start = with_suffix(hour, minute, suffix).ok_or_else(needs_suffix)?;
            let start = if end <= start {
                with_suffix(hour, minute, suffix.other()).ok_or_else(needs_suffix)?
            } else {
                start
            };
            (start, end)
        }
        (Written::Suffixed { suffix, .. }, Written::Bare { hour, minute }) => {
            let start = absolute(from).ok_or_else(needs_suffix)?;
            let end = with_suffix(hour, minute, suffix).ok_or_else(needs_suffix)?;
            let end = if end <= start {
                with_suffix(hour, minute, suffix.other()).ok_or_else(needs_suffix)?
            } else {
                end
            };
            (start, end)
        }
        _ => (
            absolute(from).ok_or_else(needs_suffix)?,
            absolute(to).ok_or_else(needs_suffix)?,
        ),
    };
    Ok(Some(pair))
}

/// Parses a weekday as a three-letter or full English name, ignoring case.
pub fn parse_weekday(text: &str) -> Option<Weekday> {
    let lower = text.to_ascii_lowercase();
    [
        ("mon", "monday", Weekday::Mon),
        ("tue", "tuesday", Weekday::Tue),
        ("wed", "wednesday", Weekday::Wed),
        ("thu", "thursday", Weekday::Thu),
        ("fri", "friday", Weekday::Fri),
        ("sat", "saturday", Weekday::Sat),
        ("sun", "sunday", Weekday::Sun),
    ]
    .into_iter()
    .find_map(|(short, long, day)| (lower == short || lower == long).then_some(day))
}

fn parse_month(text: &str) -> Option<u32> {
    let lower = text.to_ascii_lowercase();
    let months = [
        "january",
        "february",
        "march",
        "april",
        "may",
        "june",
        "july",
        "august",
        "september",
        "october",
        "november",
        "december",
    ];
    months
        .iter()
        .zip(1..)
        .find_map(|(name, number)| (lower == *name || lower == name[..3]).then_some(number))
}

#[derive(Clone, Copy, Debug)]
enum TimeOfDay {
    Clock(u32),
    Range(u32, u32),
}

impl TimeOfDay {
    fn start(self) -> u32 {
        match self {
            Self::Clock(minutes) | Self::Range(minutes, _) => minutes,
        }
    }
}

/// A zone as typed: fixed by name, or an abbreviation that only the typed instant can resolve.
#[derive(Clone, Debug)]
enum ZoneToken {
    Fixed(Tz),
    Abbreviation(String),
}

#[derive(Clone, Debug)]
enum Token {
    Anchor(Moment),
    Time(TimeOfDay),
    Day(NaiveDate),
    Zone(ZoneToken),
}

/// The typed-time parts found so far, with the text each came from.
#[derive(Default)]
struct Parts {
    anchor: Option<(Moment, String)>,
    time: Option<(TimeOfDay, String)>,
    day: Option<(NaiveDate, String)>,
    zone: Option<(ZoneToken, String)>,
}

fn place_once<T>(
    slot: &mut Option<(T, String)>,
    value: T,
    text: String,
    kind: &'static str,
) -> Result<(), TimeError> {
    match slot {
        Some((_, first)) => Err(TimeError::Repeated {
            kind,
            first: first.clone(),
            second: text,
        }),
        None => {
            *slot = Some((value, text));
            Ok(())
        }
    }
}

struct Context<'a> {
    roster: &'a Roster,
    now: DateTime<Utc>,
    today: NaiveDate,
}

/// Parses typed time text (tokens in any order, at most one of each kind) as of `now`.
///
/// # Errors
///
/// Any [`TimeError`]: an unknown token, a repeated kind, a bare range, an ambiguous zone
/// abbreviation or city, a date that does not exist, or a clock time the clocks skip.
pub fn parse_time(text: &str, roster: &Roster, now: DateTime<Utc>) -> Result<Moment, TimeError> {
    let tokens: Vec<&str> = text.split_whitespace().collect();
    parse_time_tokens(&tokens, roster, now)
}

/// [`parse_time`] over text already split into tokens, as the CLI receives it.
///
/// # Errors
///
/// As for [`parse_time`].
pub fn parse_time_tokens(
    tokens: &[&str],
    roster: &Roster,
    now: DateTime<Utc>,
) -> Result<Moment, TimeError> {
    let ctx = Context {
        roster,
        now,
        today: now.with_timezone(&roster.axis()).date_naive(),
    };
    let mut parts = Parts::default();
    let mut rest = tokens;
    while let Some(first) = rest.first() {
        let (token, used) = read_token(rest, &ctx)?.ok_or_else(|| TimeError::Unknown {
            token: (*first).to_owned(),
        })?;
        let text = rest[..used].join(" ");
        match token {
            Token::Anchor(moment) => place_once(&mut parts.anchor, moment, text, "now or next")?,
            Token::Time(time) => place_once(&mut parts.time, time, text, "time")?,
            Token::Day(day) => place_once(&mut parts.day, day, text, "day")?,
            Token::Zone(zone) => place_once(&mut parts.zone, zone, text, "zone")?,
        }
        rest = &rest[used..];
    }
    assemble(parts, &ctx)
}

fn read_token(tokens: &[&str], ctx: &Context) -> Result<Option<(Token, usize)>, TimeError> {
    let first = tokens[0];
    let lower = first.to_ascii_lowercase();
    let one = |token| Ok(Some((token, 1)));
    match lower.as_str() {
        "now" => return one(Token::Anchor(Moment::Now)),
        "next" => return one(Token::Anchor(Moment::Next)),
        "today" => return one(Token::Day(ctx.today)),
        "tomorrow" => return one(Token::Day(ctx.today + ChronoDays::new(1))),
        _ => {}
    }
    if let Some((start, end)) = parse_local_range(first)? {
        return one(Token::Time(TimeOfDay::Range(start, end)));
    }
    if let Some(minutes) = parse_clock(first) {
        return one(Token::Time(TimeOfDay::Clock(minutes)));
    }
    if let Some(day) = parse_weekday(first) {
        let ahead =
            (day.num_days_from_monday() + 7 - ctx.today.weekday().num_days_from_monday()) % 7;
        return one(Token::Day(ctx.today + ChronoDays::new(u64::from(ahead))));
    }
    if let Ok(date) = NaiveDate::parse_from_str(first, "%Y-%m-%d") {
        return one(Token::Day(date));
    }
    if let (Ok(day), Some(month)) = (
        first.parse::<u32>(),
        tokens.get(1).and_then(|m| parse_month(m)),
    ) {
        return day_month(day, month, ctx.today)
            .map(|date| Some((Token::Day(date), 2)))
            .ok_or_else(|| TimeError::BadDate {
                text: tokens[..2].join(" "),
            });
    }
    Ok(resolve_zone(first, ctx)?.map(|zone| (Token::Zone(zone), 1)))
}

/// The next date on or after `today` with this day and month.
fn day_month(day: u32, month: u32, today: NaiveDate) -> Option<NaiveDate> {
    let this_year = NaiveDate::from_ymd_opt(today.year(), month, day);
    match this_year {
        Some(date) if date >= today => Some(date),
        _ => NaiveDate::from_ymd_opt(today.year() + 1, month, day),
    }
}

fn utc_offset_label(seconds: i32) -> String {
    let sign = if seconds < 0 { '-' } else { '+' };
    let (hours, minutes) = (seconds.abs() / 3600, seconds.abs() % 3600 / 60);
    if minutes == 0 {
        format!("UTC{sign}{hours}")
    } else {
        format!("UTC{sign}{hours}:{minutes:02}")
    }
}

/// Whether any zone uses `token` as an abbreviation now or half a year from now.
fn is_abbreviation(token: &str, now: DateTime<Utc>) -> bool {
    let half_year_on = now + chrono::Duration::days(182);
    TZ_VARIANTS.iter().any(|zone| {
        [now, half_year_on].iter().any(|instant| {
            zone.offset_from_utc_datetime(&instant.naive_utc())
                .abbreviation()
                .is_some_and(|abbr| abbr.eq_ignore_ascii_case(token))
        })
    })
}

/// The one zone, of `zones`, whose abbreviation at the typed instant is `abbreviation`.
///
/// `start_in` gives the typed instant read in a candidate zone. Several matching UTC offsets
/// are an ambiguity error naming the first zone found for each.
fn match_abbreviation(
    abbreviation: &str,
    zones: &[Tz],
    start_in: impl Fn(Tz) -> Result<DateTime<Utc>, TimeError>,
) -> Result<Option<Tz>, TimeError> {
    let mut by_offset: Vec<(i32, Tz)> = Vec::new();
    for zone in zones {
        let Ok(instant) = start_in(*zone) else {
            continue;
        };
        let offset = zone.offset_from_utc_datetime(&instant.naive_utc());
        let matches = offset
            .abbreviation()
            .is_some_and(|abbr| abbr.eq_ignore_ascii_case(abbreviation));
        let seconds = offset.fix().local_minus_utc();
        if matches && !by_offset.iter().any(|(known, _)| *known == seconds) {
            by_offset.push((seconds, *zone));
        }
    }
    match by_offset.as_slice() {
        [] => Ok(None),
        [(_, zone)] => Ok(Some(*zone)),
        several => Err(TimeError::AmbiguousAbbreviation {
            abbreviation: abbreviation.to_owned(),
            candidates: several
                .iter()
                .map(|(seconds, zone)| format!("{} ({})", zone.name(), utc_offset_label(*seconds)))
                .collect(),
        }),
    }
}

/// Every chrono-tz zone, with the canonical zones the bundled cities use ahead of legacy links.
fn ranked_zones() -> Vec<Tz> {
    let listed: Vec<Tz> = cities::all().iter().map(|city| city.tz).collect();
    let mut zones = TZ_VARIANTS.to_vec();
    zones.sort_by_key(|zone| !listed.contains(zone));
    zones
}

/// Resolves an abbreviation at the typed instant: configured zones first, then every zone.
fn resolve_abbreviation(
    abbreviation: &str,
    ctx: &Context,
    date: NaiveDate,
    start_in: impl Fn(Tz) -> Result<DateTime<Utc>, TimeError> + Copy,
) -> Result<Tz, TimeError> {
    match match_abbreviation(abbreviation, &ctx.roster.zones(), start_in)? {
        Some(zone) => Ok(zone),
        None => match_abbreviation(abbreviation, &ranked_zones(), start_in)?.ok_or_else(|| {
            TimeError::AbbreviationNotInUse {
                abbreviation: abbreviation.to_owned(),
                date,
            }
        }),
    }
}

/// Reads a zone token: `@place` (then `@city`), an abbreviation, an IANA name or a city.
fn resolve_zone(token: &str, ctx: &Context) -> Result<Option<ZoneToken>, TimeError> {
    if let Some(name) = token.strip_prefix('@') {
        if let Some(place) = ctx.roster.place_named(name) {
            return Ok(Some(ZoneToken::Fixed(ctx.roster.place(place).location.tz)));
        }
        return Ok(Some(ZoneToken::Fixed(cities::lookup(name)?.tz)));
    }
    if token.starts_with(['+', '-']) || token.is_empty() {
        return Ok(None);
    }
    if is_abbreviation(token, ctx.now) {
        return Ok(Some(ZoneToken::Abbreviation(token.to_ascii_lowercase())));
    }
    if let Some(zone) = TZ_VARIANTS
        .iter()
        .find(|zone| zone.name().eq_ignore_ascii_case(token))
    {
        return Ok(Some(ZoneToken::Fixed(*zone)));
    }
    match cities::lookup(token) {
        Ok(city) => Ok(Some(ZoneToken::Fixed(city.tz))),
        Err(CityError::Unknown { .. }) => Ok(None),
        Err(error) => Err(error.into()),
    }
}

fn at_local(zone: Tz, date: NaiveDate, minutes: u32) -> Result<DateTime<Utc>, TimeError> {
    let time =
        NaiveTime::from_num_seconds_from_midnight_opt(minutes * 60, 0).unwrap_or(NaiveTime::MIN);
    match zone.from_local_datetime(&date.and_time(time)) {
        LocalResult::Single(local) | LocalResult::Ambiguous(local, _) => {
            Ok(local.with_timezone(&Utc))
        }
        LocalResult::None => Err(TimeError::Skipped {
            time: time.format("%H:%M").to_string(),
            zone: zone.name().to_owned(),
            date,
        }),
    }
}

fn assemble(parts: Parts, ctx: &Context) -> Result<Moment, TimeError> {
    if let Some((anchor, anchor_text)) = parts.anchor {
        let other = [
            parts.time.map(|p| p.1),
            parts.day.map(|p| p.1),
            parts.zone.map(|p| p.1),
        ]
        .into_iter()
        .flatten()
        .next();
        return match other {
            Some(other) => Err(TimeError::Standalone {
                anchor: anchor_text,
                other,
            }),
            None => Ok(anchor),
        };
    }
    let time = parts.time.map(|(time, _)| time);
    let day = parts.day.map(|(day, _)| day);
    let start_in = |zone: Tz| match (time, day) {
        (None, None) => Ok(ctx.now),
        (None, Some(day)) => {
            let minutes = ctx
                .now
                .with_timezone(&zone)
                .time()
                .signed_duration_since(NaiveTime::MIN)
                .num_minutes();
            at_local(zone, day, u32::try_from(minutes).unwrap_or(0))
        }
        (Some(time), day) => at_local(zone, day.unwrap_or(ctx.today), time.start()),
    };
    let zone = match parts.zone {
        None => ctx.roster.axis(),
        Some((ZoneToken::Fixed(zone), _)) => zone,
        Some((ZoneToken::Abbreviation(abbreviation), _)) => {
            resolve_abbreviation(&abbreviation, ctx, day.unwrap_or(ctx.today), start_in)?
        }
    };
    match (time, day) {
        (None, None) => Ok(Moment::Now),
        (None, Some(_)) | (Some(TimeOfDay::Clock(_)), _) => {
            Ok(Moment::Fixed(When::At(start_in(zone)?)))
        }
        (Some(TimeOfDay::Range(start, end)), day) => {
            let day = day.unwrap_or(ctx.today);
            let end_day = if end <= start {
                day + ChronoDays::new(1)
            } else {
                day
            };
            let (start, end) = (at_local(zone, day, start)?, at_local(zone, end_day, end)?);
            Ok(
                Range::covering(start, end).map_or(Moment::Fixed(When::At(start)), |range| {
                    Moment::Fixed(When::Over(range))
                }),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    const FIXTURE: &str = include_str!("../tests/fixtures/sample.toml");

    fn roster() -> Roster {
        Config::parse(FIXTURE).unwrap().roster
    }

    fn utc(text: &str) -> DateTime<Utc> {
        text.parse().unwrap()
    }

    /// Sat 24 Oct 2026, 9:04am AEDT.
    fn now() -> DateTime<Utc> {
        utc("2026-10-23T22:04:00Z")
    }

    fn at(text: &str) -> DateTime<Utc> {
        match parse_time(text, &roster(), now()).unwrap() {
            Moment::Fixed(When::At(instant)) => instant,
            other => panic!("expected an instant from {text}, got {other:?}"),
        }
    }

    fn over(text: &str) -> (DateTime<Utc>, DateTime<Utc>) {
        match parse_time(text, &roster(), now()).unwrap() {
            Moment::Fixed(When::Over(range)) => (range.start().start(), range.end().start()),
            other => panic!("expected a range from {text}, got {other:?}"),
        }
    }

    fn error(text: &str) -> String {
        parse_time(text, &roster(), now()).unwrap_err().to_string()
    }

    #[test]
    fn clock_forms_all_read_in_the_axis_zone_today() {
        let three_pm = utc("2026-10-24T04:00:00Z");
        assert_eq!(at("3pm"), three_pm);
        assert_eq!(at("15:00"), three_pm);
        assert_eq!(at("1500"), three_pm);
        assert_eq!(at("3:30pm"), utc("2026-10-24T04:30:00Z"));
        assert_eq!(at("noon"), utc("2026-10-24T01:00:00Z"));
    }

    #[test]
    fn a_lone_midnight_is_12am_at_the_start_of_the_day() {
        assert_eq!(at("midnight"), utc("2026-10-23T13:00:00Z"));
    }

    #[test]
    fn a_time_already_passed_today_stays_today() {
        assert_eq!(at("8am"), utc("2026-10-23T21:00:00Z"));
    }

    #[test]
    fn range_forms_parse_with_shared_or_explicit_suffixes() {
        let three_to_five = (utc("2026-10-24T04:00:00Z"), utc("2026-10-24T06:00:00Z"));
        assert_eq!(over("3-5pm"), three_to_five);
        assert_eq!(over("3pm-5pm"), three_to_five);
        assert_eq!(over("15:00-17:00"), three_to_five);
    }

    #[test]
    fn eleven_to_one_pm_starts_in_the_morning() {
        assert_eq!(
            over("11-1pm"),
            (utc("2026-10-24T00:00:00Z"), utc("2026-10-24T02:00:00Z"))
        );
    }

    #[test]
    fn ten_pm_to_two_am_crosses_midnight() {
        assert_eq!(
            over("10pm-2am"),
            (utc("2026-10-24T11:00:00Z"), utc("2026-10-24T15:00:00Z"))
        );
    }

    #[test]
    fn a_bare_range_asks_for_am_pm_or_hh_mm() {
        assert_eq!(
            error("9-5"),
            "9-5 needs am/pm or HH:MM, such as 9am-5pm or 09:00-17:00"
        );
        assert_eq!(
            error("15-17"),
            "15-17 needs am/pm or HH:MM, such as 9am-5pm or 09:00-17:00"
        );
    }

    #[test]
    fn today_and_tomorrow_name_the_axis_date() {
        assert_eq!(at("today 3pm"), utc("2026-10-24T04:00:00Z"));
        assert_eq!(at("tomorrow 3pm"), utc("2026-10-25T04:00:00Z"));
    }

    #[test]
    fn a_weekday_is_the_next_one_on_or_after_today() {
        assert_eq!(at("mon 3pm"), utc("2026-10-26T04:00:00Z"));
        assert_eq!(at("3pm monday"), utc("2026-10-26T04:00:00Z"));
        assert_eq!(at("sat 3pm"), utc("2026-10-24T04:00:00Z"));
    }

    #[test]
    fn mon_on_a_monday_is_today() {
        let monday_noon = utc("2026-10-26T01:00:00Z");
        let moment = parse_time("mon 3pm", &roster(), monday_noon).unwrap();
        assert_eq!(moment, Moment::Fixed(When::At(utc("2026-10-26T04:00:00Z"))));
    }

    #[test]
    fn day_month_is_its_next_occurrence_and_iso_dates_are_exact() {
        assert_eq!(at("26 oct 3pm"), utc("2026-10-26T04:00:00Z"));
        assert_eq!(at("1 jan 3pm"), utc("2027-01-01T04:00:00Z"));
        assert_eq!(at("2026-10-26 3pm"), utc("2026-10-26T04:00:00Z"));
        assert_eq!(error("31 feb"), "31 feb is not a date");
    }

    #[test]
    fn a_day_without_a_time_keeps_the_time_of_now() {
        assert_eq!(at("mon"), utc("2026-10-25T22:04:00Z"));
    }

    #[test]
    fn zones_resolve_from_abbreviations_cities_places_and_iana_names() {
        let eight_am_sast = utc("2026-10-24T06:00:00Z");
        assert_eq!(at("8am sast"), eight_am_sast);
        assert_eq!(at("8am johannesburg"), eight_am_sast);
        assert_eq!(at("8am Africa/Johannesburg"), eight_am_sast);
        assert_eq!(at("8am @uk"), utc("2026-10-24T07:00:00Z"));
    }

    #[test]
    fn an_abbreviation_with_several_offsets_is_an_error_listing_candidates() {
        let message = error("3pm cst");
        assert!(message.starts_with("cst is ambiguous: "), "{message}");
        assert!(
            message.contains("(UTC-6)") && message.contains("(UTC+8)"),
            "{message}"
        );
    }

    #[test]
    fn an_abbreviation_is_matched_on_the_typed_date_not_today() {
        assert_eq!(error("3pm tomorrow bst"), "bst is not in use on Sun 25 Oct");
        assert_eq!(at("3pm tomorrow gmt"), utc("2026-10-25T15:00:00Z"));
        assert_eq!(error("9am 2 nov cdt"), "cdt is not in use on Mon 2 Nov");
        assert_eq!(at("9am 2 nov cst"), utc("2026-11-02T15:00:00Z"));
    }

    #[test]
    fn ambiguity_candidates_use_canonical_zone_names() {
        assert_eq!(
            error("3pm ist"),
            "ist is ambiguous: Asia/Kolkata (UTC+5:30), Europe/Dublin (UTC+1)"
        );
    }

    #[test]
    fn now_and_next_stand_alone() {
        assert_eq!(parse_time("now", &roster(), now()).unwrap(), Moment::Now);
        assert_eq!(parse_time("next", &roster(), now()).unwrap(), Moment::Next);
        assert_eq!(parse_time("", &roster(), now()).unwrap(), Moment::Now);
        assert_eq!(error("next mon"), "next stands alone, so drop mon");
    }

    #[test]
    fn each_kind_appears_at_most_once() {
        assert_eq!(
            error("mon tue"),
            "tue: only one day per time, and mon is already one"
        );
    }

    #[test]
    fn an_unknown_token_is_named() {
        assert_eq!(error("3pm blah"), "blah is not a time, a day or a zone");
    }
}
