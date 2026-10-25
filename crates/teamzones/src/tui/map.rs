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

/// What the map is framed on: the point at its centre and how many steps it is zoomed in.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
    pub lat: f64,
    pub lon: f64,
    pub zoom: i8,
}

/// The default camera: the whole world's width, on the latitude midway between the
/// northernmost and southernmost of home and the counted markers.
pub fn framing(app: &App) -> Camera {
    let roster = app.roster();
    let lats: Vec<f64> = app
        .who_rows()
        .above
        .into_iter()
        .chain(app.where_rows().above)
        .map(|row| match row {
            Row::Person(id) => roster.person(id).location.lat,
            Row::Place(id) => roster.place(id).location.lat,
        })
        .collect();
    let north = lats.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let south = lats.iter().copied().fold(f64::INFINITY, f64::min);
    Camera {
        lat: if lats.is_empty() {
            0.0
        } else {
            (north + south) / 2.0
        },
        lon: 0.0,
        zoom: 0,
    }
}

/// The furthest map mode zooms in: four times the area's width.
const MAX_ZOOM: i8 = 4;

/// The area the map is drawn in at the terminal's last size, if it is drawn.
fn drawn_area(app: &App) -> Option<Rect> {
    super::view::map_area(app, Rect::new(0, 0, app.size.0, app.size.1))
}

/// Whether the map is drawn, in place or as the overlay, at the terminal's last size.
pub fn is_drawn(app: &App) -> bool {
    drawn_area(app).is_some()
}

/// The app's camera after a pan of `steps`, or as it is while the map is not drawn.
pub fn panned(app: &App, steps: (i8, i8)) -> Camera {
    drawn_area(app).map_or(app.camera, |area| pan(area, app.camera, steps))
}

/// The app's camera zoomed `by` steps, or as it is while the map is not drawn.
pub fn zoomed(app: &App, by: i8) -> Camera {
    drawn_area(app).map_or(app.camera, |area| zoom(area, app.camera, by))
}

/// The camera after a pan of `across` and `down` steps (east and south positive) from the band as
/// drawn: a step is an eighth of the drawn columns or a quarter of the drawn rows, and longitude
/// wraps.
fn pan(area: Rect, camera: Camera, (across, down): (i8, i8)) -> Camera {
    let camera = settled(area, camera);
    let view = Projection::new(area, camera);
    let step = |cells: u16, parts: u16, by: i8| f64::from((cells / parts).max(1)) * f64::from(by);
    let east = step(view.columns.len, 8, across) * 2.0 * view.dot;
    let south = step(view.rows.len, 4, down) * 4.0 * view.dot;
    settled(
        area,
        Camera {
            lat: camera.lat - south,
            lon: (camera.lon + east + 180.0).rem_euclid(360.0) - 180.0,
            ..camera
        },
    )
}

/// The camera zoomed `by` steps about the band as drawn, out no further than the whole map
/// showing and in no further than [`MAX_ZOOM`].
fn zoom(area: Rect, camera: Camera, by: i8) -> Camera {
    let camera = settled(area, camera);
    let fits = |zoom: i8| Projection::new(area, Camera { zoom, ..camera }).fits();
    let widest = (-12..=0).rev().find(|zoom| fits(*zoom)).unwrap_or(-12);
    let zoom = camera.zoom.saturating_add(by).clamp(widest, MAX_ZOOM);
    settled(area, Camera { zoom, ..camera })
}

/// The camera on the latitude the view is centred on once held inside the map's bounds, so a
/// pan past an edge does not wind up.
fn settled(area: Rect, camera: Camera) -> Camera {
    let view = Projection::new(area, camera);
    let middle = f64::from(view.rows.first) + f64::from(view.rows.len) / 2.0;
    Camera {
        lat: NORTH - middle * 4.0 * view.dot,
        ..camera
    }
}

/// Where the world lands in one map area for one camera. A Braille dot is square, so a degree
/// takes the same distance both ways, and the world's columns wrap around.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Projection {
    area: Rect,
    /// Degrees a Braille dot spans, across and down alike.
    dot: f64,
    /// The world's width and height in cells.
    world: (u16, u16),
    columns: Run,
    rows: Run,
}

/// The drawn cells along one axis: the first (from the area's edge), how many, and the world
/// cell the first one shows.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Run {
    start: u16,
    len: u16,
    first: u16,
}

