//! The picker: `a` adds a person or a place, `e` edits the focused row, and `d` deletes it from
//! config and every group. Each field is a text prompt; `enter` moves to the next and saves on
//! the last, and `esc` cancels.

use ratatui::buffer::Buffer;
use ratatui::layout::{Margin, Rect};
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, Widget};
use teamzones_core::cities;
use teamzones_core::config::{PersonId, Roster};
use teamzones_core::question::Row;
use toml_edit::Value;

use super::app::{App, Mode};
use super::edit::{adopt, flush, reparse, undoable};
use super::notice::Notice;
use super::theme::Role;
use super::undo::Snapshot;
use super::update::Effect;
use super::view::{Look, put, truncate_with_mark};
use crate::write::{self, Entry, Field};

/// What the picker writes: a new person or place, or the row it edits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    NewPerson,
    NewPlace,
    Edit(Row),
}

/// The picker's fields, each a key and the text typed for it, the field being typed, and the
/// text each field started with, so an edit writes only what changed.
#[derive(Clone, Debug, PartialEq)]
pub struct Picker {
    pub target: Target,
    pub fields: Vec<(&'static str, String)>,
    pub at: usize,
    pub started: Vec<String>,
}

const PERSON: [&str; 3] = ["name", "city", "hours"];
const PLACE: [&str; 4] = ["name", "zone", "trading", "marker"];

fn is_place(target: Target) -> bool {
    match target {
        Target::NewPlace | Target::Edit(Row::Place(_)) => true,
        Target::NewPerson | Target::Edit(Row::Person(_)) => false,
    }
}

fn picker(target: Target, values: Vec<String>) -> Picker {
    let keys: &[&'static str] = if is_place(target) { &PLACE } else { &PERSON };
    Picker {
        target,
        fields: keys.iter().copied().zip(values.iter().cloned()).collect(),
        at: 0,
        started: values,
    }
}

/// The bundled city nearest `(lat, lon)`, by plain degrees, which is close enough to prefill.
fn nearest(lat: f64, lon: f64) -> Option<&'static cities::City> {
    let distance = |city: &cities::City| (city.lat - lat).powi(2) + (city.lon - lon).powi(2);
    cities::all()
        .iter()
        .min_by(|a, b| distance(a).total_cmp(&distance(b)))
}

/// Opens the picker on a new person or place, prefilled from a point on the map when given: the
/// nearest city for a person, or the point and that city's zone for a place.
pub fn open_new(app: App, place: bool, point: Option<(f64, f64)>) -> App {
    let near = point.and_then(|(lat, lon)| nearest(lat, lon));
    let values = if place {
        vec![
            String::new(),
            near.map_or_else(String::new, |city| city.tz.name().to_owned()),
            String::new(),
            point.map_or_else(String::new, |(lat, lon)| format!("{lat:.1}, {lon:.1}")),
        ]
    } else {
        vec![
            String::new(),
            near.map_or_else(String::new, |city| city.name.to_owned()),
            String::new(),
        ]
    };
    let target = if place {
        Target::NewPlace
    } else {
        Target::NewPerson
    };
    App {
        mode: Mode::Picker(Box::new(picker(target, values))),
        ..app
    }
}

/// The entry in config for `row`: its kind and index in config order.
fn entry(roster: &Roster, row: Row) -> Entry {
    match row {
        Row::Person(id) => Entry::Person(roster.people().position(|(i, _)| i == id).unwrap_or(0)),
        Row::Place(id) => Entry::Place(roster.places().position(|(i, _)| i == id).unwrap_or(0)),
    }
}

/// `e`: opens the picker on the focused row, prefilled with what its table says.
pub fn open_edit(app: App) -> App {
    let Some(text) = app.text.clone() else {
        return refused(app, "e", "no config file: E creates one");
    };
    let row = app.focus;
    let at = entry(app.roster(), row);
    let read = |key: &str| write::entry_value(&text, at, key).unwrap_or_default();
    let values = match row {
        Row::Person(_) => {
            let city = write::entry_value(&text, at, "city").unwrap_or_else(|| read("label"));
            vec![read("name"), city, read("hours")]
        }
        Row::Place(_) => vec![read("name"), read("tz"), read("trading"), read("at")],
    };
    App {
        mode: Mode::Picker(Box::new(picker(Target::Edit(row), values))),
        ..app
    }
}

/// The picker the app shows, if it shows one.
pub fn of(app: &App) -> Option<&Picker> {
    match &app.mode {
        Mode::Picker(picker) => Some(picker),
        Mode::Normal
        | Mode::Prompt(_)
        | Mode::NewGroup(_)
        | Mode::AddMenu(_)
        | Mode::CopyMenu
        | Mode::SortMenu
        | Mode::Help(..)
        | Mode::Matrix(_)
        | Mode::DriftMenu => None,
    }
}

/// The active field's text after `edit`.
pub fn edit_field(app: App, edit: impl FnOnce(&str) -> String) -> App {
    let Some(mut picker) = of(&app).cloned() else {
        return app;
    };
    if let Some((_, value)) = picker.fields.get_mut(picker.at) {
        *value = edit(value);
    }
    App {
        mode: Mode::Picker(Box::new(picker)),
        ..app
    }
}

fn refused(app: App, key: &str, error: &str) -> App {
    let now = app.now;
    App {
        notice: Some(Notice::new(key, &format!("not saved: {error}"), now)),
        ..app
    }
}

/// `"lat, lon"` as a TOML pair of floats.
fn marker(text: &str) -> Result<Value, String> {
    let numbers: Vec<f64> = text
        .split(',')
        .map(|part| part.trim().parse::<f64>())
        .collect::<Result<_, _>>()
        .map_err(|_| {
            format!("marker \"{text}\" is not a latitude and longitude such as 54.5, -2.5")
        })?;
    match numbers.as_slice() {
        [lat, lon] => Ok(Value::Array([*lat, *lon].into_iter().collect())),
        _ => Err(format!(
            "marker \"{text}\" is not a latitude and longitude such as 54.5, -2.5"
        )),
    }
}

/// A place's marker: the one typed, or the first bundled city in its zone.
fn marker_or_zone(text: &str, zone: &str) -> Result<Value, String> {
    if !text.trim().is_empty() {
        return marker(text);
    }
    cities::all()
        .iter()
        .find(|city| city.tz.name() == zone)
        .map(|city| Value::Array([city.lat, city.lon].into_iter().collect()))
        .ok_or_else(|| "a place needs a marker such as 54.5, -2.5".to_owned())
}

fn optional(text: &str) -> Option<Value> {
    let text = text.trim();
    (!text.is_empty()).then(|| Value::from(text))
}

/// The fields to write: every one for a new row, only the changed ones for an edit.
fn fields(picker: &Picker) -> Result<Vec<Field>, String> {
    let value = |key: &str| {
        picker
            .fields
            .iter()
            .find(|(field, _)| *field == key)
            .map_or("", |(_, value)| value.as_str())
    };
    let changed = |key: &str| {
        let index = picker.fields.iter().position(|(field, _)| *field == key);
        let new = picker.target == Target::NewPerson || picker.target == Target::NewPlace;
        new || index
            .is_some_and(|index| picker.started.get(index).map(String::as_str) != Some(value(key)))
    };
    let mut out: Vec<Field> = Vec::new();
    if changed("name") {
        out.push(("name", Some(Value::from(value("name").trim()))));
    }
    if is_place(picker.target) {
        if changed("zone") {
            out.push(("tz", Some(Value::from(value("zone").trim()))));
        }
        if changed("trading") {
            out.push(("trading", optional(value("trading"))));
        }
        if changed("marker") || changed("zone") && value("marker").trim().is_empty() {
            out.push((
                "at",
                Some(marker_or_zone(value("marker"), value("zone").trim())?),
            ));
        }
    } else {
        if changed("city") {
            out.push(("city", optional(value("city"))));
            if picker.target != Target::NewPerson {
                out.extend([("label", None), ("tz", None), ("at", None)]);
            }
        }
        if changed("hours") {
            out.push(("hours", optional(value("hours"))));
        }
    }
    Ok(out)
}

/// The key the picker's notices go under: `a` for a new row, `e` for an edit.
fn key_of(target: Target) -> &'static str {
    match target {
        Target::NewPerson | Target::NewPlace => "a",
        Target::Edit(_) => "e",
    }
}

