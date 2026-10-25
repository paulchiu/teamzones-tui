//! The mouse: a pointer event becomes the messages its keymap row's key would send, hit-tested
//! against the areas the last render drew at the terminal's last reported size.

use ratatui::layout::{Position, Rect};
use teamzones_core::clock::Slot;
use teamzones_core::fit::slot_fits;
use teamzones_core::question::Row;

use super::app::{App, Mode};
use super::glyphs::Glyphs;
use super::keys::Key;
use super::layout::{Areas, MapView};
use super::theme::{TerminalEnv, Theme};
use super::update::Msg;
use super::view::{self, Look, Track};

/// What the pointer did: the left button's press, drag and release, or a wheel notch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Down,
    Drag,
    Up,
    ScrollUp,
    ScrollDown,
    ScrollLeft,
    ScrollRight,
    /// Moves, and the other buttons, which nothing reads.
    Other,
}

/// A pointer event at a cell, independent of the terminal library that read it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pointer {
    pub kind: Kind,
    pub x: u16,
    pub y: u16,
    pub shift: bool,
}

impl Pointer {
    /// The pointer event a crossterm mouse event names.
    pub fn from_crossterm(event: crossterm::event::MouseEvent) -> Self {
        use crossterm::event::{KeyModifiers, MouseButton, MouseEventKind};
        let kind = match event.kind {
            MouseEventKind::Down(MouseButton::Left) => Kind::Down,
            MouseEventKind::Drag(MouseButton::Left) => Kind::Drag,
            MouseEventKind::Up(MouseButton::Left) => Kind::Up,
            MouseEventKind::ScrollUp => Kind::ScrollUp,
            MouseEventKind::ScrollDown => Kind::ScrollDown,
            MouseEventKind::ScrollLeft => Kind::ScrollLeft,
            MouseEventKind::ScrollRight => Kind::ScrollRight,
            MouseEventKind::Down(_)
            | MouseEventKind::Up(_)
            | MouseEventKind::Drag(_)
            | MouseEventKind::Moved => Kind::Other,
        };
        Self {
            kind,
            x: event.column,
            y: event.row,
            shift: event.modifiers.contains(KeyModifiers::SHIFT),
        }
    }
}

/// What a left-button drag holds: a range edge, the range's middle, or a range being drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Grab {
    Start,
    End,
    Middle,
    /// Drawing from the cell pressed; `moved` once the visual range has started.
    Draw {
        origin: u16,
        moved: bool,
    },
}

/// A drag in progress: what it holds, and the slot it last moved that to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Drag {
    pub grab: Grab,
    pub at: Slot,
}

fn key(c: char) -> Msg {
    Msg::Key(Key::Char(c))
}

/// `count` presses of `later` when positive, of `earlier` when negative.
fn presses(count: i64, earlier: char, later: char) -> Vec<Msg> {
    let c = if count < 0 { earlier } else { later };
    (0..count.unsigned_abs()).map(|_| key(c)).collect()
}

/// The messages `pointer` stands for on the app's screen.
pub fn msgs(app: &App, pointer: Pointer) -> Vec<Msg> {
    let theme = Theme::resolve(&app.config.theme, TerminalEnv::default());
    let look = Look {
        theme: &theme,
        glyphs: Glyphs::of(app.config.theme.glyphs),
    };
    let size = Rect::new(0, 0, app.size.0, app.size.1);
    let at = Position::new(pointer.x, pointer.y);
    let outside = |area: Rect| pointer.kind == Kind::Down && !area.contains(at);
    match &app.mode {
        Mode::CopyMenu => {
            let rows = app.copy_menu();
            let (area, step) = view::copy_dialog_area(size, rows.len());
            let inner = area.inner(ratatui::layout::Margin::new(1, 1));
            if outside(area) {
                return vec![Msg::Key(Key::Esc)];
            }
            let picked = (pointer.kind == Kind::Down && inner.contains(at))
                .then(|| rows.get(usize::from((pointer.y - inner.y) / step)))
                .flatten();
            picked.map(|(c, _, _)| key(*c)).into_iter().collect()
        }
        Mode::Help(from) if outside(view::help_area(size, *from, look)) => {
            vec![Msg::Key(Key::Esc)]
        }
        Mode::Normal if app.map_view() == MapView::Overlay => {
            let area = view::map_overlay_area(size);
            match pointer.kind {
                Kind::Down if area.contains(at) => Vec::new(),
                Kind::Down => vec![Msg::Key(Key::Esc)],
                Kind::Drag
                | Kind::Up
                | Kind::ScrollUp
                | Kind::ScrollDown
                | Kind::ScrollLeft
                | Kind::ScrollRight
                | Kind::Other => main(app, look, size, pointer),
            }
        }
        Mode::Normal => main(app, look, size, pointer),
        Mode::Help(_) | Mode::Prompt(_) | Mode::SortMenu => Vec::new(),
    }
}

/// The screen lines of the slider: the ruler's lane, the track's label and the track's lane.
struct Slider {
    ruler: u16,
    label: u16,
    track: u16,
}