/// How many times the area's width the world is at `zoom` steps in; each step is √2.
fn magnification(zoom: i8) -> f64 {
    2f64.powf(f64::from(zoom) / 2.0)
}

impl Projection {
    /// The projection of `camera` into `area`. Each `as` cast stays in range: the world's width
    /// is clamped to a `u16`, its height is less than its width, and offsets wrap or clamp inside it.
    pub fn new(area: Rect, camera: Camera) -> Self {
        let width = (f64::from(area.width) * magnification(camera.zoom)).round();
        let world_cols = width.clamp(1.0, f64::from(u16::MAX)) as u16;
        let dot = 360.0 / (2.0 * f64::from(world_cols));
        let world_rows = ((NORTH - SOUTH) / (4.0 * dot)).ceil() as u16;
        let across = area.width.min(world_cols);
        let first_col = (camera.lon + 180.0) / (2.0 * dot) - f64::from(across) / 2.0;
        let columns = Run {
            start: (area.width - across) / 2,
            len: across,
            first: (first_col.round() as i64).rem_euclid(i64::from(world_cols)) as u16,
        };
        let down = area.height.min(world_rows);
        let first_row = (NORTH - camera.lat) / (4.0 * dot) - f64::from(down) / 2.0;
        let rows = Run {
            start: (area.height - down) / 2,
            len: down,
            first: first_row.round().clamp(0.0, f64::from(world_rows - down)) as u16,
        };
        Self {
            area,
            dot,
            world: (world_cols, world_rows),
            columns,
            rows,
        }
    }

    /// Whether every latitude of the map shows.
    fn fits(&self) -> bool {
        self.rows.len == self.world.1
    }

    /// The cell holding a point; `None` off the map or outside the view.
    pub fn project(&self, lat: f64, lon: f64) -> Option<(u16, u16)> {
        let (world_cols, world_rows) = self.world;
        let dot_x = ((lon + 180.0) / self.dot).round() as i64;
        let dot_y = ((NORTH - lat) / self.dot).round() as i64;
        let col = dot_x.rem_euclid(2 * i64::from(world_cols)) / 2;
        let row = (dot_y >= 0).then_some(dot_y / 4)?;
        if row >= i64::from(world_rows) {
            return None;
        }
        let along = (col - i64::from(self.columns.first)).rem_euclid(i64::from(world_cols));
        let down = row - i64::from(self.rows.first);
        let x = u16::try_from(along)
            .ok()
            .filter(|x| *x < self.columns.len)?;
        let y = u16::try_from(down).ok().filter(|y| *y < self.rows.len)?;
        Some((
            self.area.x + self.columns.start + x,
            self.area.y + self.rows.start + y,
        ))
    }

    /// The latitude and longitude at the centre of cell (`x`, `y`); `None` where no map is drawn.
    pub fn unproject(&self, x: u16, y: u16) -> Option<(f64, f64)> {
        let (col, row) = self.world_cell(x, y)?;
        let lon = -180.0 + self.dot * (2.0 * f64::from(col) + 0.5);
        let lat = NORTH - self.dot * (4.0 * f64::from(row) + 1.5);
        Some((lat, lon))
    }

    /// The latitude and longitude of the dot `across` and `down` from the first one drawn; either
    /// may run past the view, and longitude past 180E is left unwrapped.
    fn dot_at(&self, across: i32, down: i32) -> (f64, f64) {
        let world_x = 2 * i32::from(self.columns.first) + across;
        let world_y = 4 * i32::from(self.rows.first) + down;
        (
            NORTH - self.dot * f64::from(world_y),
            -180.0 + self.dot * f64::from(world_x),
        )
    }

    /// The world column and row that screen cell (`x`, `y`) shows, if it shows one.
    fn world_cell(&self, x: u16, y: u16) -> Option<(u16, u16)> {
        let along = x.checked_sub(self.area.x + self.columns.start)?;
        let down = y.checked_sub(self.area.y + self.rows.start)?;
        let shown = along < self.columns.len && down < self.rows.len;
        let col = (u32::from(self.columns.first) + u32::from(along)) % u32::from(self.world.0);
        u16::try_from(col)
            .ok()
            .filter(|_| shown)
            .map(|col| (col, self.rows.first + down))
    }