/// `enter`: moves to the next field, or saves on the last, staying open on a refusal.
pub fn submit(app: App) -> (App, Vec<Effect>) {
    let Some(picker) = of(&app).cloned() else {
        return (app, Vec::new());
    };
    if picker.at + 1 < picker.fields.len() {
        let next = Picker {
            at: picker.at + 1,
            ..picker
        };
        return (
            App {
                mode: Mode::Picker(Box::new(next)),
                ..app
            },
            Vec::new(),
        );
    }
    let key = key_of(picker.target);
    let name = picker
        .fields
        .first()
        .map(|(_, name)| name.trim().to_owned())
        .unwrap_or_default();
    let written = fields(&picker).and_then(|fields| {
        let roster = app.roster();
        let target = match picker.target {
            Target::Edit(row) => Some((entry(roster, row), row_name(roster, row))),
            Target::NewPerson | Target::NewPlace => None,
        };
        reparse(&app, |document| match target {
            Some((entry, old)) => write::edit_entry(document, entry, &old, &fields),
            None => write::push_entry(document, is_place(picker.target), &fields),
        })
    });
    match written {
        Ok((text, config)) => {
            let before = Snapshot::of(&App {
                mode: Mode::Normal,
                ..app.clone()
            });
            let placed = is_place(picker.target);
            let next = adopt(
                App {
                    mode: Mode::Normal,
                    ..app
                },
                config,
                Some(text),
            );
            let focus = if placed {
                next.roster().place_named(&name).map(Row::Place)
            } else {
                next.roster().person_named(&name).map(Row::Person)
            };
            let next = App {
                focus: focus.unwrap_or(next.focus),
                ..next
            };
            let verb = match picker.target {
                Target::NewPerson | Target::NewPlace => "added",
                Target::Edit(_) => "saved",
            };
            flush(
                undoable(next, before, key, &format!("{verb} {name}  u undo")),
                key,
            )
        }
        Err(error) => (refused(app, key, &error), Vec::new()),
    }
}

