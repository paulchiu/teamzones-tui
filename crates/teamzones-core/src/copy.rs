//! The say line and the copy-as formats: zone entries in copy order, plus the places' clause.

use chrono::{DateTime, NaiveDate, Offset, Timelike, Utc};
use chrono_tz::Tz;
use serde::Deserialize;

use crate::clock::When;
use crate::config::{PersonId, Place, Roster, Rule};
use crate::question::{Marks, Row, place_state};
use crate::schedule::Level;

/// What a line does with the counted places: their states, their zones, or nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Clause {
    #[default]
    States,
    Zones,
    #[serde(rename = "none")]
    Drop,
}

/// How to shape a say line: a date prefix, names and one clause handling, which the CLI may combine.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Variant {
    pub date: bool,
    pub names: bool,
    pub clause: Clause,
}

/// Whose zones a format names: the marks, everyone (`0 all`), or the focused row before the axis.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Who {
    #[default]
    Marked,
    All,
    FocusedFirst,
}

/// One copy-as format: the dialog key that copies it, its description, and the line's shape.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Format {
    pub key: char,
    pub label: String,
    pub who: Who,
    pub variant: Variant,
}

/// The copy-as formats in dialog order, exactly one of which is the default.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Formats {
    pub(crate) formats: Vec<Format>,
    pub(crate) default: usize,
}

/// Keys the copy-as dialog keeps for itself (`?` help and `q` quit), so no format may use them.
pub const DIALOG_KEYS: [char; 2] = ['?', 'q'];

/// The built-in formats as `(key, label, who, date, names, clause)`, with `y` the default.
const BUILTIN: [(char, &str, Who, bool, bool, Clause); 7] = [
    ('y', "the times", Who::Marked, false, false, Clause::States),
    (
        'd',
        "with the date",
        Who::Marked,
        true,
        false,
        Clause::States,
    ),
    ('w', "with names", Who::Marked, false, true, Clause::States),
    (
        't',
        "the focused row's time first",
        Who::FocusedFirst,
        false,
        false,
        Clause::Drop,
    ),
    (
        'z',
        "with each place's time instead of its state",
        Who::Marked,
        false,
        false,
        Clause::Zones,
    ),
    ('s', "times only", Who::Marked, false, false, Clause::Drop),
    ('a', "for everyone", Who::All, false, false, Clause::States),
];

impl Formats {
    /// The formats a config without `[[copy]]` tables has.
    pub fn builtin() -> Self {
        let formats = BUILTIN
            .iter()
            .map(|&(key, label, who, date, names, clause)| Format {
                key,
                label: label.to_owned(),
                who,
                variant: Variant {
                    date,
                    names,
                    clause,
                },
            })
            .collect();
        Self {
            formats,
            default: 0,
        }
    }

    /// Every format, in dialog order.
    pub fn all(&self) -> &[Format] {
        &self.formats
    }

    /// The format that is the times box line, what `y` copies and what `say` prints.
    pub fn default_format(&self) -> &Format {
        self.formats
            .get(self.default)
            .or_else(|| self.formats.first())
            .expect("a loaded config always has a default copy format")
    }