    /// The drawn area in at most two pieces, split where the world wraps, each with the canvas
    /// bounds that put its dots on this projection's grid.
    fn pieces(&self) -> Vec<(Rect, [f64; 2], [f64; 2])> {
        let Run { start, len, first } = self.columns;
        let before_wrap = len.min(self.world.0 - first);
        let top = NORTH - self.dot * 4.0 * f64::from(self.rows.first);
        let lat = [top - self.dot * (4.0 * f64::from(self.rows.len) - 1.0), top];
        let piece = |x: u16, width: u16, col: u16| {
            let west = -180.0 + self.dot * 2.0 * f64::from(col);
            let rect = Rect::new(
                self.area.x + x,
                self.area.y + self.rows.start,
                width,
                self.rows.len,
            );
            (
                rect,
                [west, west + self.dot * (2.0 * f64::from(width) - 1.0)],
                lat,
            )
        };
        [
            (before_wrap > 0).then(|| piece(start, before_wrap, first)),
            (len > before_wrap).then(|| piece(start + before_wrap, len - before_wrap, 0)),
        ]
        .into_iter()
        .flatten()
        .collect()
    }
}

/// The bit each dot of a Braille cell sets, by column then row, as Unicode numbers them.
const BRAILLE_DOTS: [[u32; 4]; 2] = [[0x01, 0x02, 0x04, 0x40], [0x08, 0x10, 0x20, 0x80]];

