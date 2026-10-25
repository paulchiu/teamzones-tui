//! The world map: land in Braille, the night side, the sun, and a marker for every row.

use chrono::{DateTime, Utc};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::symbols::Marker;
use ratatui::widgets::Widget;
use ratatui::widgets::canvas::{Canvas, Map, MapResolution};
use teamzones_core::clock::When;
use teamzones_core::config::PersonId;
use teamzones_core::question::{Row, place_state};
use teamzones_core::schedule::{Location, subsolar, sun_up};

use super::app::App;
use super::say::local_text;
use super::theme::Role;
use super::view::{Look, width_of};

/// The latitudes the map shows: Antarctica and the far Arctic are cropped.
const SOUTH: f64 = -58.0;
const NORTH: f64 = 80.0;

/// The sun's glyph; the map only draws under the full glyph set.
const SUN: &str = "☼";

/// The instant the night side and the sun show: the range's midpoint, or the cursor's instant.
pub fn instant(when: When) -> DateTime<Utc> {
    match when {
        When::At(instant) => instant,
        When::Over(range) => {
            let (start, end) = (range.start().start(), range.end().start());
            start + (end - start) / 2
        }
    }
}

/// The cell holding a point, placed exactly as the canvas places its Braille dots (two by four
/// to a cell); `None` off the map.
pub fn project(area: Rect, lat: f64, lon: f64) -> Option<(u16, u16)> {
    let on_map = (SOUTH..=NORTH).contains(&lat) && (-180.0..=180.0).contains(&lon);
    if !on_map || area.is_empty() {
        return None;
    }
    let dots_x = f64::from(area.width) * 2.0 - 1.0;
    let dots_y = f64::from(area.height) * 4.0 - 1.0;
    let dot_x = ((lon + 180.0) * dots_x / 360.0).round() as u16;
    let dot_y = ((NORTH - lat) * dots_y / (NORTH - SOUTH)).round() as u16;
    Some((area.x + dot_x / 2, area.y + dot_y / 4))
}

/// The latitude and longitude at the centre of cell (`x`, `y`), inverting [`project`].
pub fn unproject(area: Rect, x: u16, y: u16) -> (f64, f64) {
    let dot_x = f64::from(x.saturating_sub(area.x)) * 2.0 + 0.5;
    let dot_y = f64::from(y.saturating_sub(area.y)) * 4.0 + 1.5;
    let lon = dot_x * 360.0 / (f64::from(area.width) * 2.0 - 1.0) - 180.0;
    let lat = NORTH - dot_y * (NORTH - SOUTH) / (f64::from(area.height) * 4.0 - 1.0);
    (lat, lon)
}

/// One marker: where it sits, and its glyph and role.
struct Pin {
    location: Location,
    glyph: &'static str,
    role: Role,
}

/// One label: the place it names, and its text.
struct Tag {
    location: Location,
    text: String,
}

/// The counted people above the who block's line, home first.
fn counted_people(app: &App) -> Vec<PersonId> {
    app.who_rows()
        .above
        .iter()
        .filter_map(|row| match row {
            Row::Person(id) => Some(*id),
            Row::Place(_) => None,
        })
        .collect()
}

/// Every row's marker in drawing order: uncounted first, so a counted marker wins a shared
/// cell, and home last.
fn pins(app: &App, look: Look) -> Vec<Pin> {
    let glyphs = look.glyphs;
    let roster = app.roster();
    let (people, places) = (app.who_rows(), app.where_rows());
    let person = |row: &Row, counted: bool| match *row {
        Row::Person(id) => Some(Pin {
            location: roster.person(id).location,
            glyph: if id == PersonId::HOME {
                glyphs.home
            } else {
                glyphs.person
            },
            role: if counted { Role::Marker } else { Role::Dim },
        }),
        Row::Place(_) => None,
    };
    let place = |row: &Row, counted: bool| match *row {
        Row::Place(id) => Some(Pin {
            location: roster.place(id).location,
            glyph: if counted {
                glyphs.place
            } else {
                glyphs.place_other
            },
            role: if counted { Role::Marker } else { Role::Dim },
        }),
        Row::Person(_) => None,
    };
    let below = people.below.iter().filter_map(|row| person(row, false));
    let other_places = places.below.iter().filter_map(|row| place(row, false));
    let counted_places = places.above.iter().filter_map(|row| place(row, true));
    let counted = people
        .above
        .iter()
        .rev()
        .filter_map(|row| person(row, true));
    below
        .chain(other_places)
        .chain(counted_places)
        .chain(counted)
        .collect()
}

