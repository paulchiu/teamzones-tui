//! Who-expressions: a base group plus `+`/`-` modifiers, followed by typed time.

use chrono::{DateTime, Utc};
use thiserror::Error;

use crate::cities::{self, CityError};
use crate::config::{Group, Roster};
use crate::question::{CityClock, Marks};
use crate::time::{Moment, TimeError, parse_time_tokens};

/// A parsed expression: its text, the marks it sets and the time it names.
#[derive(Clone, Debug, PartialEq)]
pub struct Expr {
    pub text: String,
    pub marks: Marks,
    pub moment: Moment,
}

/// Why an expression did not parse; the Display is the exact text the CLI prints.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum ExprError {
    #[error("there are no groups in config")]
    NoGroups,
    #[error("{0} is a place: did you mean @{0}?")]
    IsPlace(String),
    #[error("{0} is a person: did you mean +{0}?")]
    IsPerson(String),
    #[error("{0} is a group, so it goes before the time")]
    GroupAfterTime(String),
    #[error("{0}: an expression starts from one group, so did you mean +{0}?")]
    SecondGroup(String),
    #[error("{token} is not a group or a time{}", hint(.suggestion.as_deref()))]
    Unknown {
        token: String,
        suggestion: Option<String>,
    },
    #[error("{0}: no person, group, place or city has that name")]
    UnknownName(String),
    #[error("{expr}: {detail}")]
    NoChange { expr: String, detail: String },
    #[error(transparent)]
    City(#[from] CityError),
    #[error(transparent)]
    Time(#[from] TimeError),
}

fn hint(suggestion: Option<&str>) -> String {
    suggestion
        .map(|s| format!(": did you mean {s}?"))
        .unwrap_or_default()
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Sign {
    Add,
    Remove,
}

/// The marks and text so far, as each modifier is applied.
struct Building<'a> {
    roster: &'a Roster,
    base: String,
    text: String,
    marks: Marks,
}

impl Building<'_> {
    fn no_change(&self, detail: String) -> ExprError {
        ExprError::NoChange {
            expr: self.text.clone(),
            detail,
        }
    }

    /// Adds or removes one row, or reports that it changed nothing.
    fn toggle<T: Ord>(set: &mut std::collections::BTreeSet<T>, sign: Sign, item: T) -> bool {
        match sign {
            Sign::Add => set.insert(item),
            Sign::Remove => set.remove(&item),
        }
    }

    fn apply(&mut self, token: &str) -> Result<(), ExprError> {
        let (sign, name) = match token.split_at(1) {
            ("+", name) => (Sign::Add, name),
            (_, name) => (Sign::Remove, name),
        };
        self.text = format!("{} {token}", self.text);
        let roster = self.roster;
        let already = |what: &str| match sign {
            Sign::Add => format!("{what} is already in {}", self.base),
            Sign::Remove => format!("{what} is not in {}", self.base),
        };
        if let Some(place_name) = name.strip_prefix('@') {
            if let Some(place) = roster.place_named(place_name) {
                let detail = already(&format!("@{place_name}"));
                return if Self::toggle(&mut self.marks.places, sign, place) {
                    Ok(())
                } else {
                    Err(self.no_change(detail))
                };
            }
            let city = cities::lookup(place_name).map_err(|error| match error {
                CityError::Unknown { .. } => ExprError::UnknownName(token.to_owned()),
                ambiguous @ CityError::Ambiguous { .. } => ambiguous.into(),
            })?;
            let present = self
                .marks
                .clocks
                .iter()
                .position(|clock| clock.tz == city.tz && clock.name == city.name);
            return match (sign, present) {
                (Sign::Add, None) => {
                    self.marks.clocks.push(CityClock {
                        name: city.name.to_owned(),
                        tz: city.tz,
                    });
                    Ok(())
                }
                (Sign::Remove, Some(index)) => {
                    self.marks.clocks.remove(index);
                    Ok(())
                }
                (Sign::Add, Some(_)) | (Sign::Remove, None) => {
                    Err(self.no_change(already(&format!("@{place_name}"))))
                }
            };
        }
        if let Some(person) = roster.person_named(name) {
            let detail = already(name);
            return if Self::toggle(&mut self.marks.people, sign, person) {
                Ok(())
            } else {
                Err(self.no_change(detail))
            };
        }
        let group = roster
            .group_named(name)
            .ok_or_else(|| ExprError::UnknownName(token.to_owned()))?;
        self.apply_group(sign, name, group)
    }

    fn apply_group(&mut self, sign: Sign, name: &str, group: &Group) -> Result<(), ExprError> {
        let changed_people = group
            .people
            .iter()
            .filter(|id| Self::toggle(&mut self.marks.people, sign, **id))
            .count();
        let changed_places = group
            .places
            .iter()
            .filter(|id| Self::toggle(&mut self.marks.places, sign, **id))
            .count();
        if changed_people + changed_places > 0 {
            return Ok(());
        }
        let kinds = match (group.people.is_empty(), group.places.is_empty()) {
            (false, true) | (true, true) => "people",
            (true, false) => "places",
            (false, false) => "people or places",
        };
        let mark = match sign {
            Sign::Add => '+',
            Sign::Remove => '-',
        };
        let place_hint = self
            .roster
            .place_named(name)
            .map(|_| format!("; did you mean {mark}@{name}?"))
            .unwrap_or_default();
        let detail = match sign {
            Sign::Add => format!(
                "all of {name}'s {kinds} are already in {}{place_hint}",
                self.base
            ),
            Sign::Remove => format!("none of {name}'s {kinds} are in {}{place_hint}", self.base),
        };
        Err(self.no_change(detail))
    }
}

