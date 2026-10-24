//! Loading and validating `config.toml` into a [`Roster`] of resolved people, places and groups.

use std::collections::BTreeMap;

use chrono::{NaiveDate, Weekday};
use chrono_tz::Tz;
use serde::Deserialize;
use thiserror::Error;
use toml::Spanned;

use crate::cities::{self, CityError};
use crate::schedule::{
    Daylight, Days, Hours, Level, LocalSpan, Location, ScheduleError, Trading, TradingDay,
};

/// A config error: the line it is on and the reason, displayed as `line N: reason`.
#[derive(Clone, Debug, Error, PartialEq)]
#[error("line {line}: {reason}")]
pub struct ConfigError {
    pub line: usize,
    pub reason: Reason,
}

/// Why a config was rejected.
#[derive(Clone, Debug, Error, PartialEq)]
pub enum Reason {
    #[error("unknown key \"{0}\"")]
    UnknownKey(String),
    #[error("{0}")]
    Toml(String),
    #[error("[home] is required")]
    MissingHome,
    #[error("\"{name}\" is already a name on line {first}")]
    Clash { name: String, first: usize },
    #[error(
        "\"{0}\" is reserved: names cannot be all, start with -, or contain +, a comma, @ or whitespace"
    )]
    Reserved(String),
    #[error("group \"{0}\" lists no people, so where = \"context\" has nothing to add context to")]
    ContextWithoutPeople(String),
    #[error("unknown profile \"{0}\"")]
    UnknownProfile(String),
    #[error("profile \"{name}\" is a {kind} profile")]
    WrongProfileKind { name: String, kind: &'static str },
    #[error("profile \"{0}\" needs {1}")]
    IncompleteProfile(String, &'static str),
    #[error("\"{key}\" does not belong in a {kind} profile")]
    ForeignKey {
        key: &'static str,
        kind: &'static str,
    },
    #[error(transparent)]
    City(#[from] CityError),
    #[error("unknown time zone \"{0}\"")]
    UnknownZone(String),
    #[error("give either city or all of label, tz and at")]
    Whereabouts,
    #[error("at = [{0}, {1}] is not a latitude and longitude")]
    BadCoordinates(f64, f64),
    #[error(transparent)]
    Schedule(#[from] ScheduleError),
    #[error("unknown person \"{0}\"")]
    UnknownPerson(String),
    #[error("unknown place \"{0}\"")]
    UnknownPlace(String),
    #[error("\"{0}\" is listed twice")]
    Repeated(String),
    #[error("until must be a date such as 2026-11-01")]
    BadUntil,
    #[error("\"{0}\" is not a colour such as #fb923c")]
    BadColour(String),
}

fn fail<T>(line: usize, reason: impl Into<Reason>) -> Result<T, ConfigError> {
    Err(ConfigError {
        line,
        reason: reason.into(),
    })
}

/// A person (home first) with stable index; see [`Roster::person`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PersonId(usize);

impl PersonId {
    /// The `[home]` person.
    pub const HOME: Self = Self(0);
}

/// A place's stable index; see [`Roster::place`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PlaceId(usize);

/// Someone you tell: a resolved `[home]` or `[[person]]`.
#[derive(Clone, Debug, PartialEq)]
pub struct Person {
    pub name: String,
    /// The city name, or the `label` of a city the bundled list lacks.
    pub city: String,
    pub location: Location,
    pub hours: Option<Hours>,
}

/// A region we operate in: a resolved `[[place]]`.
#[derive(Clone, Debug, PartialEq)]
pub struct Place {
    pub name: String,
    pub location: Location,
    pub trading: Option<Trading>,
    pub profile: Option<String>,
}

/// What a group's places are for: counted when off-peak, counted when quiet, or context only.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Rule {
    #[default]
    OffPeak,
    Quiet,
    Context,
}

impl Rule {
    /// Whether a counted place at `level` qualifies under this rule; under `context` none does.
    pub fn admits(self, level: Level) -> bool {
        match (self, level) {
            (Self::OffPeak, Level::OffPeak | Level::Quiet) | (Self::Quiet, Level::Quiet) => true,
            (Self::OffPeak, Level::Peak)
            | (Self::Quiet, Level::Peak | Level::OffPeak)
            | (Self::Context, Level::Peak | Level::OffPeak | Level::Quiet) => false,
        }
    }
}

/// A named, ordered list of people and places.
#[derive(Clone, Debug, PartialEq)]
pub struct Group {
    pub name: String,
    pub aliases: Vec<String>,
    pub people: Vec<PersonId>,
    pub places: Vec<PlaceId>,
    pub rule: Rule,
}

/// Everyone and everything the config names, resolved, in config order.
#[derive(Clone, Debug, PartialEq)]
pub struct Roster {
    pub daylight: Daylight,
    people: Vec<Person>,
    places: Vec<Place>,
    pub groups: Vec<Group>,
}

impl Roster {
    /// The zone typed times are read in and every copy starts with.
    pub fn axis(&self) -> Tz {
        self.person(PersonId::HOME).location.tz
    }

    pub fn person(&self, id: PersonId) -> &Person {
        &self.people[id.0]
    }

    pub fn place(&self, id: PlaceId) -> &Place {
        &self.places[id.0]
    }

    /// Every person, home first, in config order.
    pub fn people(&self) -> impl Iterator<Item = (PersonId, &Person)> {
        self.people
            .iter()
            .enumerate()
            .map(|(index, person)| (PersonId(index), person))
    }

    /// Every place in config order.
    pub fn places(&self) -> impl Iterator<Item = (PlaceId, &Place)> {
        self.places
            .iter()
            .enumerate()
            .map(|(index, place)| (PlaceId(index), place))
    }

    pub fn person_named(&self, name: &str) -> Option<PersonId> {
        self.people()
            .find(|(_, person)| person.name.eq_ignore_ascii_case(name))
            .map(|(id, _)| id)
    }

    pub fn place_named(&self, name: &str) -> Option<PlaceId> {
        self.places()
            .find(|(_, place)| place.name.eq_ignore_ascii_case(name))
            .map(|(id, _)| id)
    }

    /// The group with this name or alias, ignoring case.
    pub fn group_named(&self, name: &str) -> Option<&Group> {
        self.groups.iter().find(|group| {
            group.name.eq_ignore_ascii_case(name)
                || group
                    .aliases
                    .iter()
                    .any(|alias| alias.eq_ignore_ascii_case(name))
        })
    }

    /// Every configured zone, home first, then people and places in config order.
    pub fn zones(&self) -> Vec<Tz> {
        self.people
            .iter()
            .map(|p| p.location.tz)
            .chain(self.places.iter().map(|p| p.location.tz))
            .collect()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MapShow {
    #[default]
    Auto,
    Always,
    Never,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Scheme {
    #[default]
    Dark,
    Light,
    Mono,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Glyphs {
    #[default]
    Full,
    Ascii,
}

/// The `[theme]` table: a scheme, a glyph set and per-role colour overrides as `#rrggbb`.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Theme {
    pub scheme: Scheme,
    pub glyphs: Glyphs,
    pub overrides: BTreeMap<String, String>,
}

/// A dated travel override from `[travel]`.
#[derive(Clone, Debug, PartialEq)]
pub struct Travel {
    pub here: String,
    pub location: Location,
    pub until: NaiveDate,
}

impl Travel {
    /// Whether the override holds at `now`: through the end of `until` in the travel city's local date.
    pub fn is_active(&self, now: chrono::DateTime<chrono::Utc>) -> bool {
        now.with_timezone(&self.location.tz).date_naive() <= self.until
    }
}

/// A validated config.
#[derive(Clone, Debug, PartialEq)]
pub struct Config {
    pub roster: Roster,
    pub map: MapShow,
    pub travel: Option<Travel>,
    pub theme: Theme,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawConfig {
    bands: Option<RawBands>,
    home: Option<RawPerson>,
    #[serde(default)]
    person: Vec<RawPerson>,
    #[serde(default)]
    place: Vec<RawPlace>,
    #[serde(default)]
    profile: Vec<RawProfile>,
    #[serde(default)]
    group: Vec<RawGroup>,
    map: Option<RawMap>,
    travel: Option<RawTravel>,
    theme: Option<RawTheme>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawBands {
    source: Option<Spanned<RawSource>>,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum RawSource {
    Sun,
    Clock,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawPerson {
    name: Spanned<String>,
    city: Option<Spanned<String>>,
    label: Option<Spanned<String>>,
    tz: Option<Spanned<String>>,
    at: Option<Spanned<[f64; 2]>>,
    hours: Option<Spanned<String>>,
    days: Option<Spanned<String>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawPlace {
    name: Spanned<String>,
    tz: Spanned<String>,
    trading: Option<Spanned<String>>,
    at: Spanned<[f64; 2]>,
}

#[derive(Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum ProfileKind {
    Hours,
    Trading,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawProfile {
    name: Spanned<String>,
    kind: Spanned<ProfileKind>,
    hours: Option<Spanned<String>>,
    days: Option<Spanned<String>>,
    open: Option<Spanned<String>>,
    peak: Option<Vec<Spanned<String>>>,
    #[serde(default)]
    on: Vec<RawProfileOn>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawProfileOn {
    days: Spanned<String>,
    open: Option<Spanned<String>>,
    peak: Option<Vec<Spanned<String>>>,
    closed: Option<bool>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawGroup {
    name: Spanned<String>,
    #[serde(default)]
    aliases: Vec<Spanned<String>>,
    #[serde(default)]
    people: Vec<Spanned<String>>,
    #[serde(default)]
    places: Vec<Spanned<String>>,
    #[serde(rename = "where", default)]
    rule: Rule,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawMap {
    show: Option<MapShow>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawTravel {
    here: Spanned<String>,
    until: Spanned<toml::value::Datetime>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawTheme {
    scheme: Option<Scheme>,
    glyphs: Option<Glyphs>,
    text: Option<Spanned<String>>,
    dim: Option<Spanned<String>>,
    accent: Option<Spanned<String>>,
    marker: Option<Spanned<String>>,
    ok: Option<Spanned<String>>,
    warn: Option<Spanned<String>>,
    danger: Option<Spanned<String>>,
    day: Option<Spanned<String>>,
    night: Option<Spanned<String>>,
    dawn: Option<Spanned<String>>,
    work: Option<Spanned<String>>,
    quiet: Option<Spanned<String>>,
    night_tint: Option<Spanned<String>>,
    selection: Option<Spanned<String>>,
}

/// Turns byte offsets into 1-based line numbers.
struct Lines<'a>(&'a str);

impl Lines<'_> {
    fn at(&self, offset: usize) -> usize {
        self.0
            .get(..offset)
            .map_or(1, |head| head.matches('\n').count() + 1)
    }

    fn of<T>(&self, spanned: &Spanned<T>) -> usize {
        self.at(spanned.span().start)
    }
}

/// The text of a toml error without the source excerpt toml appends.
fn toml_reason(error: &toml::de::Error) -> Reason {
    let message = error
        .message()
        .lines()
        .next()
        .unwrap_or_default()
        .to_owned();
    match message
        .strip_prefix("unknown field `")
        .and_then(|rest| rest.split_once('`'))
    {
        Some((key, _)) => Reason::UnknownKey(key.to_owned()),
        None => Reason::Toml(message),
    }
}

impl Config {
    /// Parses and validates config text.
    ///
    /// # Errors
    ///
    /// A [`ConfigError`] naming the line and the [`Reason`]: a toml or type error, an unknown
    /// key, a name clash, a reserved name, an unknown profile, person, place, zone or city,
    /// an ambiguous city, a bad range or day list, or `context` on a group without people.
    pub fn parse(text: &str) -> Result<Self, ConfigError> {
        let lines = Lines(text);
        let raw: RawConfig = toml::from_str(text).or_else(|error| {
            fail(
                error.span().map_or(1, |span| lines.at(span.start)),
                toml_reason(&error),
            )
        })?;
        build(raw, &lines)
    }

    /// The config for `text`, where no file, or one holding only comments, means the defaults.
    ///
    /// # Errors
    ///
    /// The [`ConfigError`] from [`Config::parse`] for any text with a table in it.
    pub fn load(text: Option<&str>, home_zone: Tz) -> Result<Self, ConfigError> {
        match text {
            Some(text) if !is_blank(text) => Self::parse(text),
            Some(_) | None => Ok(Self::without_file(home_zone)),
        }
    }

    /// Home in `zone`, named after it, with no hours, and no people, places or groups.
    fn without_file(zone: Tz) -> Self {
        let city = zone
            .name()
            .rsplit('/')
            .next()
            .unwrap_or_default()
            .replace('_', " ");
        let bundled = cities::all()
            .iter()
            .filter(|known| known.tz == zone)
            .min_by_key(|known| known.name != city);
        let home = Person {
            name: city.clone(),
            city,
            location: Location {
                tz: zone,
                lat: bundled.map_or(0.0, |known| known.lat),
                lon: bundled.map_or(0.0, |known| known.lon),
            },
            hours: None,
        };
        Self {
            roster: Roster {
                daylight: Daylight::Sun,
                people: vec![home],
                places: Vec::new(),
                groups: Vec::new(),
            },
            map: MapShow::default(),
            travel: None,
            theme: Theme::default(),
        }
    }
}

/// Whether `text` holds no tables or keys, only comments and whitespace.
fn is_blank(text: &str) -> bool {
    text.lines()
        .map(str::trim_start)
        .all(|line| line.is_empty() || line.starts_with('#'))
}

fn build(raw: RawConfig, lines: &Lines) -> Result<Config, ConfigError> {
    let RawConfig {
        bands,
        home,
        person,
        place,
        profile,
        group,
        map,
        travel,
        theme,
    } = raw;
    let home = home.map_or_else(|| fail(1, Reason::MissingHome), Ok)?;
    let daylight = match bands
        .and_then(|RawBands { source }| source)
        .map(|source| *source.get_ref())
    {
        None | Some(RawSource::Sun) => Daylight::Sun,
        Some(RawSource::Clock) => Daylight::Clock,
    };
    check_names(&home, &person, &place, &profile, &group, lines)?;
    let profiles = profile
        .iter()
        .map(|raw| resolve_profile(raw, lines))
        .collect::<Result<Vec<_>, _>>()?;
    let profiles = profiles.as_slice();
    let people = std::iter::once(&home)
        .chain(&person)
        .map(|raw| resolve_person(raw, profiles, lines))
        .collect::<Result<Vec<_>, _>>()?;
    let places = place
        .iter()
        .map(|raw| resolve_place(raw, profiles, lines))
        .collect::<Result<Vec<_>, _>>()?;
    let mut roster = Roster {
        daylight,
        people,
        places,
        groups: Vec::new(),
    };
    roster.groups = group
        .iter()
        .map(|raw| resolve_group(raw, &roster, lines))
        .collect::<Result<_, _>>()?;
    Ok(Config {
        roster,
        map: map.and_then(|RawMap { show }| show).unwrap_or_default(),
        travel: travel.map(|raw| resolve_travel(&raw, lines)).transpose()?,
        theme: theme
            .map(|raw| resolve_theme(raw, lines))
            .transpose()?
            .unwrap_or_default(),
    })
}

fn is_reserved(name: &str) -> bool {
    name.eq_ignore_ascii_case("all")
        || name.is_empty()
        || name.starts_with('-')
        || name
            .chars()
            .any(|c| matches!(c, '+' | ',' | '@') || c.is_whitespace())
}

/// Checks reserved names, then clashes within each namespace, naming both lines.
fn check_names(
    home: &RawPerson,
    people: &[RawPerson],
    places: &[RawPlace],
    profiles: &[RawProfile],
    groups: &[RawGroup],
    lines: &Lines,
) -> Result<(), ConfigError> {
    let shared = std::iter::once(&home.name)
        .chain(people.iter().map(|p| &p.name))
        .chain(
            groups
                .iter()
                .flat_map(|g| std::iter::once(&g.name).chain(&g.aliases)),
        );
    let namespaces: [Vec<&Spanned<String>>; 3] = [
        shared.collect(),
        places.iter().map(|p| &p.name).collect(),
        profiles.iter().map(|p| &p.name).collect(),
    ];
    for names in &namespaces {
        let mut seen: Vec<&Spanned<String>> = Vec::new();
        for name in names {
            if is_reserved(name.get_ref()) {
                return fail(lines.of(name), Reason::Reserved(name.get_ref().clone()));
            }
            if let Some(first) = seen
                .iter()
                .find(|seen| seen.get_ref().eq_ignore_ascii_case(name.get_ref()))
            {
                return fail(
                    lines.of(name),
                    Reason::Clash {
                        name: name.get_ref().clone(),
                        first: lines.of(first),
                    },
                );
            }
            seen.push(name);
        }
    }
    Ok(())
}

/// A `[[profile]]` resolved once, whether or not anything references it.
#[derive(Clone, Debug)]
enum Profile {
    Hours { span: LocalSpan, days: Option<Days> },
    Trading(Box<Trading>),
}

impl Profile {
    fn kind(&self) -> &'static str {
        match self {
            Self::Hours { .. } => "hours",
            Self::Trading(_) => "trading",
        }
    }
}

/// A resolved profile and the name it was declared with.
struct NamedProfile<'a> {
    name: &'a str,
    profile: Profile,
}

/// The profile `name` refers to, ignoring case like every other name.
fn find_profile<'a>(profiles: &'a [NamedProfile], name: &str) -> Option<&'a NamedProfile<'a>> {
    profiles
        .iter()
        .find(|profile| profile.name.eq_ignore_ascii_case(name))
}

fn span(text: &Spanned<String>, lines: &Lines) -> Result<LocalSpan, ConfigError> {
    LocalSpan::parse(text.get_ref()).or_else(|error| fail(lines.of(text), error))
}

fn days(text: &Spanned<String>, lines: &Lines) -> Result<Days, ConfigError> {
    Days::parse(text.get_ref()).or_else(|error| fail(lines.of(text), error))
}

/// Fails on the first key present that belongs to the other kind of profile.
fn reject_foreign(
    keys: &[(&'static str, Option<usize>)],
    kind: &'static str,
) -> Result<(), ConfigError> {
    keys.iter()
        .find_map(|&(key, line)| line.map(|line| (key, line)))
        .map_or(Ok(()), |(key, line)| {
            fail(line, Reason::ForeignKey { key, kind })
        })
}

fn resolve_profile<'a>(
    raw: &'a RawProfile,
    lines: &Lines,
) -> Result<NamedProfile<'a>, ConfigError> {
    let RawProfile {
        name,
        kind,
        hours,
        days: profile_days,
        open,
        peak,
        on,
    } = raw;
    let line_of = |text: &Spanned<String>| lines.of(text);
    let profile = match kind.get_ref() {
        ProfileKind::Hours => {
            reject_foreign(
                &[
                    ("open", open.as_ref().map(line_of)),
                    (
                        "peak",
                        peak.as_ref()
                            .map(|peaks| peaks.first().map_or(lines.of(name), line_of)),
                    ),
                    ("on", on.first().map(|entry| lines.of(&entry.days))),
                ],
                "hours",
            )?;
            let hours = hours.as_ref().map_or_else(
                || {
                    fail(
                        lines.of(name),
                        Reason::IncompleteProfile(name.get_ref().clone(), "hours"),
                    )
                },
                Ok,
            )?;
            Profile::Hours {
                span: span(hours, lines)?,
                days: profile_days
                    .as_ref()
                    .map(|text| days(text, lines))
                    .transpose()?,
            }
        }
        ProfileKind::Trading => {
            reject_foreign(
                &[
                    ("hours", hours.as_ref().map(line_of)),
                    ("days", profile_days.as_ref().map(line_of)),
                ],
                "trading",
            )?;
            Profile::Trading(Box::new(resolve_trading(
                name,
                open.as_ref(),
                peak.as_ref(),
                on,
                lines,
            )?))
        }
    };
    Ok(NamedProfile {
        name: name.get_ref(),
        profile,
    })
}

/// A range starts with a digit or is noon/midnight; anything else names a profile.
fn looks_like_range(text: &str) -> bool {
    text.starts_with(|c: char| c.is_ascii_digit())
        || text.starts_with("noon")
        || text.starts_with("midnight")
}

fn wrong_kind(name: &Spanned<String>, profile: &Profile, lines: &Lines) -> ConfigError {
    ConfigError {
        line: lines.of(name),
        reason: Reason::WrongProfileKind {
            name: name.get_ref().clone(),
            kind: profile.kind(),
        },
    }
}

/// A person's hours: a profile of that name if there is one, otherwise a range.
fn resolve_hours(
    hours: Option<&Spanned<String>>,
    own_days: Option<&Spanned<String>>,
    profiles: &[NamedProfile],
    lines: &Lines,
) -> Result<Option<Hours>, ConfigError> {
    let Some(hours) = hours else {
        return Ok(None);
    };
    let own_days = own_days.map(|text| days(text, lines)).transpose()?;
    let (span, profile_days) = match find_profile(profiles, hours.get_ref()) {
        Some(NamedProfile {
            profile: Profile::Hours { span, days },
            ..
        }) => (*span, *days),
        Some(NamedProfile { profile, .. }) => return Err(wrong_kind(hours, profile, lines)),
        None if looks_like_range(hours.get_ref()) => (span(hours, lines)?, None),
        None => {
            return fail(
                lines.of(hours),
                Reason::UnknownProfile(hours.get_ref().clone()),
            );
        }
    };
    Ok(Some(Hours {
        span,
        days: own_days.or(profile_days).unwrap_or(Days::MON_FRI),
    }))
}

fn zone(text: &Spanned<String>, lines: &Lines) -> Result<Tz, ConfigError> {
    text.get_ref()
        .parse()
        .or_else(|_| fail(lines.of(text), Reason::UnknownZone(text.get_ref().clone())))
}

fn coordinates(at: &Spanned<[f64; 2]>, lines: &Lines) -> Result<(f64, f64), ConfigError> {
    let [lat, lon] = *at.get_ref();
    if (-90.0..=90.0).contains(&lat) && (-180.0..=180.0).contains(&lon) {
        Ok((lat, lon))
    } else {
        fail(lines.of(at), Reason::BadCoordinates(lat, lon))
    }
}

fn resolve_person(
    raw: &RawPerson,
    profiles: &[NamedProfile],
    lines: &Lines,
) -> Result<Person, ConfigError> {
    let RawPerson {
        name,
        city,
        label,
        tz,
        at,
        hours,
        days,
    } = raw;
    let (city, location) = match (city, label, tz, at) {
        (Some(city), None, None, None) => {
            let found =
                cities::lookup(city.get_ref()).or_else(|error| fail(lines.of(city), error))?;
            (
                found.name.to_owned(),
                Location {
                    tz: found.tz,
                    lat: found.lat,
                    lon: found.lon,
                },
            )
        }
        (None, Some(label), Some(tz), Some(at)) => {
            let (lat, lon) = coordinates(at, lines)?;
            (
                label.get_ref().clone(),
                Location {
                    tz: zone(tz, lines)?,
                    lat,
                    lon,
                },
            )
        }
        _ => return fail(lines.of(name), Reason::Whereabouts),
    };
    Ok(Person {
        name: name.get_ref().clone(),
        city,
        location,
        hours: resolve_hours(hours.as_ref(), days.as_ref(), profiles, lines)?,
    })
}

fn resolve_trading(
    name: &Spanned<String>,
    open: Option<&Spanned<String>>,
    peak: Option<&Vec<Spanned<String>>>,
    on: &[RawProfileOn],
    lines: &Lines,
) -> Result<Trading, ConfigError> {
    let open = open.map_or_else(
        || {
            fail(
                lines.of(name),
                Reason::IncompleteProfile(name.get_ref().clone(), "open"),
            )
        },
        |text| span(text, lines),
    )?;
    let spans = |texts: &Vec<Spanned<String>>| {
        texts
            .iter()
            .map(|text| span(text, lines))
            .collect::<Result<Vec<_>, _>>()
    };
    let peaks = peak.map(spans).transpose()?.unwrap_or_default();
    let overrides = on
        .iter()
        .map(|entry| {
            let RawProfileOn {
                days: entry_days,
                open,
                peak,
                closed,
            } = entry;
            let open = open.as_ref().map(|text| span(text, lines)).transpose()?;
            Ok((
                days(entry_days, lines)?,
                open,
                peak.as_ref().map(spans).transpose()?,
                closed.unwrap_or(false),
            ))
        })
        .collect::<Result<Vec<_>, ConfigError>>()?;
    let weekdays = [
        Weekday::Mon,
        Weekday::Tue,
        Weekday::Wed,
        Weekday::Thu,
        Weekday::Fri,
        Weekday::Sat,
        Weekday::Sun,
    ];
    let days = weekdays.map(|weekday| {
        // The narrowest matching entry wins, and a later entry breaks a tie.
        let entry = overrides
            .iter()
            .filter(|(days, ..)| days.contains(weekday))
            .fold(
                None,
                |best: Option<&(Days, _, _, bool)>, entry| match best {
                    Some(best) if best.0.count() < entry.0.count() => Some(best),
                    _ => Some(entry),
                },
            );
        match entry {
            None => TradingDay {
                open: Some(open),
                peaks: peaks.clone(),
            },
            Some((_, _, _, true)) => TradingDay {
                open: None,
                peaks: Vec::new(),
            },
            Some((_, entry_open, entry_peaks, false)) => TradingDay {
                open: Some(entry_open.unwrap_or(open)),
                peaks: entry_peaks.clone().unwrap_or_else(|| peaks.clone()),
            },
        }
    });
    Ok(Trading { days })
}

fn resolve_place(
    raw: &RawPlace,
    profiles: &[NamedProfile],
    lines: &Lines,
) -> Result<Place, ConfigError> {
    let RawPlace {
        name,
        tz,
        trading,
        at,
    } = raw;
    let (lat, lon) = coordinates(at, lines)?;
    let profile = trading
        .as_ref()
        .map(
            |reference| match find_profile(profiles, reference.get_ref()) {
                Some(NamedProfile {
                    name,
                    profile: Profile::Trading(trading),
                }) => Ok(((*name).to_owned(), Trading::clone(trading))),
                Some(NamedProfile { profile, .. }) => Err(wrong_kind(reference, profile, lines)),
                None => fail(
                    lines.of(reference),
                    Reason::UnknownProfile(reference.get_ref().clone()),
                ),
            },
        )
        .transpose()?;
    let (profile, trading) = profile.unzip();
    Ok(Place {
        name: name.get_ref().clone(),
        location: Location {
            tz: zone(tz, lines)?,
            lat,
            lon,
        },
        trading,
        profile,
    })
}

fn members<T: PartialEq + Copy>(
    names: &[Spanned<String>],
    find: impl Fn(&str) -> Option<T>,
    unknown: fn(String) -> Reason,
    lines: &Lines,
) -> Result<Vec<T>, ConfigError> {
    names.iter().try_fold(Vec::new(), |mut found, name| {
        let id = find(name.get_ref())
            .map_or_else(|| fail(lines.of(name), unknown(name.get_ref().clone())), Ok)?;
        if found.contains(&id) {
            return fail(lines.of(name), Reason::Repeated(name.get_ref().clone()));
        }
        found.push(id);
        Ok(found)
    })
}

fn resolve_group(raw: &RawGroup, roster: &Roster, lines: &Lines) -> Result<Group, ConfigError> {
    let RawGroup {
        name,
        aliases,
        people,
        places,
        rule,
    } = raw;
    let people = members(
        people,
        |n| roster.person_named(n),
        Reason::UnknownPerson,
        lines,
    )?;
    let places = members(
        places,
        |n| roster.place_named(n),
        Reason::UnknownPlace,
        lines,
    )?;
    if *rule == Rule::Context && people.is_empty() {
        return fail(
            lines.of(name),
            Reason::ContextWithoutPeople(name.get_ref().clone()),
        );
    }
    Ok(Group {
        name: name.get_ref().clone(),
        aliases: aliases
            .iter()
            .map(|alias| alias.get_ref().clone())
            .collect(),
        people,
        places,
        rule: *rule,
    })
}

fn resolve_travel(raw: &RawTravel, lines: &Lines) -> Result<Travel, ConfigError> {
    let RawTravel { here, until } = raw;
    let city = cities::lookup(here.get_ref()).or_else(|error| fail(lines.of(here), error))?;
    let until = match until.get_ref() {
        toml::value::Datetime {
            date: Some(date),
            time: None,
            offset: None,
        } => NaiveDate::from_ymd_opt(
            i32::from(date.year),
            u32::from(date.month),
            u32::from(date.day),
        ),
        _ => None,
    }
    .map_or_else(|| fail(lines.of(until), Reason::BadUntil), Ok)?;
    Ok(Travel {
        here: city.name.to_owned(),
        location: Location {
            tz: city.tz,
            lat: city.lat,
            lon: city.lon,
        },
        until,
    })
}

fn resolve_theme(raw: RawTheme, lines: &Lines) -> Result<Theme, ConfigError> {
    let RawTheme {
        scheme,
        glyphs,
        text,
        dim,
        accent,
        marker,
        ok,
        warn,
        danger,
        day,
        night,
        dawn,
        work,
        quiet,
        night_tint,
        selection,
    } = raw;
    let roles = [
        ("text", text),
        ("dim", dim),
        ("accent", accent),
        ("marker", marker),
        ("ok", ok),
        ("warn", warn),
        ("danger", danger),
        ("day", day),
        ("night", night),
        ("dawn", dawn),
        ("work", work),
        ("quiet", quiet),
        ("night_tint", night_tint),
        ("selection", selection),
    ];
    let is_hex = |value: &str| {
        value.len() == 7
            && value.starts_with('#')
            && value[1..].bytes().all(|b| b.is_ascii_hexdigit())
    };
    let overrides = roles
        .into_iter()
        .filter_map(|(role, value)| value.map(|value| (role, value)))
        .map(|(role, value)| {
            if is_hex(value.get_ref()) {
                Ok((role.to_owned(), value.into_inner()))
            } else {
                fail(lines.of(&value), Reason::BadColour(value.get_ref().clone()))
            }
        })
        .collect::<Result<_, _>>()?;
    Ok(Theme {
        scheme: scheme.unwrap_or_default(),
        glyphs: glyphs.unwrap_or_default(),
        overrides,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("../tests/fixtures/sample.toml");

    fn rejection(text: &str) -> ConfigError {
        Config::parse(text).unwrap_err()
    }

    /// The fixture with `from` replaced by `to`, which must occur exactly once.
    fn edited(from: &str, to: &str) -> String {
        assert_eq!(
            FIXTURE.matches(from).count(),
            1,
            "{from:?} must occur once in the fixture"
        );
        FIXTURE.replacen(from, to, 1)
    }

    fn line_of(text: &str, needle: &str) -> usize {
        text.lines().position(|line| line.contains(needle)).unwrap() + 1
    }

    #[test]
    fn the_fixture_parses_into_home_six_people_three_places_and_four_groups() {
        let roster = Config::parse(FIXTURE).unwrap().roster;
        let names: Vec<_> = roster.people().map(|(_, p)| p.name.as_str()).collect();
        assert_eq!(
            names,
            ["Paul", "Priya", "Sam", "Thandi", "Johan", "Ellie", "Marcus"]
        );
        assert_eq!(roster.places().count(), 3);
        let groups: Vec<_> = roster.groups.iter().map(|g| g.name.as_str()).collect();
        assert_eq!(groups, ["team", "au", "regions", "deploy"]);
        assert_eq!(roster.axis(), chrono_tz::Australia::Melbourne);
        assert_eq!(roster.group_named("APPS").unwrap().name, "regions");
    }

    #[test]
    fn a_profile_name_in_hours_takes_the_profile_hours_and_days() {
        let roster = Config::parse(FIXTURE).unwrap().roster;
        let thandi = roster.person(roster.person_named("thandi").unwrap());
        assert_eq!(
            thandi.hours.unwrap().span,
            LocalSpan {
                start: 8 * 60,
                end: 16 * 60
            }
        );
        assert_eq!(
            roster.person(roster.person_named("Ellie").unwrap()).hours,
            None
        );
    }

    #[test]
    fn the_narrowest_on_entry_overrides_peaks_and_keeps_the_base_open() {
        let roster = Config::parse(FIXTURE).unwrap().roster;
        let trading = roster
            .place(roster.place_named("AU").unwrap())
            .trading
            .clone()
            .unwrap();
        let open = Some(LocalSpan {
            start: 7 * 60,
            end: 23 * 60,
        });
        assert_eq!(trading.on(Weekday::Sat).open, open);
        assert_eq!(
            trading.on(Weekday::Sat).peaks[1],
            LocalSpan {
                start: 18 * 60,
                end: 22 * 60
            }
        );
        assert_eq!(
            trading.on(Weekday::Mon).peaks[1],
            LocalSpan {
                start: 17 * 60 + 30,
                end: 21 * 60
            }
        );
    }

    #[test]
    fn an_unknown_key_is_rejected_on_its_line() {
        let text = edited("hours = \"9am-6pm\"", "hour = \"9am-6pm\"");
        let error = rejection(&text);
        assert_eq!(
            error.to_string(),
            format!("line {}: unknown key \"hour\"", line_of(&text, "hour ="))
        );
    }

    #[test]
    fn a_group_named_like_a_person_clashes_naming_both_lines() {
        let text = edited("name   = \"au\"", "name   = \"sam\"");
        let error = rejection(&text);
        let first = line_of(&text, "\"Sam\"");
        assert_eq!(
            error,
            ConfigError {
                line: line_of(&text, "\"sam\""),
                reason: Reason::Clash {
                    name: "sam".into(),
                    first
                }
            }
        );
    }

    #[test]
    fn two_people_with_one_name_clash() {
        let text = edited("name  = \"Johan\"", "name  = \"PRIYA\"");
        assert_eq!(
            rejection(&text).reason,
            Reason::Clash {
                name: "PRIYA".into(),
                first: line_of(&text, "\"Priya\"")
            }
        );
    }

    #[test]
    fn context_on_a_places_only_group_is_rejected() {
        let text = edited(
            "places  = [\"AU\", \"UK\", \"US\"]",
            "places  = [\"AU\", \"UK\", \"US\"]\nwhere   = \"context\"",
        );
        let error = rejection(&text);
        assert_eq!(
            error,
            ConfigError {
                line: line_of(&text, "\"regions\""),
                reason: Reason::ContextWithoutPeople("regions".into())
            }
        );
    }

    #[test]
    fn an_unknown_profile_is_rejected_on_its_line() {
        let text = FIXTURE.replacen("hours = \"jnb-office\"", "hours = \"jnb-offce\"", 1);
        let error = rejection(&text);
        assert_eq!(
            error,
            ConfigError {
                line: line_of(&text, "jnb-offce"),
                reason: Reason::UnknownProfile("jnb-offce".into())
            }
        );
    }

    #[test]
    fn an_unknown_city_is_rejected_on_its_line() {
        let text = edited("city  = \"London\"", "city  = \"Atlantis\"");
        assert_eq!(
            rejection(&text).to_string(),
            format!(
                "line {}: unknown city \"Atlantis\"",
                line_of(&text, "Atlantis")
            )
        );
    }

    #[test]
    fn an_ambiguous_city_is_rejected_listing_candidates() {
        let text = edited("city  = \"London\"", "city  = \"Hyderabad\"");
        assert_eq!(
            rejection(&text).to_string(),
            format!(
                "line {}: city \"Hyderabad\" is ambiguous: Hyderabad, IN; Hyderabad, PK",
                line_of(&text, "Hyderabad")
            )
        );
    }

    #[test]
    fn a_reserved_character_in_a_name_is_rejected() {
        for name in ["Ell+ie", "Ell ie", "El,lie", "@Ellie", "-Ellie", "all"] {
            let text = edited("name  = \"Ellie\"", &format!("name  = \"{name}\""));
            assert_eq!(
                rejection(&text),
                ConfigError {
                    line: line_of(&text, "city  = \"London\"") - 1,
                    reason: Reason::Reserved(name.into())
                }
            );
        }
    }

    #[test]
    fn a_dash_inside_a_name_is_fine() {
        assert!(
            Config::parse(
                &edited("name    = \"US\"", "name    = \"US-E\"").replace("\"US\"]", "\"US-E\"]")
            )
            .is_ok()
        );
    }

    #[test]
    fn a_group_naming_an_unknown_person_is_rejected() {
        let text = edited(
            "people = [\"Priya\", \"Sam\"]",
            "people = [\"Priya\", \"Samuel\"]",
        );
        assert_eq!(
            rejection(&text).reason,
            Reason::UnknownPerson("Samuel".into())
        );
    }

    /// A minimal config whose one place trades on `profile`, given as the body of a `[[profile]]`.
    fn with_trading(profile: &str) -> String {
        format!(
            "[home]\nname = \"Paul\"\ncity = \"Melbourne\"\n\n[[place]]\nname = \"AU\"\n\
             tz = \"Australia/Melbourne\"\ntrading = \"venue\"\nat = [-24.0, 134.0]\n\n\
             [[profile]]\nname = \"venue\"\nkind = \"trading\"\nopen = \"7am-11pm\"\n{profile}"
        )
    }

    fn trading_of(text: &str) -> Trading {
        let roster = Config::parse(text).unwrap().roster;
        let au = roster.place_named("AU").unwrap();
        roster.place(au).trading.clone().unwrap()
    }

    fn peak(start: u32, end: u32) -> Vec<LocalSpan> {
        vec![LocalSpan {
            start: start * 60,
            end: end * 60,
        }]
    }

    #[test]
    fn a_narrower_on_entry_wins_over_a_wider_one_listed_before_it() {
        let trading = trading_of(&with_trading(
            "[[profile.on]]\ndays = \"Mon-Fri\"\npeak = [\"9am-10am\"]\n\
             [[profile.on]]\ndays = \"Mon\"\npeak = [\"1pm-2pm\"]\n",
        ));
        assert_eq!(trading.on(Weekday::Mon).peaks, peak(13, 14));
        assert_eq!(trading.on(Weekday::Tue).peaks, peak(9, 10));
    }

    #[test]
    fn a_narrower_on_entry_wins_over_a_wider_one_listed_after_it() {
        let trading = trading_of(&with_trading(
            "[[profile.on]]\ndays = \"Mon\"\npeak = [\"1pm-2pm\"]\n\
             [[profile.on]]\ndays = \"Mon-Fri\"\npeak = [\"9am-10am\"]\n",
        ));
        assert_eq!(trading.on(Weekday::Mon).peaks, peak(13, 14));
    }

    #[test]
    fn the_later_of_two_equally_wide_on_entries_wins() {
        let trading = trading_of(&with_trading(
            "[[profile.on]]\ndays = \"Mon,Tue\"\npeak = [\"9am-10am\"]\n\
             [[profile.on]]\ndays = \"Mon-Tue\"\npeak = [\"1pm-2pm\"]\n",
        ));
        assert_eq!(trading.on(Weekday::Mon).peaks, peak(13, 14));
    }

    #[test]
    fn a_closed_on_entry_has_no_open_hours_or_peaks() {
        let trading = trading_of(&with_trading(
            "peak = [\"noon-1pm\"]\n[[profile.on]]\ndays = \"Sun\"\nclosed = true\n",
        ));
        let closed = TradingDay {
            open: None,
            peaks: Vec::new(),
        };
        assert_eq!(trading.on(Weekday::Sun), &closed);
        assert_eq!(trading.on(Weekday::Sat).peaks, peak(12, 13));
    }

    #[test]
    fn a_bad_value_in_an_unreferenced_profile_is_rejected() {
        let text = format!(
            "{FIXTURE}\n[[profile]]\nname = \"spare\"\nkind = \"trading\"\nopen = \"garbage\"\n"
        );
        assert_eq!(rejection(&text).line, line_of(&text, "open = \"garbage\""));
    }

    #[test]
    fn a_key_from_the_other_kind_of_profile_is_rejected_on_its_line() {
        let trading_with_days = edited(
            "kind  = \"trading\"",
            "kind  = \"trading\"\ndays  = \"Mon-Fri\"",
        );
        assert_eq!(
            rejection(&trading_with_days),
            ConfigError {
                line: line_of(&trading_with_days, "kind  = \"trading\"") + 1,
                reason: Reason::ForeignKey {
                    key: "days",
                    kind: "trading"
                }
            }
        );
        let hours_with_open = edited(
            "kind  = \"hours\"",
            "kind  = \"hours\"\nopen  = \"7am-11pm\"",
        );
        assert_eq!(
            rejection(&hours_with_open).reason,
            Reason::ForeignKey {
                key: "open",
                kind: "hours"
            }
        );
    }

    #[test]
    fn a_profile_reference_ignores_case_like_every_other_name() {
        let text = edited(
            "trading = \"venue\"\nat      = [54.5",
            "trading = \"Venue\"\nat      = [54.5",
        );
        assert!(Config::parse(&text).is_ok(), "{:?}", Config::parse(&text));
    }

    #[test]
    fn a_profile_whose_name_starts_with_a_digit_is_a_profile_not_a_range() {
        let text = FIXTURE.replace("jnb-office", "24x7");
        let roster = Config::parse(&text).unwrap().roster;
        let thandi = roster.person(roster.person_named("Thandi").unwrap());
        assert_eq!(thandi.hours.unwrap().span.start, 8 * 60);
    }

    #[test]
    fn a_config_without_home_is_rejected() {
        assert_eq!(
            rejection("[bands]\nsource = \"clock\"\n").reason,
            Reason::MissingHome
        );
    }

    const MELBOURNE: Tz = chrono_tz::Australia::Melbourne;

    #[test]
    fn no_file_means_home_in_the_system_zone_with_no_hours_and_nobody_else() {
        let config = Config::load(None, MELBOURNE).unwrap();
        let people: Vec<_> = config.roster.people().map(|(_, p)| p).collect();
        assert_eq!(people.len(), 1);
        assert_eq!(people[0].name, "Melbourne");
        assert_eq!(people[0].location.tz, MELBOURNE);
        assert_eq!(people[0].hours, None);
        assert_eq!(config.roster.places().count(), 0);
        assert!(config.roster.groups.is_empty());
        assert_eq!(config.travel, None);
    }

    #[test]
    fn a_file_of_only_comments_is_the_same_as_no_file() {
        let commented = "# [home]\n# name = \"Kim\"\n\n## prose\n";
        assert_eq!(
            Config::load(Some(commented), MELBOURNE),
            Config::load(None, MELBOURNE)
        );
    }

    #[test]
    fn a_file_with_a_table_but_no_home_is_still_rejected() {
        let error = Config::load(Some("[bands]\nsource = \"clock\"\n"), MELBOURNE).unwrap_err();
        assert_eq!(error.reason, Reason::MissingHome);
    }

    #[test]
    fn home_without_a_file_takes_its_coordinates_from_the_bundled_city_of_that_name() {
        let config = Config::load(None, MELBOURNE).unwrap();
        let home = config.roster.person(PersonId::HOME);
        assert!(
            (home.location.lat - -37.8).abs() < 0.1,
            "{:?}",
            home.location
        );
    }

    #[test]
    fn travel_holds_through_the_end_of_until_in_the_travel_citys_date() {
        let text = format!("{FIXTURE}\n[travel]\nhere = \"London\"\nuntil = 2026-11-01\n");
        let travel = Config::parse(&text).unwrap().travel.unwrap();
        // 11:30pm GMT on 1 Nov is still the 1st in London, though it is the 2nd in Melbourne.
        assert!(travel.is_active("2026-11-01T23:30:00Z".parse().unwrap()));
        assert!(!travel.is_active("2026-11-02T00:00:00Z".parse().unwrap()));
    }
}
