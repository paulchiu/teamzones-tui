//! Editing config from the TUI: each edit rewrites the config text through `write`, is parsed
//! back, and is adopted with the lens, marks and focus carried across by name.

use teamzones_core::config::{Config, PersonId, Roster};
use teamzones_core::question::{Marks, Row};
use toml_edit::DocumentMut;

use super::app::{App, Lens, base_marks};
use super::editor::Loaded;
use super::notice::{Notice, TIMEOUT};
use super::undo::{Snapshot, Undo};
use super::update::Effect;
use crate::write::{self, Members};

/// The config text after `edit`, and the config it parses to, or why it was refused.
///
/// # Errors
///
/// The reason, as the notice row says it: no file to edit, or text that would not load.
pub fn reparse(app: &App, edit: impl FnOnce(&mut DocumentMut)) -> Result<(String, Config), String> {
    let text = app
        .text
        .as_deref()
        .ok_or_else(|| "no config file: E creates one".to_owned())?;
    let text = write::edited(text, edit).map_err(|error| error.to_string())?;
    let home = app.config.roster.person(PersonId::HOME).location.tz;
    let config = Config::load(Some(&text), home).map_err(|error| error.to_string())?;
    Ok((text, config))
}

/// A row's kind and name, which find it again in a re-parsed roster.
fn row_name(roster: &Roster, row: Row) -> (bool, String) {
    match row {
        Row::Person(id) => (false, roster.person(id).name.clone()),
        Row::Place(id) => (true, roster.place(id).name.clone()),
    }
}

/// The row with this kind and name in `roster`; home is always home.
fn find_row(roster: &Roster, row: Row, (place, name): &(bool, String)) -> Option<Row> {
    match (row, place) {
        (Row::Person(id), _) if id == PersonId::HOME => Some(row),
        (_, true) => roster.place_named(name).map(Row::Place),
        (_, false) => roster.person_named(name).map(Row::Person),
    }
}

/// `marks` from `old` re-resolved in `new` by name, dropping any row that is gone.
fn carried_marks(old: &Roster, marks: &Marks, new: &Roster) -> Marks {
    let carried = |row: Row| find_row(new, row, &row_name(old, row));
    Marks {
        people: marks
            .people
            .iter()
            .filter_map(|id| match carried(Row::Person(*id)) {
                Some(Row::Person(id)) => Some(id),
                Some(Row::Place(_)) | None => None,
            })
            .collect(),
        places: marks
            .places
            .iter()
            .filter_map(|id| match carried(Row::Place(*id)) {
                Some(Row::Place(id)) => Some(id),
                Some(Row::Person(_)) | None => None,
            })
            .collect(),
        clocks: marks.clocks.clone(),
        rule: marks.rule,
    }
}

/// The group in `new` that the lens showed, found by its name or, after a rename, its alias.
fn carried_lens(app: &App, new: &Roster) -> Option<Lens> {
    match app.lens {
        Lens::All => Some(Lens::All),
        Lens::Group(index) => {
            let name = &app.roster().groups.get(index)?.name;
            new.groups
                .iter()
                .position(|group| {
                    group.name.eq_ignore_ascii_case(name)
                        || group.aliases.iter().any(|a| a.eq_ignore_ascii_case(name))
                })
                .map(Lens::Group)
        }
    }
}

/// The first group, or `0 all` without one.
pub fn first_lens(roster: &Roster) -> Lens {
    if roster.groups.is_empty() {
        Lens::All
    } else {
        Lens::Group(0)
    }
}

/// The app on `config`, parsed from `text`. The lens stays on its group by name, the marks follow
/// the group's new members unless they had drifted, and focus stays on its row by name.
pub fn adopt(app: App, config: Config, text: Option<String>) -> App {
    let new = &config.roster;
    let carried = carried_lens(&app, new);
    let lens = carried.unwrap_or_else(|| first_lens(new));
    let marks = match carried {
        Some(_) if !app.drift().is_empty() => carried_marks(app.roster(), &app.marks, new),
        Some(_) | None => base_marks(new, lens),
    };
    let focus = find_row(new, app.focus, &row_name(app.roster(), app.focus)).unwrap_or(app.focus);
    App {
        lens,
        marks,
        focus,
        config,
        text,
        ..app
    }
}

/// Writes the config text when the file does not hold it yet, ending any pending close.
pub fn flush(app: App, key: &str) -> (App, Vec<Effect>) {
    let write = match (&app.text, &app.disk) {
        (Some(text), Some(disk)) if text != disk => Some(Effect::Write {
            key: key.to_owned(),
            text: text.clone(),
            over: disk.clone(),
        }),
        (Some(_) | None, Some(_) | None) => None,
    };
    let disk = if write.is_some() {
        app.text.clone()
    } else {
        app.disk.clone()
    };
    let flushed = App {
        disk,
        flush_at: None,
        ..app
    };
    (flushed, write.into_iter().collect())
}

/// Puts `disk` back in the file while keeping any pending close pending, for an undo that returns
/// to a state whose close had not been written yet.
pub fn write_back(app: App, disk: Option<String>, key: &str) -> (App, Vec<Effect>) {
    let write = match (&disk, &app.disk) {
        (Some(text), Some(over)) if text != over => Some(Effect::Write {
            key: key.to_owned(),
            text: text.clone(),
            over: over.clone(),
        }),
        (Some(_) | None, Some(_) | None) => None,
    };
    let disk = if write.is_some() {
        disk
    } else {
        app.disk.clone()
    };
    (App { disk, ..app }, write.into_iter().collect())
}