fn is_modifier(token: &str) -> bool {
    token.len() > 1 && token.starts_with(['+', '-'])
}

fn names_a_group(roster: &Roster, token: &str) -> bool {
    token.eq_ignore_ascii_case("all") || roster.group_named(token).is_some()
}

/// The Levenshtein distance between two short names, ignoring ASCII case.
fn distance(a: &str, b: &str) -> usize {
    let (a, b): (Vec<char>, Vec<char>) = (
        a.to_ascii_lowercase().chars().collect(),
        b.to_ascii_lowercase().chars().collect(),
    );
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.iter().enumerate() {
        let mut diagonal = row[0];
        row[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let substitution = diagonal + usize::from(ca != cb);
            diagonal = row[j + 1];
            row[j + 1] = substitution.min(row[j] + 1).min(row[j + 1] + 1);
        }
    }
    row[b.len()]
}

/// The group or alias nearest to `token`, if any is within two edits.
fn nearest_group(roster: &Roster, token: &str) -> Option<String> {
    roster
        .groups
        .iter()
        .flat_map(|group| std::iter::once(&group.name).chain(&group.aliases))
        .map(|name| (distance(name, token), name))
        .filter(|(edits, _)| *edits <= 2)
        .min_by_key(|(edits, _)| *edits)
        .map(|(_, name)| name.clone())
}

/// Explains a TIME token that is really a place, a person or a misspelt group.
fn explain(roster: &Roster, error: TimeError) -> ExprError {
    let TimeError::Unknown { token } = error else {
        return error.into();
    };
    if roster.place_named(&token).is_some() {
        ExprError::IsPlace(token)
    } else if roster.person_named(&token).is_some() {
        ExprError::IsPerson(token)
    } else if names_a_group(roster, &token) {
        ExprError::GroupAfterTime(token)
    } else {
        let suggestion = nearest_group(roster, &token);
        ExprError::Unknown { token, suggestion }
    }
}

/// Parses the tokens after `say` or `status`: EXPR while each token is a group, an alias or a
/// modifier, then TIME. With no base group, the first group is used.
///
/// # Errors
///
/// Any [`ExprError`]: no groups to default to, a second base group, an unknown or no-op
/// modifier, a place or person where a time belongs, or a [`TimeError`].
pub fn parse_expr(tokens: &[&str], roster: &Roster, now: DateTime<Utc>) -> Result<Expr, ExprError> {
    let explicit = tokens.first().filter(|token| names_a_group(roster, token));
    let (base, marks) = match explicit {
        Some(token) if token.eq_ignore_ascii_case("all") => {
            ((*token).to_owned(), Marks::all(roster))
        }
        Some(token) => (
            (*token).to_owned(),
            roster
                .group_named(token)
                .map(Marks::of_group)
                .unwrap_or_default(),
        ),
        None => {
            let first = roster.groups.first().ok_or(ExprError::NoGroups)?;
            (first.name.clone(), Marks::of_group(first))
        }
    };
    let mut building = Building {
        roster,
        text: base.clone(),
        base,
        marks,
    };
    let mut rest = &tokens[usize::from(explicit.is_some())..];
    while let Some(token) = rest.first() {
        if is_modifier(token) {
            building.apply(token)?;
        } else if names_a_group(roster, token) {
            return Err(ExprError::SecondGroup((*token).to_owned()));
        } else {
            break;
        }
        rest = &rest[1..];
    }
    let moment = parse_time_tokens(rest, roster, now).map_err(|error| explain(roster, error))?;
    Ok(Expr {
        text: building.text,
        marks: building.marks,
        moment,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    const FIXTURE: &str = include_str!("../tests/fixtures/sample.toml");

    fn parse(text: &str) -> Result<Expr, ExprError> {
        let roster = Config::parse(FIXTURE).unwrap().roster;
        let tokens: Vec<&str> = text.split_whitespace().collect();
        parse_expr(&tokens, &roster, "2026-10-23T22:04:00Z".parse().unwrap())
    }

    fn error(text: &str) -> String {
        parse(text).unwrap_err().to_string()
    }

    #[test]
    fn the_first_group_is_the_base_when_none_is_given() {
        let expr = parse("+ellie 3pm").unwrap();
        assert_eq!(expr.text, "team +ellie");
    }

    #[test]
    fn a_bare_person_name_asks_for_a_plus() {
        assert_eq!(error("team sam"), "sam is a person: did you mean +sam?");
    }

    #[test]
    fn plus_city_adds_a_clock_that_is_never_counted() {
        let expr = parse("au +@london 3pm").unwrap();
        assert_eq!(
            expr.marks.clocks,
            [CityClock {
                name: "London".into(),
                tz: chrono_tz::Europe::London
            }]
        );
        assert_eq!(expr.marks.counted_people().len(), 3);
    }

    #[test]
    fn a_modifier_that_changes_nothing_is_an_error() {
        assert_eq!(error("team +sam"), "team +sam: sam is already in team");
        assert_eq!(error("au -@uk"), "au -@uk: @uk is not in au");
        assert_eq!(
            error("team +au"),
            "team +au: all of au's people are already in team; did you mean +@au?"
        );
    }

    #[test]
    fn a_second_base_group_asks_for_a_plus() {
        assert_eq!(
            error("team au"),
            "au: an expression starts from one group, so did you mean +au?"
        );
    }

    #[test]
    fn an_unknown_word_suggests_the_nearest_group() {
        assert_eq!(
            error("tema"),
            "tema is not a group or a time: did you mean team?"
        );
        assert_eq!(
            error("+nobody"),
            "+nobody: no person, group, place or city has that name"
        );
    }
}