    pub fn get(&self, key: char) -> Option<&Format> {
        self.formats.iter().find(|format| format.key == key)
    }
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

/// A zone a line names, with the names of whoever brought it.
type Source = (Tz, Vec<String>);

/// The axis zone, carrying home's name whenever the marks ask who.
fn axis_source(roster: &Roster, marks: &Marks) -> Source {
    let home = marks
        .asks_who()
        .then(|| roster.person(PersonId::HOME).name.clone());
    (roster.axis(), home.into_iter().collect())
}

/// Home's own zone while travelling, unnamed, whenever the marks ask who.
fn reference_source(roster: &Roster, marks: &Marks) -> Option<Source> {
    roster
        .reference()
        .filter(|_| marks.asks_who())
        .map(|reference| (reference.location.tz, Vec::new()))
}

/// The counted places' zones, which only `Clause::Zones` names.
fn place_sources(roster: &Roster, marks: &Marks, clause: Clause) -> Vec<Source> {
    let places = match clause {
        Clause::Zones => marks.counted_places(),
        Clause::States | Clause::Drop => Vec::new(),
    };
    places
        .into_iter()
        .map(|id| {
            let place = roster.place(id);
            (place.location.tz, vec![place.name.clone()])
        })
        .collect()
}

/// The zones a say line names, axis first, then by descending offset at the start.
fn zone_sources(roster: &Roster, marks: &Marks, when: When, clause: Clause) -> Vec<Source> {
    let people = marks.counted_people().into_iter().skip(1).map(|id| {
        let person = roster.person(id);
        (person.location.tz, vec![person.name.clone()])
    });
    let clocks = marks
        .clocks
        .iter()
        .map(|clock| (clock.tz, vec![clock.name.clone()]));
    let mut sources: Vec<Source> = std::iter::once(axis_source(roster, marks))
        .chain(reference_source(roster, marks))
        .chain(people)
        .chain(clocks)
        .chain(place_sources(roster, marks, clause))
        .collect();
    let start = when.start();
    sources[1..].sort_by_key(|(tz, _)| -offset_seconds(*tz, start));
    sources
}

/// Each source's entry in order, merging entries that render identically so no time repeats.
fn entries(sources: Vec<Source>, when: When, axis_date: NaiveDate, names: bool) -> Vec<String> {
    let mut merged: Vec<(String, Vec<String>)> = Vec::new();
    for (tz, who) in sources {
        let text = entry(when, tz, axis_date);
        match merged.iter_mut().find(|(known, _)| *known == text) {
            Some((_, more)) => more.extend(who),
            None => merged.push((text, who)),
        }
    }
    merged
        .into_iter()
        .map(|(text, who)| {
            if names && !who.is_empty() {
                format!("{text} ({})", who.join(", "))
            } else {
                text
            }
        })
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

/// Levels in clause order: the ones the rule counts first, then the rest.
fn clause_order(rule: Rule) -> &'static [Level] {
    match rule {
        Rule::Open => &[Level::Open, Level::Closed, Level::Busy],
        Rule::Closed => &[Level::Closed, Level::Busy, Level::Open],
        Rule::Context => &[],
    }
}

/// The places' states, as in `open in AU and US, closed in UK`.
///
/// Each place is named by its real state in its own profile's labels, so the line never calls a
/// closed place open. Places that share a state but not its word form separate groups, in
/// config order.
fn clause(roster: &Roster, marks: &Marks, when: When) -> Option<String> {
    let states: Vec<(&Place, Level)> = marks
        .counted_places()
        .into_iter()
        .filter_map(|id| place_state(roster, id, when).map(|level| (roster.place(id), level)))
        .collect();
    let mut groups: Vec<(&str, Vec<&str>)> = Vec::new();
    for &section in clause_order(marks.rule) {
        let first = groups.len();
        let listed = states.iter().filter(|(_, level)| *level == section);
        for (place, _) in listed {
            let word = place.labels().word(section);
            match groups[first..].iter_mut().find(|(known, _)| *known == word) {
                Some((_, names)) => names.push(&place.name),
                None => groups.push((word, vec![&place.name])),
            }
        }
    }
    let parts: Vec<String> = groups
        .into_iter()
        .map(|(word, names)| format!("{word} in {}", and_list(&names)))
        .collect();
    (!parts.is_empty()).then(|| parts.join(", "))
}

/// The zone part of a line wrapped in the date prefix and the states clause `variant` asks for.
fn wrapped(roster: &Roster, marks: &Marks, when: When, variant: Variant, zones: &str) -> String {
    let start = when.start();
    let prefix = if variant.date {
        format!(
            "{}, ",
            start.with_timezone(&roster.axis()).format("%a %-d %b")
        )
    } else {
        String::new()
    };
    let states = match variant.clause {
        Clause::States => clause(roster, marks, when).map(|states| format!(", {states}")),
        Clause::Zones | Clause::Drop => None,
    };
    format!("{prefix}{zones}{}", states.unwrap_or_default())
}

fn axis_date(roster: &Roster, when: When) -> NaiveDate {
    when.start().with_timezone(&roster.axis()).date_naive()
}

/// The say line for the marks over `when`, shaped by `variant`.
pub fn say_line(roster: &Roster, marks: &Marks, when: When, variant: Variant) -> String {
    let sources = zone_sources(roster, marks, when, variant.clause);
    let zones = entries(sources, when, axis_date(roster, when), variant.names).join(" / ");
    wrapped(roster, marks, when, variant, &zones)
}

/// The focused row's entry, then the axis (and any place zones) in parentheses unless they
/// render the same time; with names on, the rest follows after ` / ` so parentheses never nest.
fn focused_first(roster: &Roster, marks: &Marks, row: Row, when: When, variant: Variant) -> String {
    let focused = match row {
        Row::Person(id) => {
            let person = roster.person(id);
            (person.location.tz, vec![person.name.clone()])
        }
        Row::Place(id) => {
            let place = roster.place(id);
            (place.location.tz, vec![place.name.clone()])
        }
    };
    let sources = [focused, axis_source(roster, marks)]
        .into_iter()
        .chain(place_sources(roster, marks, variant.clause))
        .collect();
    let zones = match entries(sources, when, axis_date(roster, when), variant.names).as_slice() {
        [] => String::new(),
        [only] => only.clone(),
        [first, rest @ ..] if variant.names => format!("{first} / {}", rest.join(" / ")),
        [first, rest @ ..] => format!("{first} ({})", rest.join(" / ")),
    };
    wrapped(roster, marks, when, variant, &zones)
}

/// The line `format` copies for the marks, with `focus` as the focused row.
pub fn format_line(
    roster: &Roster,
    marks: &Marks,
    focus: Row,
    when: When,
    format: &Format,
) -> String {
    match format.who {
        Who::Marked => say_line(roster, marks, when, format.variant),
        Who::All => say_line(roster, &Marks::all(roster), when, format.variant),
        Who::FocusedFirst => focused_first(roster, marks, focus, when, format.variant),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::time::Moment;

    const FIXTURE: &str = include_str!("../tests/fixtures/sample.toml");
    const VENUE: &str = "name  = \"venue\"\nkind  = \"trading\"\n";

    /// The fixture with the venue profile labelled, and US moved to an unlabelled copy of it.
    fn labelled() -> Roster {
        let text = FIXTURE
            .replacen(
                VENUE,
                &format!("{VENUE}labels = {{ open = \"trading hours\", busy = \"rush\" }}\n"),
                1,
            )
            .replacen(
                "tz      = \"America/Chicago\"\ntrading = \"venue\"",
                "tz      = \"America/Chicago\"\ntrading = \"desk\"",
                1,
            )
            + "\n[[profile]]\nname = \"desk\"\nkind = \"trading\"\nopen = \"7am-11pm\"\n";
        Config::parse(&text).unwrap().roster
    }

    /// The say line `teamzones say <words>` prints at the golden instant.
    fn line(roster: &Roster, words: &str) -> String {
        let tokens: Vec<&str> = words.split(' ').collect();
        let now = "2026-10-23T22:04:00Z".parse().unwrap();
        let expr = crate::expr::parse_expr(&tokens, roster, now).unwrap();
        let Moment::Fixed(when) = expr.moment else {
            panic!("{words} has no fixed time");
        };
        say_line(roster, &expr.marks, when, Variant::default())
    }

    /// The say line for `words` on Mon 26 Oct 6am GMT, travelling in London until 1 Nov.
    fn travelling_line(words: &str, variant: Variant) -> String {
        let text = format!("{FIXTURE}\n[travel]\nhere = \"London\"\nuntil = 2026-11-01\n");
        let config = Config::parse(&text).unwrap();
        let now = "2026-10-26T06:00:00Z".parse().unwrap();
        let roster = config.roster_at(now);
        let tokens: Vec<&str> = words.split(' ').collect();
        let expr = crate::expr::parse_expr(&tokens, roster, now).unwrap();
        let Moment::Fixed(when) = expr.moment else {
            panic!("{words} has no fixed time");
        };
        say_line(roster, &expr.marks, when, variant)
    }

    #[test]
    fn while_travelling_home_s_own_zone_is_copied_in_offset_order_after_the_axis() {
        assert_eq!(
            travelling_line("team 3pm", Variant::default()),
            "3pm GMT / Tue 2am AEDT / Tue 1am AEST / 5pm SAST"
        );
    }

    #[test]
    fn while_travelling_names_go_to_whoever_is_in_each_zone_and_the_reference_has_none() {
        let names = Variant {
            names: true,
            ..Variant::default()
        };
        assert_eq!(
            travelling_line("team 3pm", names),
            "3pm GMT (Paul) / Tue 2am AEDT / Tue 1am AEST (Priya, Sam) / 5pm SAST (Thandi, Johan)"
        );
    }

    #[test]
    fn while_travelling_a_places_only_line_names_the_axis_alone() {
        assert_eq!(
            travelling_line("regions 3pm", Variant::default()),
            "3pm GMT, open in UK and US, closed in AU"
        );
    }

    #[test]
    fn a_counting_place_is_named_by_its_real_state_not_the_question() {
        let roster = Config::parse(FIXTURE).unwrap().roster;
        assert_eq!(
            line(&roster, "apps 3pm"),
            "3pm AEDT, open in AU, closed in UK and US"
        );
        assert_eq!(
            line(&roster, "regions 2-4pm mon"),
            "2-4pm AEDT, open in AU and US, closed in UK"
        );
    }

    #[test]
    fn the_focus_line_for_a_row_in_the_axis_zone_names_the_time_once() {
        let roster = Config::parse(FIXTURE).unwrap().roster;
        let now: DateTime<Utc> = "2026-10-23T22:04:00Z".parse().unwrap();
        let formats = Formats::builtin();
        let t = formats.get('t').unwrap();
        let marks = Marks::of_group(&roster.groups[0]);
        let home = Row::Person(crate::config::PersonId::HOME);
        let au = Row::Place(roster.place_named("AU").unwrap());
        for row in [home, au] {
            assert_eq!(
                format_line(&roster, &marks, row, When::At(now), t),
                "9:04am AEDT"
            );
        }
    }

    const COPY_SPEC: &str = include_str!("../../../docs/spec/copy.md");

    /// The `who` and `places` words of one spec row, read by serde as config reads them.
    #[derive(Deserialize)]
    struct Words {
        who: Who,
        places: Clause,
    }

    /// The spec's `## Formats` table as formats, plus the key its `default` column marks.
    fn spec_formats() -> (Vec<Format>, Vec<char>) {
        let yes = |cell: &str| match cell {
            "yes" => true,
            "no" => false,
            other => panic!("{other} is not yes or no"),
        };
        let rows: Vec<Vec<String>> = COPY_SPEC
            .lines()
            .skip_while(|line| *line != "## Formats")
            .skip_while(|line| !line.starts_with("| Key |"))
            .take_while(|line| line.starts_with('|'))
            .skip(2)
            .map(|line| {
                line.trim_matches('|')
                    .split(" | ")
                    .map(|cell| cell.trim().trim_matches('`').to_owned())
                    .collect()
            })
            .collect();
        let formats = rows
            .iter()
            .map(|cells| {
                let words: Words = toml::from_str(&format!(
                    "who = \"{}\"\nplaces = \"{}\"",
                    cells[2], cells[5]
                ))
                .unwrap();
                Format {
                    key: cells[0].chars().next().unwrap(),
                    label: cells[1].clone(),
                    who: words.who,
                    variant: Variant {
                        date: yes(&cells[3]),
                        names: yes(&cells[4]),
                        clause: words.places,
                    },
                }
            })
            .collect();
        let defaults = rows
            .iter()
            .filter(|cells| yes(&cells[6]))
            .map(|cells| cells[0].chars().next().unwrap())
            .collect();
        (formats, defaults)
    }

    #[test]
    fn the_built_in_formats_are_the_spec_formats_table_row_for_row() {
        let (formats, defaults) = spec_formats();
        let builtin = Formats::builtin();
        assert_eq!(builtin.all(), formats.as_slice());
        assert_eq!(defaults, [builtin.default_format().key]);
    }

    #[test]
    fn a_focused_first_format_composes_names_dates_and_states_like_any_other() {
        let roster = Config::parse(FIXTURE).unwrap().roster;
        let tokens = ["regions", "2-4pm", "mon"];
        let now = "2026-10-23T22:04:00Z".parse().unwrap();
        let expr = crate::expr::parse_expr(&tokens, &roster, now).unwrap();
        let Moment::Fixed(when) = expr.moment else {
            panic!("no fixed time");
        };
        let uk = Row::Place(roster.place_named("UK").unwrap());
        let format = Format {
            key: 'k',
            label: "mine".to_owned(),
            who: Who::FocusedFirst,
            variant: Variant {
                date: true,
                names: true,
                clause: Clause::States,
            },
        };
        assert_eq!(
            format_line(&roster, &expr.marks, uk, when, &format),
            "Mon 26 Oct, 3-5am GMT (UK) / 2-4pm AEDT, open in AU and US, closed in UK"
        );
    }

    #[test]
    fn a_focused_first_format_with_names_never_nests_parentheses() {
        let roster = Config::parse(FIXTURE).unwrap().roster;
        let tokens = ["team", "5-6pm", "mon"];
        let now = "2026-10-23T22:04:00Z".parse().unwrap();
        let expr = crate::expr::parse_expr(&tokens, &roster, now).unwrap();
        let Moment::Fixed(when) = expr.moment else {
            panic!("no fixed time");
        };
        let thandi = roster
            .people()
            .find(|(_, person)| person.name == "Thandi")
            .map(|(id, _)| Row::Person(id))
            .unwrap();
        let format = Format {
            key: 'k',
            label: "mine".to_owned(),
            who: Who::FocusedFirst,
            variant: Variant {
                date: false,
                names: true,
                clause: Clause::Drop,
            },
        };
        assert_eq!(
            format_line(&roster, &expr.marks, thandi, when, &format),
            "8-9am SAST (Thandi) / 5-6pm AEDT (Paul)"
        );
    }

    #[test]
    fn each_place_speaks_its_own_profile_s_labels() {
        let roster = labelled();
        assert_eq!(
            line(&roster, "regions -@us 5:30pm mon"),
            "5:30pm AEDT, closed in UK, rush in AU"
        );
        assert_eq!(
            line(&roster, "regions 2-4pm mon"),
            "2-4pm AEDT, trading hours in AU, open in US, closed in UK"
        );
    }
}