fn slider(areas: &Areas, track: &Track) -> Slider {
    let ruler = areas.slider.y + track.dy;
    Slider {
        ruler,
        label: ruler + 1,
        track: ruler + 1 + track.dy,
    }
}

/// The track cell under column `x`, if it is over the track.
fn cell_at(track: &Track, x: u16) -> Option<u16> {
    (track.x..track.x + track.cells)
        .contains(&x)
        .then(|| x - track.x)
}

/// The track cell nearest column `x`, for a drag that leaves the track.
fn nearest_cell(track: &Track, x: u16) -> u16 {
    x.clamp(track.x, track.x + track.cells.saturating_sub(1)) - track.x
}

/// The row whose label line is screen line `y`, in either block.
fn row_at(app: &App, areas: &Areas, track: &Track, y: u16) -> Option<Row> {
    let lines = track.dy + 1;
    let block = |top: u16, rows: super::app::BlockRows| {
        let above = rows.above.len();
        rows.above
            .into_iter()
            .map(Some)
            .chain(std::iter::once(None))
            .chain(rows.below.into_iter().map(Some))
            .enumerate()
            .find(|(index, _)| {
                u16::try_from(*index).is_ok_and(|index| top + 1 + index * lines == y)
            })
            .and_then(|(index, row)| (index != above).then_some(row).flatten())
    };
    block(areas.who.y, app.who_rows()).or_else(|| block(areas.place.y, app.where_rows()))
}

/// Whether screen line `y` is a lane: a row's or answer row's lane, or the ruler's.
fn on_lane(areas: &Areas, track: &Track, slider: &Slider, y: u16) -> bool {
    let lines = track.dy + 1;
    let lane_of = |top: u16, height: u16| {
        (top + 1..top + height).any(|line| (line - top - 1) % lines == track.dy && line == y)
    };
    y == slider.ruler
        || lane_of(areas.who.y, areas.who.height)
        || lane_of(areas.place.y, areas.place.height)
}

fn main(app: &App, look: Look, size: Rect, pointer: Pointer) -> Vec<Msg> {
    let areas = view::areas(app, size);
    let track = view::track_for(app, &areas, look);
    let grid = areas.slider.union(areas.who).union(areas.place);
    let over_grid = grid.contains(Position::new(pointer.x, pointer.y));
    match pointer.kind {
        Kind::ScrollDown if over_grid => vec![key(if pointer.shift { 'L' } else { 'l' })],
        Kind::ScrollUp if over_grid => vec![key(if pointer.shift { 'H' } else { 'h' })],
        Kind::ScrollRight if over_grid => vec![key(']')],
        Kind::ScrollLeft if over_grid => vec![key('[')],
        Kind::Down => down(app, look, &areas, &track, pointer),
        Kind::Drag => app
            .drag
            .map_or_else(Vec::new, |drag| dragged(&track, drag, pointer)),
        Kind::Up => released(app, &areas, &track, pointer),
        Kind::ScrollDown | Kind::ScrollUp | Kind::ScrollRight | Kind::ScrollLeft | Kind::Other => {
            Vec::new()
        }
    }
}

/// The key a click on the who strip stands for: `s` on its title, a group's number on its name
/// or number, and `b` on the legend; the drift tag and the trailing gap send nothing.
fn strip_click(app: &App, look: Look, areas: &Areas, track: &Track, x: u16) -> Option<Msg> {
    let strip = view::strip(app, look);
    let legend = view::who_legend(app, look, track);
    let legend_width = u16::try_from(legend.width()).unwrap_or(u16::MAX);
    if view::legend_fits(&strip, &legend, areas.who.width)
        && x >= areas.who.right().saturating_sub(legend_width)
    {
        return Some(key('b'));
    }
    let mut left = areas.who.x;
    let mut after_digit = None;
    for (index, span) in strip.spans.iter().enumerate() {
        let width = u16::try_from(span.width()).unwrap_or(u16::MAX);
        let digit = span.content.trim().parse::<u8>().ok();
        let number = digit.or(after_digit);
        after_digit = digit;
        if (left..left + width).contains(&x) {
            return match (index, number) {
                (0, _) => Some(key('s')),
                (_, Some(n)) => char::from_digit(u32::from(n), 10).map(key),
                (_, None) => None,
            };
        }
        left += width;
    }
    None
}

/// The grab a press on track cell `cell` takes: an edge or the middle of the range it hits, or
/// a new range drawn from that cell.
fn grab_at(track: &Track, cell: u16, ranged: bool) -> Grab {
    let selected: Vec<u16> = (0..track.cells)
        .filter(|index| track.selected(*index))
        .collect();
    match (ranged, selected.first(), selected.last()) {
        (true, Some(&first), Some(&last)) if first != last && cell == first => Grab::Start,
        (true, Some(&first), Some(&last)) if first != last && cell == last => Grab::End,
        (true, Some(&first), Some(&last)) if first < cell && cell < last => Grab::Middle,
        _ => Grab::Draw {
            origin: cell,
            moved: false,
        },
    }
}