/// Offers `before` to `u` for 10 s and raises the notice `key  text`.
pub fn undoable(app: App, before: Snapshot, key: &str, text: &str) -> App {
    let now = app.now;
    App {
        undo: Some(Undo {
            before: Box::new(before),
            expires: now + TIMEOUT,
        }),
        notice: Some(Notice::new(key, text, now)),
        ..app
    }
}

/// The marks as a group lists them, in config order, with `where` only when it changed.
fn members(app: &App, saved: Option<&teamzones_core::config::Group>) -> Members {
    let roster = app.roster();
    let people = roster
        .people()
        .filter(|(id, _)| *id != PersonId::HOME && app.marks.people.contains(id))
        .map(|(_, person)| person.name.clone())
        .collect();
    let places = roster
        .places()
        .filter(|(id, _)| app.marks.places.contains(id))
        .map(|(_, place)| place.name.clone())
        .collect();
    let rule = saved.map_or_else(Default::default, |group| group.rule);
    Members {
        people,
        places,
        rule: (app.marks.rule != rule).then_some(app.marks.rule),
    }
}

fn refused(app: App, key: &str, error: &str) -> (App, Vec<Effect>) {
    let now = app.now;
    let text = format!("not saved: {error}");
    (
        App {
            notice: Some(Notice::new(key, &text, now)),
            ..app
        },
        Vec::new(),
    )
}

/// `w`: writes the marks into the active group.
pub fn save(app: App) -> (App, Vec<Effect>) {
    let Lens::Group(index) = app.lens else {
        return (app, Vec::new());
    };
    let members = members(&app, app.roster().groups.get(index));
    let tags = app.drift();
    if tags.is_empty() {
        return (app, Vec::new());
    }
    match reparse(&app, |document| {
        write::save_group(document, index, &members);
    }) {
        Ok((text, config)) => {
            let before = Snapshot::of(&app);
            let notice = format!("{}: {} saved  u undo", app.lens_name(), tags.join(" "));
            let next = adopt(app, config, Some(text));
            flush(undoable(next, before, "w", &notice), "w")
        }
        Err(error) => refused(app, "w", &error),
    }
}

/// Adopts a config read afresh, which the file now holds, or keeps the last good one and says
/// why. Undo is dropped, since what it would put back predates the file it would overwrite.
pub fn reloaded(app: App, key: &str, result: Result<Box<Loaded>, String>) -> App {
    let now = app.now;
    match result {
        Ok(loaded) => {
            let Loaded { config, text } = *loaded;
            let next = adopt(app, config, text.clone());
            App {
                first_run: text.is_none(),
                disk: text,
                flush_at: None,
                undo: None,
                notice: Some(Notice::new(key, "config reloaded", now)),
                ..next
            }
        }
        Err(error) => App {
            notice: Some(Notice::new(key, &error, now)),
            ..app
        },
    }
}

/// `x`: closes the active group at once, writing the file only once the undo window has passed
/// (or on quit), so `u` before then never touches the file.
pub fn close(app: App) -> (App, Vec<Effect>) {
    let Lens::Group(index) = app.lens else {
        return refused(app, "x", "0 all cannot be closed");
    };
    match reparse(&app, |document| write::remove_group(document, index)) {
        Ok((text, config)) => {
            let before = Snapshot::of(&app);
            let notice = format!(
                "closed {}: its people and places stay  u undo",
                app.lens_name()
            );
            let flush_at = Some(app.now + TIMEOUT);
            let next = App {
                flush_at,
                ..adopt(app, config, Some(text))
            };
            (undoable(next, before, "x", &notice), Vec::new())
        }
        Err(error) => refused(app, "x", &error),
    }
}

/// The name `+` offers for the marks: the expression lowercased, without `+` or `@`, its parts
/// joined by `_`, so `team +@UK` offers `team_uk`.
pub fn offered_name(app: &App) -> String {
    app.expression()
        .to_lowercase()
        .replace(['+', '@'], "")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("_")
}

/// `+`, once its name is typed: saves the marks as a new group named `name` and shows it.
pub fn save_as(app: App, name: &str) -> (App, Vec<Effect>) {
    let members = members(&app, None);
    match reparse(&app, |document| write::push_group(document, name, &members)) {
        Ok((text, config)) => {
            let before = Snapshot::of(&app);
            let index = config.roster.groups.len().saturating_sub(1);
            let next = adopt(app, config, Some(text));
            let next = App {
                lens: Lens::Group(index),
                marks: base_marks(&next.config.roster, Lens::Group(index)),
                ..next
            };
            let notice = format!("saved {name}  u undo");
            flush(undoable(next, before, "+", &notice), "+")
        }
        Err(error) => refused(app, "+", &error),
    }
}

/// `=`: the next `where` for the marks, skipping `context` while no person is marked.
pub fn cycle_where(app: App) -> App {
    use teamzones_core::config::Rule;
    let people = app.marks.people.iter().any(|id| *id != PersonId::HOME);
    let rule = match app.marks.rule {
        Rule::Open => Rule::Closed,
        Rule::Closed if people => Rule::Context,
        Rule::Closed | Rule::Context => Rule::Open,
    };
    App {
        marks: Marks { rule, ..app.marks },
        ..app
    }
}