/// The cells the edge of the night side passes through, each with the Braille glyph of its edge
/// dots: a dot is on the edge when it is dark and a dot beside, above or below it is lit.
fn terminator(projection: &Projection, at: DateTime<Utc>) -> Vec<((u16, u16), char)> {
    let (cols, rows) = (projection.columns.len, projection.rows.len);
    let (across, down) = (2 * i32::from(cols), 4 * i32::from(rows));
    let lit: Vec<Vec<bool>> = (-1..=down)
        .map(|y| {
            (-1..=across)
                .map(|x| {
                    let (lat, lon) = projection.dot_at(x, y);
                    sun_up(lat, lon, at)
                })
                .collect()
        })
        .collect();
    let is_lit = |x: i32, y: i32| {
        usize::try_from(y + 1)
            .ok()
            .zip(usize::try_from(x + 1).ok())
            .and_then(|(y, x)| lit.get(y)?.get(x).copied())
            .unwrap_or(false)
    };
    let on_edge = |x: i32, y: i32| {
        !is_lit(x, y)
            && [(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)]
                .into_iter()
                .any(|(x, y)| is_lit(x, y))
    };
    let origin = (
        projection.area.x + projection.columns.start,
        projection.area.y + projection.rows.start,
    );
    (0..rows)
        .flat_map(|row| (0..cols).map(move |col| (col, row)))
        .filter_map(|(col, row)| {
            let bits: u32 = (0u8..2)
                .flat_map(|i| (0u8..4).map(move |j| (i, j)))
                .filter(|(i, j)| {
                    on_edge(
                        2 * i32::from(col) + i32::from(*i),
                        4 * i32::from(row) + i32::from(*j),
                    )
                })
                .map(|(i, j)| BRAILLE_DOTS[usize::from(i)][usize::from(j)])
                .sum();
            let glyph = char::from_u32(0x2800 + bits).filter(|_| bits != 0)?;
            Some(((origin.0 + col, origin.1 + row), glyph))
        })
        .collect()
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
    let projection = Projection::new(area, app.camera);
    for (piece, x_bounds, y_bounds) in projection.pieces() {
        Canvas::default()
            .marker(Marker::Braille)
            .x_bounds(x_bounds)
            .y_bounds(y_bounds)
            .paint(|ctx| {
                ctx.draw(&Map {
                    resolution: MapResolution::High,
                    color: land,
                });
            })
            .render(piece, buf);
    }
    let at = instant(app.when());
    let tint =
        Some(theme.color(Role::NightTint)).filter(|tint| !theme.is_mono() && *tint != Color::Reset);
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            let Some((lat, lon)) = projection.unproject(x, y) else {
                continue;
            };
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
    if !look.glyphs.ascii {
        for (cell, glyph) in terminator(&projection, at) {
            buf[cell].set_char(glyph).set_fg(theme.color(Role::Dawn));
        }
    }
    let (sun_lon, declination) = subsolar(at);
    if let Some((x, y)) = projection.project(declination, sun_lon) {
        buf[(x, y)].set_symbol(SUN).set_fg(theme.color(Role::Day));
    }
    for pin in pins(app, look) {
        if let Some(cell) = projection.project(pin.location.lat, pin.location.lon) {
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
        let Some(cell) = projection.project(tag.location.lat, tag.location.lon) else {
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

    const CENTRED: Camera = Camera {
        lat: 0.0,
        lon: 0.0,
        zoom: 0,
    };

    #[test]
    fn a_degree_of_longitude_and_of_latitude_take_the_same_distance_at_every_size() {
        let wrapping = Camera {
            lon: 150.0,
            zoom: 2,
            ..CENTRED
        };
        for (width, height) in [(120, 12), (250, 12), (100, 20), (60, 8)] {
            let area = Rect::new(0, 1, width, height);
            for camera in [CENTRED, wrapping] {
                let map = Projection::new(area, camera);
                let (north, west) = map.unproject(0, 1).unwrap();
                let (south, east) = map.unproject(width - 1, height).unwrap();
                let per_column = (east - west).rem_euclid(360.0) / f64::from(width - 1);
                let per_half_row = (north - south) / f64::from(2 * (height - 1));
                assert!(
                    (per_column - per_half_row).abs() < 1e-9,
                    "{width}x{height}: {per_column} degrees a column, {per_half_row} a half row"
                );
                // The canvas places the land, so its bounds must give a dot the same span too.
                for (rect, x, y) in map.pieces() {
                    let across = (x[1] - x[0]) / f64::from(2 * rect.width - 1);
                    let down = (y[1] - y[0]) / f64::from(4 * rect.height - 1);
                    assert!(
                        (across - map.dot).abs() < 1e-9 && (down - map.dot).abs() < 1e-9,
                        "{width}x{height} {camera:?}: {across} and {down} a dot, not {}",
                        map.dot
                    );
                }
            }
        }
    }

    #[test]
    fn at_zoom_0_the_world_fills_the_width() {
        let map = Projection::new(AREA, CENTRED);
        assert_eq!(map.project(0.0, -180.0).map(|(x, _)| x), Some(0));
        assert_eq!(map.project(0.0, 179.0).map(|(x, _)| x), Some(119));
    }

    #[test]
    fn the_band_never_shows_beyond_the_map_s_latitudes() {
        let north = Projection::new(
            AREA,
            Camera {
                lat: 80.0,
                ..CENTRED
            },
        );
        assert_eq!(north.project(NORTH - 0.1, 0.0).map(|(_, y)| y), Some(1));
        let south = Projection::new(
            AREA,
            Camera {
                lat: SOUTH,
                ..CENTRED
            },
        );
        assert_eq!(south.project(SOUTH + 1.0, 0.0).map(|(_, y)| y), Some(12));
    }

    #[test]
    fn an_area_taller_than_the_map_centres_it_with_blank_rows_above_and_below() {
        // At 120 columns a dot is 1.5 degrees, so 80N to 58S takes 92 dots: 23 rows of 40.
        let tall = Rect::new(0, 1, 120, 40);
        let map = Projection::new(tall, CENTRED);
        assert_eq!(map.project(NORTH - 0.1, 0.0).map(|(_, y)| y), Some(1 + 8));
        assert_eq!(
            map.project(SOUTH + 1.0, 0.0).map(|(_, y)| y),
            Some(1 + 8 + 22)
        );
        assert_eq!(map.unproject(0, 1 + 7), None);
        assert_eq!(map.unproject(0, 1 + 8 + 23), None);
    }

    #[test]
    fn a_pan_or_zoom_starts_from_the_band_as_drawn_not_a_centre_past_the_edge() {
        let past_the_edge = Camera {
            lat: -32.0,
            ..CENTRED
        };
        let drawn = settled(AREA, past_the_edge);
        assert_ne!(drawn, past_the_edge, "the band is held at the edge");
        assert_eq!(pan(AREA, past_the_edge, (0, -1)), pan(AREA, drawn, (0, -1)));
        assert_eq!(zoom(AREA, past_the_edge, 1), zoom(AREA, drawn, 1));
    }

    /// The latitudes on meridian `lon` where the sun sits on the refracted horizon (-0.833
    /// degrees) at `at`, solved in closed form from the subsolar point.
    fn latitudes_on_the_edge(lon: f64, at: DateTime<Utc>) -> Vec<f64> {
        let (sun_lon, declination) = subsolar(at);
        let d = declination.to_radians();
        let (a, b) = (d.sin(), d.cos() * (lon - sun_lon).to_radians().cos());
        let level = (-0.833f64).to_radians().sin() / a.hypot(b);
        if level.abs() > 1.0 {
            return Vec::new();
        }
        let (crossing, phi) = (level.asin(), b.atan2(a));
        [crossing - phi, std::f64::consts::PI - crossing - phi]
            .into_iter()
            .flat_map(|lat| {
                [
                    lat,
                    lat + std::f64::consts::TAU,
                    lat - std::f64::consts::TAU,
                ]
            })
            .filter(|lat| lat.abs() <= std::f64::consts::FRAC_PI_2)
            .map(f64::to_degrees)
            .collect()
    }

    /// The terminator's dots as `(across, down)` from the first dot drawn.
    fn edge_dots(map: &Projection, at: DateTime<Utc>) -> Vec<(i32, i32)> {
        let origin = (map.area.x + map.columns.start, map.area.y + map.rows.start);
        terminator(map, at)
            .into_iter()
            .flat_map(|((x, y), glyph)| {
                let bits = u32::from(glyph) - 0x2800;
                let cell = (2 * i32::from(x - origin.0), 4 * i32::from(y - origin.1));
                (0..2).flat_map(move |i| {
                    (0..4)
                        .filter(move |j| bits & BRAILLE_DOTS[i][*j] != 0)
                        .map(move |j| (cell.0 + i as i32, cell.1 + j as i32))
                })
            })
            .collect()
    }

    #[test]
    fn the_terminator_s_dots_sit_within_one_dot_of_the_computed_edge_of_night() {
        let at: DateTime<Utc> = "2026-10-26T04:00:00Z".parse().unwrap();
        for camera in [
            CENTRED,
            Camera {
                lon: 150.0,
                zoom: 3,
                ..CENTRED
            },
            Camera {
                lat: 40.0,
                lon: 20.0,
                zoom: 2,
            },
        ] {
            let map = Projection::new(AREA, settled(AREA, camera));
            let (across, down) = (2 * i32::from(map.columns.len), 4 * i32::from(map.rows.len));
            let (north, west) = map.dot_at(0, 0);
            let (south, _) = map.dot_at(0, down - 1);
            let span = map.dot * f64::from(across - 1);
            let curve: Vec<(f64, f64)> = (0..=across * 20)
                .map(|step| west + span * f64::from(step) / f64::from(across * 20))
                .flat_map(|lon| {
                    latitudes_on_the_edge(lon, at)
                        .into_iter()
                        .map(move |lat| (lat, lon))
                })
                .collect();
            let in_dots =
                |(lat, lon): (f64, f64)| ((lon - west) / map.dot, (north - lat) / map.dot);
            let dots = edge_dots(&map, at);
            assert!(!dots.is_empty(), "{camera:?}: no terminator drawn");
            let apart = |(x, y): (f64, f64), (dx, dy): (i32, i32)| {
                (x - f64::from(dx)).hypot(y - f64::from(dy))
            };
            for dot in &dots {
                let nearest = curve
                    .iter()
                    .map(|point| apart(in_dots(*point), *dot))
                    .fold(f64::INFINITY, f64::min);
                assert!(
                    nearest <= 1.0 + 1e-9,
                    "{camera:?}: {dot:?} is {nearest} dots off"
                );
            }
            let shown = curve
                .iter()
                .filter(|(lat, _)| (south..=north).contains(lat));
            for point in shown {
                let nearest = dots
                    .iter()
                    .map(|dot| apart(in_dots(*point), *dot))
                    .fold(f64::INFINITY, f64::min);
                assert!(
                    nearest <= 1.5,
                    "{camera:?}: a gap {nearest} dots wide at {point:?}"
                );
            }
        }
    }

    #[test]
    fn unproject_inverts_project_for_every_cell() {
        for camera in [
            CENTRED,
            Camera {
                lon: 150.0,
                zoom: 3,
                ..CENTRED
            },
        ] {
            let map = Projection::new(AREA, camera);
            for y in AREA.top()..AREA.bottom() {
                for x in AREA.left()..AREA.right() {
                    let (lat, lon) = map.unproject(x, y).unwrap();
                    assert_eq!(map.project(lat, lon), Some((x, y)), "({x}, {y})");
                }
            }
        }
    }
}