fn down(app: &App, look: Look, areas: &Areas, track: &Track, pointer: Pointer) -> Vec<Msg> {
    let Pointer { x, y, .. } = pointer;
    let slider = slider(areas, track);
    if y == areas.header.y && view::header_now_columns(app).contains(&x) {
        return vec![key('.')];
    }
    if y == slider.label && app.day() != app.today() && track.label > 0 && x == track.label - 1 {
        return vec![key('.')];
    }
    if y == areas.who.y {
        return strip_click(app, look, areas, track, x)
            .into_iter()
            .collect();
    }
    let where_width = u16::try_from(view::width_of(view::WHERE_TITLE)).unwrap_or(0);
    if y == areas.place.y && x < areas.place.x + where_width {
        return vec![key('s')];
    }
    let (copy_key, copy_label) = view::COPY_HINT;
    let copy_hint =
        u16::try_from(view::width_of(copy_key) + view::width_of(copy_label)).unwrap_or(0);
    let say_title = areas.say.y + 1;
    let copy_right = areas.say.right().saturating_sub(1);
    if y == say_title && (copy_right.saturating_sub(copy_hint)..copy_right).contains(&x) {
        return vec![key('y')];
    }
    if y == areas.notice.y && undo_columns(app).is_some_and(|columns| columns.contains(&x)) {
        return vec![key('u')];
    }
    let in_label = track.dy == 1 || x < track.label;
    if let Some(row) = row_at(app, areas, track, y).filter(|_| in_label) {
        let focus = Msg::Focus(row);
        return if x == areas.who.x {
            vec![focus, key(' ')]
        } else {
            vec![focus]
        };
    }
    let Some(cell) = cell_at(track, x) else {
        return Vec::new();
    };
    let grab = if y == slider.track {
        grab_at(track, cell, app.shown_range().is_some())
    } else if on_lane(areas, track, &slider, y) {
        Grab::Draw {
            origin: cell,
            moved: false,
        }
    } else {
        return Vec::new();
    };
    vec![Msg::Drag(Some(Drag {
        grab,
        at: track.first_slot(cell),
    }))]
}

/// The notice row's columns under its `u undo`, while a notice offering it shows.
fn undo_columns(app: &App) -> Option<std::ops::Range<u16>> {
    let notice = app
        .notice
        .as_ref()
        .filter(|notice| notice.visible(app.now))?;
    let (before, _) = notice.text.split_once("u undo")?;
    let start = u16::try_from(view::width_of(before)).ok()?;
    Some(start..start + 6)
}

/// The slot a drawn range's moving end reaches over `cell`: its last slot drawing later than the
/// origin, its first slot drawing earlier.
fn reach(track: &Track, origin: u16, cell: u16) -> Slot {
    if cell >= origin {
        track.slots(cell).last().unwrap_or(track.first_slot(cell))
    } else {
        track.first_slot(cell)
    }
}

fn dragged(track: &Track, drag: Drag, pointer: Pointer) -> Vec<Msg> {
    let cell = nearest_cell(track, pointer.x);
    let (moves, at, grab) = match drag.grab {
        Grab::Draw {
            origin,
            moved: false,
        } if cell == origin => return Vec::new(),
        Grab::Draw { origin, moved } => {
            let at = reach(track, origin, cell);
            let start = (!moved).then(|| {
                let from = track.first_slot(origin);
                vec![Msg::Cursor(from), key('v')]
            });
            let from = if moved {
                drag.at
            } else {
                track.first_slot(origin)
            };
            let moves = start
                .unwrap_or_default()
                .into_iter()
                .chain(presses(at.since(from), 'h', 'l'))
                .collect();
            (
                moves,
                at,
                Grab::Draw {
                    origin,
                    moved: true,
                },
            )
        }
        grab => {
            let at = track.first_slot(cell);
            let (earlier, later) = match grab {
                Grab::Start => ('{', '}'),
                Grab::End => ('<', '>'),
                Grab::Middle | Grab::Draw { .. } => ('h', 'l'),
            };
            (presses(at.since(drag.at), earlier, later), at, grab)
        }
    };
    moves
        .into_iter()
        .chain(std::iter::once(Msg::Drag(Some(Drag { grab, at }))))
        .collect()
}

fn released(app: &App, areas: &Areas, track: &Track, pointer: Pointer) -> Vec<Msg> {
    let Some(drag) = app.drag else {
        return Vec::new();
    };
    let on_track = pointer.y == slider(areas, track).track;
    let ending = match drag.grab {
        Grab::Draw { moved: true, .. } => Some(Msg::Key(Key::Enter)),
        Grab::Draw { moved: false, .. } if on_track => cell_at(track, pointer.x)
            .and_then(|cell| {
                track
                    .slots(cell)
                    .find(|slot| slot_fits(app.roster(), &app.marks, *slot))
            })
            .map(Msg::Run),
        Grab::Draw { moved: false, .. } | Grab::Start | Grab::End | Grab::Middle => None,
    };
    ending
        .into_iter()
        .chain(std::iter::once(Msg::Drag(None)))
        .collect()
}