fn row_name(roster: &Roster, row: Row) -> String {
    match row {
        Row::Person(id) => roster.person(id).name.clone(),
        Row::Place(id) => roster.place(id).name.clone(),
    }
}

/// How many groups list `row`.
fn listed_in(roster: &Roster, row: Row) -> usize {
    roster
        .groups
        .iter()
        .filter(|group| match row {
            Row::Person(id) => group.people.contains(&id),
            Row::Place(id) => group.places.contains(&id),
        })
        .count()
}

/// `d`: deletes the focused row from config and every group that lists it, with undo.
pub fn delete(app: App) -> (App, Vec<Effect>) {
    if app.focus == Row::Person(PersonId::HOME) {
        return (refused(app, "d", "home cannot be deleted"), Vec::new());
    }
    let roster = app.roster();
    let name = row_name(roster, app.focus);
    let groups = listed_in(roster, app.focus);
    let at = entry(roster, app.focus);
    match reparse(&app, |document| write::remove_entry(document, at, &name)) {
        Ok((text, config)) => {
            let before = Snapshot::of(&app);
            let from = match groups {
                0 => String::new(),
                1 => " and 1 group".to_owned(),
                n => format!(" and {n} groups"),
            };
            let notice = format!("deleted {name} from config{from}  u undo");
            let next = adopt(app, config, Some(text));
            flush(undoable(next, before, "d", &notice), "d")
        }
        Err(error) => (refused(app, "d", &error), Vec::new()),
    }
}

/// The picker's box: centred, one line per field inside its border.
pub fn area(size: Rect, picker: &Picker) -> Rect {
    let width = size.width.min(64);
    let height = size
        .height
        .min(u16::try_from(picker.fields.len() + 2).unwrap_or(u16::MAX));
    Rect::new(
        size.x + (size.width - width) / 2,
        size.y + (size.height - height) / 2,
        width,
        height,
    )
}

fn title(app: &App, target: Target) -> String {
    match target {
        Target::NewPerson => " new person ".to_owned(),
        Target::NewPlace => " new place ".to_owned(),
        Target::Edit(row) => format!(" edit {} ", row_name(app.roster(), row)),
    }
}

/// Draws the picker over the screen, the field being typed in accent with its cursor.
pub fn draw(buf: &mut Buffer, size: Rect, app: &App, picker: &Picker, look: Look) {
    let (theme, glyphs) = (look.theme, look.glyphs);
    let area = area(size, picker);
    Clear.render(area, buf);
    let save = if picker.at + 1 < picker.fields.len() {
        " next "
    } else {
        " save "
    };
    Block::bordered()
        .border_set(glyphs.rounded)
        .border_style(theme.fg(Role::Accent))
        .title(Span::styled(
            title(app, picker.target),
            theme.fg(Role::Text).add_modifier(Modifier::BOLD),
        ))
        .title_bottom(
            Line::from(vec![
                Span::styled(" enter", theme.fg(Role::Accent)),
                Span::styled(save, theme.fg(Role::Dim)),
                Span::styled(" esc", theme.fg(Role::Accent)),
                Span::styled(" cancel ", theme.fg(Role::Dim)),
            ])
            .right_aligned(),
        )
        .render(area, buf);
    let inner = area.inner(Margin::new(1, 1));
    let room = usize::from(inner.width.saturating_sub(11));
    for ((index, (key, value)), y) in picker.fields.iter().enumerate().zip(inner.top()..) {
        let active = index == picker.at;
        let label_style = if active {
            theme.fg(Role::Accent)
        } else {
            theme.fg(Role::Dim)
        };
        put(buf, inner, inner.x + 1, y, key, label_style);
        let shown = if active {
            format!("{value}_")
        } else {
            value.clone()
        };
        let shown = truncate_with_mark(&glyphs.text(&shown), room, &glyphs.text("…"));
        put(buf, inner, inner.x + 10, y, &shown, theme.fg(Role::Text));
    }
}