/// The labels in the order they claim room: home, then one per city of counted people (`·N`
/// when N of them share it), then each counted place with its state over the range.
fn tags(app: &App) -> Vec<Tag> {
    let roster = app.roster();
    let when = app.when();
    let people = counted_people(app);
    let city_of = |id: &PersonId| {
        let person = roster.person(*id);
        (person.city.as_str(), person.location.tz)
    };
    let people_tags = people.iter().enumerate().filter_map(|(index, id)| {
        let city = city_of(id);
        let first = people.iter().position(|other| city_of(other) == city) == Some(index);
        first.then(|| {
            let person = roster.person(*id);
            let span = local_text(when, person.location.tz);
            let text = match people.iter().filter(|other| city_of(other) == city).count() {
                0 | 1 => format!("{} {span}", person.city),
                n => format!("{} {span} ·{n}", person.city),
            };
            Tag {
                location: person.location,
                text,
            }
        })
    });
    let place_tags = app.where_rows().above.into_iter().filter_map(|row| {
        let Row::Place(id) = row else { return None };
        let place = roster.place(id);
        let span = local_text(when, place.location.tz);
        let text = match place_state(roster, id, when) {
            Some(level) => format!("{} {} {span}", place.name, place.labels().word(level)),
            None => format!("{} {span}", place.name),
        };
        Some(Tag {
            location: place.location,
            text,
        })
    });
    people_tags.chain(place_tags).collect()
}

/// Where a label of `width` goes for a marker at (`x`, `y`): right of it on its row, then right
/// of it on the row above, then left of it; `None` when all three overlap a placed label or leave
/// the map.
fn place_label(area: Rect, (x, y): (u16, u16), width: u16, placed: &[Rect]) -> Option<Rect> {
    let candidates = [
        (x.checked_add(2), Some(y)),
        (x.checked_add(2), y.checked_sub(1)),
        (x.checked_sub(width + 1), Some(y)),
    ];
    candidates
        .into_iter()
        .filter_map(|(x, y)| Some(Rect::new(x?, y?, width, 1)))
        .find(|rect| {
            let inside = area.contains(rect.as_position()) && rect.right() <= area.right();
            let gap = |other: &Rect| Rect {
                x: other.x.saturating_sub(1),
                width: other.width + 2,
                ..*other
            };
            inside && !placed.iter().any(|other| gap(other).intersects(*rect))
        })
}

/// Draws the map into `area` for the app's range or cursor.
pub fn draw(buf: &mut Buffer, area: Rect, app: &App, look: Look) {
    if area.is_empty() {
        return;
    }
    let theme = look.theme;
    let land = theme.color(Role::Text);
    Canvas::default()
        .marker(Marker::Braille)
        .x_bounds([-180.0, 180.0])
        .y_bounds([SOUTH, NORTH])
        .paint(|ctx| {
            ctx.draw(&Map {
                resolution: MapResolution::High,
                color: land,
            });
        })
        .render(area, buf);
    let at = instant(app.when());
    let tint =
        Some(theme.color(Role::NightTint)).filter(|tint| !theme.is_mono() && *tint != Color::Reset);
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            let (lat, lon) = unproject(area, x, y);
            if sun_up(lat, lon, at) {
                continue;
            }
            let cell = &mut buf[(x, y)];
            if cell.symbol() != " " {
                cell.set_fg(theme.color(Role::Dim));
            }
            if let Some(tint) = tint {
                cell.set_bg(tint);
            }
        }
    }
    let (sun_lon, declination) = subsolar(at);
    if let Some((x, y)) = project(area, declination, sun_lon) {
        buf[(x, y)].set_symbol(SUN).set_fg(theme.color(Role::Day));
    }
    for pin in pins(app, look) {
        if let Some(cell) = project(area, pin.location.lat, pin.location.lon) {
            buf[cell]
                .set_symbol(pin.glyph)
                .set_fg(theme.color(pin.role));
        }
    }
    let style = Style::new()
        .fg(theme.color(Role::Text))
        .add_modifier(Modifier::BOLD);
    let mut placed: Vec<Rect> = Vec::new();
    for tag in tags(app) {
        let Some(cell) = project(area, tag.location.lat, tag.location.lon) else {
            continue;
        };
        let width = u16::try_from(width_of(&tag.text)).unwrap_or(u16::MAX);
        if let Some(rect) = place_label(area, cell, width, &placed) {
            buf.set_stringn(rect.x, rect.y, &tag.text, usize::from(rect.width), style);
            placed.push(rect);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const AREA: Rect = Rect::new(0, 1, 120, 12);

    #[test]
    fn the_map_s_corners_project_onto_its_corner_cells() {
        assert_eq!(project(AREA, NORTH, -180.0), Some((0, 1)));
        assert_eq!(project(AREA, SOUTH, 180.0), Some((119, 12)));
        assert_eq!(project(AREA, -60.0, 0.0), None);
    }

    #[test]
    fn unproject_inverts_project_for_every_cell() {
        for y in AREA.top()..AREA.bottom() {
            for x in AREA.left()..AREA.right() {
                let (lat, lon) = unproject(AREA, x, y);
                assert_eq!(project(AREA, lat, lon), Some((x, y)), "({x}, {y})");
            }
        }
    }
}
