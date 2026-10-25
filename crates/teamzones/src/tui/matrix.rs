//! The membership matrix: people then places down the side, groups across. Changes apply to the
//! config at once and are written when the overlay closes; until then `u` steps back one change.

use ratatui::buffer::Buffer;
use ratatui::layout::{Margin, Position, Rect};
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, Widget};
use teamzones_core::config::{Group, PersonId, Roster};
use teamzones_core::copy::{clock_text, span_text};
use teamzones_core::question::{Marks, Question, Row};

use super::app::{App, Lens, Mode};
use super::edit::{adopt, flush, reparse, undoable};
use super::keys::Key;
use super::mouse::{Kind, Pointer};
use super::notice::Notice;
use super::theme::Role;
use super::undo::Snapshot;
use super::update::Effect;
use super::update::Msg;
use super::view::{Look, put, truncate_with_mark, width_of};
use crate::write::{self, Members};

/// A name being typed in the matrix: a new group, or a new name for the group at an index.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Naming {
    Add,
    Rename(usize),
}

/// The overlay's state: the cursor, any name being typed, the state before the last change
/// (what `u` puts back), and the state when it opened (what `u` puts back once it has closed).
#[derive(Clone, Debug, PartialEq)]
pub struct Matrix {
    pub row: usize,
    pub column: usize,
    pub naming: Option<(Naming, String)>,
    pub step_back: Option<Box<Snapshot>>,
    pub opened: Box<Snapshot>,
    /// The column whose name the left button holds, and whether it has been dragged yet.
    pub held: Option<(usize, bool)>,
}

/// The matrix's rows: every person, home first, then every place, in config order.
pub fn rows(roster: &Roster) -> Vec<Row> {
    roster
        .people()
        .map(|(id, _)| Row::Person(id))
        .chain(roster.places().map(|(id, _)| Row::Place(id)))
        .collect()
}

/// Whether the group lists `row`; home belongs to every group.
pub fn member(group: &Group, row: Row) -> bool {
    match row {
        Row::Person(id) => id == PersonId::HOME || group.people.contains(&id),
        Row::Place(id) => group.places.contains(&id),
    }
}

/// The question a group asks, as the matrix heads its column.
pub fn asks(group: &Group) -> &'static str {
    match Marks::of_group(group).question() {
        Some(Question::Who) => "who",
        Some(Question::Where) => "where",
        Some(Question::Both) => "both",
        None => "nothing",
    }
}

/// `tab`: opens the matrix with the cursor on the active group and the focused row.
pub fn open(app: App) -> App {
    let column = match app.lens {
        Lens::Group(index) => index,
        Lens::All => 0,
    };
    let row = rows(app.roster())
        .iter()
        .position(|row| *row == app.focus)
        .unwrap_or(0);
    let matrix = Matrix {
        row,
        column,
        naming: None,
        step_back: None,
        opened: Box::new(Snapshot::of(&app)),
        held: None,
    };
    App {
        mode: Mode::Matrix(Box::new(matrix)),
        ..app
    }
}

/// The matrix the app shows, if it shows one.
pub fn of(app: &App) -> Option<&Matrix> {
    match &app.mode {
        Mode::Matrix(matrix) => Some(matrix),
        Mode::Normal
        | Mode::Prompt(_)
        | Mode::NewGroup(_)
        | Mode::CopyMenu
        | Mode::SortMenu
        | Mode::Help(..)
        | Mode::AddMenu(_)
        | Mode::Picker(_)
        | Mode::DriftMenu => None,
    }
}

fn with(app: App, matrix: Matrix) -> App {
    App {
        mode: Mode::Matrix(Box::new(matrix)),
        ..app
    }
}

/// Applies `change` to the matrix the app shows, keeping the app as it is when it shows none.
fn on_matrix(app: App, change: impl FnOnce(App, Matrix) -> App) -> App {
    match of(&app).cloned() {
        Some(matrix) => change(app, matrix),
        None => app,
    }
}

/// Moves the cursor by `columns` and `rows`, staying on the grid.
pub fn step(app: App, columns: isize, rows_by: isize) -> App {
    on_matrix(app, |app, matrix| {
        let last_row = rows(app.roster()).len().saturating_sub(1);
        let last_column = app.roster().groups.len().saturating_sub(1);
        let matrix = Matrix {
            row: matrix.row.saturating_add_signed(rows_by).min(last_row),
            column: matrix
                .column
                .saturating_add_signed(columns)
                .min(last_column),
            ..matrix
        };
        with(app, matrix)
    })
}

/// Puts the cursor on `row` of `column`, staying on the grid.
pub fn place_cursor(app: App, row: usize, column: usize) -> App {
    let placed = on_matrix(app, |app, matrix| {
        with(
            app,
            Matrix {
                row,
                column,
                ..matrix
            },
        )
    });
    step(placed, 0, 0)
}

fn say(app: App, text: &str) -> App {
    let now = app.now;
    App {
        notice: Some(Notice::new("tab", text, now)),
        ..app
    }
}

/// Applies `edit` to the config at once, keeping the state before it for `u`, then puts the
/// cursor back on the grid.
fn change(app: App, matrix: Matrix, edit: impl FnOnce(&mut toml_edit::DocumentMut)) -> App {
    match reparse(&app, edit) {
        Ok((text, config)) => {
            let step_back = Some(Box::new(Snapshot::of(&app)));
            let next = adopt(app, config, Some(text));
            step(
                with(
                    next,
                    Matrix {
                        step_back,
                        ..matrix
                    },
                ),
                0,
                0,
            )
        }
        Err(error) => say(app, &format!("not changed: {error}")),
    }
}

/// The group's members by name with `row` added or removed.
fn toggled(roster: &Roster, group: &Group, row: Row) -> Members {
    let flip = |listed: bool, this: bool| listed != this;
    Members {
        people: roster
            .people()
            .filter(|(id, _)| *id != PersonId::HOME)
            .filter(|(id, _)| flip(group.people.contains(id), row == Row::Person(*id)))
            .map(|(_, person)| person.name.clone())
            .collect(),
        places: roster
            .places()
            .filter(|(id, _)| flip(group.places.contains(id), row == Row::Place(*id)))
            .map(|(_, place)| place.name.clone())
            .collect(),
        rule: None,
    }
}

/// `space`: adds the row under the cursor to the group under it, or takes it out; home is locked.
pub fn toggle(app: App) -> App {
    on_matrix(app, |app, matrix| {
        let roster = app.roster();
        let row = rows(roster).get(matrix.row).copied();
        let group = roster.groups.get(matrix.column);
        match (row, group) {
            (Some(Row::Person(id)), Some(_)) if id == PersonId::HOME => {
                say(app, "home belongs to every group")
            }
            (Some(row), Some(group)) => {
                let members = toggled(roster, group, row);
                let column = matrix.column;
                change(app, matrix, |document| {
                    write::save_group(document, column, &members);
                })
            }
            (None, _) | (_, None) => app,
        }
    })
}

/// `<` and `>`: moves the group under the cursor one column, the cursor going with it.
pub fn move_column(app: App, by: isize) -> App {
    on_matrix(app, |app, matrix| {
        let from = matrix.column;
        let to = from.saturating_add_signed(by);
        if to == from || to >= app.roster().groups.len() {
            return app;
        }
        change(
            app,
            Matrix {
                column: to,
                ..matrix
            },
            |document| {
                write::move_group(document, from, to);
            },
        )
    })
}

/// `x`: closes the group under the cursor, written with the rest when the overlay closes.
pub fn close_column(app: App) -> App {
    on_matrix(app, |app, matrix| {
        let column = matrix.column;
        if column >= app.roster().groups.len() {
            return app;
        }
        change(app, matrix, |document| {
            write::remove_group(document, column);
        })
    })
}

/// `a` and `R`: starts typing a new group's name, or a new name for the group under the cursor.
pub fn start_naming(app: App, naming: Naming) -> App {
    on_matrix(app, |app, matrix| {
        let input = match naming {
            Naming::Add => String::new(),
            Naming::Rename(index) => app
                .roster()
                .groups
                .get(index)
                .map(|group| group.name.clone())
                .unwrap_or_default(),
        };
        with(
            app,
            Matrix {
                naming: Some((naming, input)),
                ..matrix
            },
        )
    })
}

/// The column the cursor is on, for `R`.
pub fn column(app: &App) -> usize {
    of(app).map_or(0, |matrix| matrix.column)
}

/// The name being typed after `edit`, which returns `None` to stop typing it.
pub fn edit_name(app: App, edit: impl FnOnce(&str) -> Option<String>) -> App {
    on_matrix(app, |app, matrix| {
        let naming = matrix
            .naming
            .and_then(|(naming, input)| edit(&input).map(|input| (naming, input)));
        with(app, Matrix { naming, ..matrix })
    })
}

/// `enter` on a name: adds the group, or renames the one under the cursor.
pub fn submit_name(app: App) -> App {
    on_matrix(app, |app, matrix| {
        let Some((naming, name)) = matrix.naming.clone() else {
            return app;
        };
        let matrix = Matrix {
            naming: None,
            ..matrix
        };
        match naming {
            Naming::Add => {
                let column = app.roster().groups.len();
                let empty = Members {
                    people: Vec::new(),
                    places: Vec::new(),
                    rule: None,
                };
                change(app, Matrix { column, ..matrix }, |document| {
                    write::push_group(document, &name, &empty);
                })
            }
            Naming::Rename(index) => change(app, matrix, |document| {
                write::rename_group(document, index, &name);
            }),
        }
    })
}

/// `u` in the matrix: puts back the state before the last change.
pub fn step_back(app: App) -> App {
    on_matrix(app, |app, matrix| match matrix.step_back.clone() {
        Some(before) => {
            let restored = before.restore(app);
            step(
                with(
                    restored,
                    Matrix {
                        step_back: None,
                        ..matrix
                    },
                ),
                0,
                0,
            )
        }
        None => app,
    })
}

/// `esc`, and `enter` after choosing a lens: closes the overlay, writing every change it made
/// with one undo back to how it opened.
pub fn finish(app: App, lens: Option<Lens>) -> (App, Vec<Effect>) {
    let Some(matrix) = of(&app).cloned() else {
        return (app, Vec::new());
    };
    let app = App {
        mode: Mode::Normal,
        ..app
    };
    let app = match lens {
        Some(lens) => App {
            marks: super::app::base_marks(app.roster(), lens),
            lens,
            ..app
        },
        None => app,
    };
    if app.text == matrix.opened.text {
        return (app, Vec::new());
    }
    flush(
        undoable(app, *matrix.opened, "tab", "groups saved  u undo"),
        "tab",
    )
}

/// `enter`: uses the group under the cursor and closes the overlay.
pub fn use_group(app: App) -> (App, Vec<Effect>) {
    let lens = of(&app)
        .filter(|matrix| matrix.column < app.roster().groups.len())
        .map(|matrix| Lens::Group(matrix.column));
    finish(app, lens)
}

/// The line the group under the cursor copies: its saved members' line now.
pub fn group_line(app: &App, group: &Group) -> String {
    teamzones_core::copy::format_line(
        app.roster(),
        &Marks::of_group(group),
        app.focus,
        app.when(),
        app.config.copy.default_format(),
    )
}

/// The `teamzones say` call that prints the group's line for the view's time.
pub fn cli_call(app: &App, group: &Group) -> String {
    format!("teamzones say {} {}", group.name, cli_time(app))
}

/// The view's time as `say` reads it: `now`, or the time or range and a day when not today.
fn cli_time(app: &App) -> String {
    if app.at_now() {
        return "now".to_owned();
    }
    let axis = app.roster().axis();
    let when = app.when();
    let start = when.start().with_timezone(&axis);
    let time = match when {
        teamzones_core::clock::When::At(_) => clock_text(&start),
        teamzones_core::clock::When::Over(range) => {
            span_text(&start, &range.end().start().with_timezone(&axis))
        }
    };
    let days = (start.date_naive() - app.today()).num_days();
    let day = match days {
        0 => String::new(),
        1..=6 => format!(" {}", start.format("%a").to_string().to_lowercase()),
        _ => format!(" {}", start.format("%Y-%m-%d")),
    };
    format!("{time}{day}")
}

/// What `space` would do on the cell under the cursor.
pub fn cursor_text(app: &App, matrix: &Matrix) -> String {
    let roster = app.roster();
    let (Some(row), Some(group)) = (
        rows(roster).get(matrix.row).copied(),
        roster.groups.get(matrix.column),
    ) else {
        return "a adds a group".to_owned();
    };
    let listed = member(group, row);
    let name = &group.name;
    match row {
        Row::Person(id) if id == PersonId::HOME => "home belongs to every group".to_owned(),
        Row::Person(id) if listed => {
            format!("space removes {} from {name}", roster.person(id).name)
        }
        Row::Person(id) => format!("space adds {} to {name}", roster.person(id).name),
        Row::Place(id) if listed => {
            format!(
                "space removes the place {} from {name}",
                roster.place(id).name
            )
        }
        Row::Place(id) => format!(
            "space adds the place {} to {name} (its state, not its zone)",
            roster.place(id).name
        ),
    }
}

/// Holds the left button on the column at `column`, or lets go with `None`.
pub fn hold(app: App, held: Option<usize>) -> App {
    on_matrix(app, |app, matrix| {
        let column = held.unwrap_or(matrix.column);
        with(
            app,
            Matrix {
                held: held.map(|column| (column, false)),
                column,
                ..matrix
            },
        )
    })
}

/// Marks the held column as dragged, so letting go of it no longer uses the group.
pub fn dragged(app: App) -> App {
    on_matrix(app, |app, matrix| {
        let held = matrix.held.map(|(column, _)| (column, true));
        with(app, Matrix { held, ..matrix })
    })
}

/// Where the overlay's pieces sit on a screen of `size`.
struct Geometry {
    area: Rect,
    inner: Rect,
    /// The columns of the row labels: the mark and name, then the city or profile.
    name_width: u16,
    label_width: u16,
    column_width: u16,
    /// The first group drawn, and how many fit.
    first: usize,
    shown: usize,
    /// The screen line of each matrix row, by its index in [`rows`].
    lines: Vec<(usize, u16)>,
}

const COLUMN: u16 = 11;
const NEW: &str = "+ new";

fn geometry(app: &App, matrix: &Matrix, size: Rect) -> Geometry {
    let roster = app.roster();
    let people = roster.people().count();
    let places = roster.places().count();
    let wide = |texts: Vec<usize>| u16::try_from(texts.into_iter().max().unwrap_or(0)).unwrap_or(0);
    let name_width =
        2 + wide(
            roster
                .people()
                .map(|(_, p)| width_of(&p.name))
                .chain(roster.places().map(|(_, p)| width_of(&p.name)))
                .collect(),
        ) + 2;
    let side = wide(
        roster
            .people()
            .map(|(_, p)| width_of(&p.city))
            .chain(roster.places().map(|(_, p)| width_of(&profile_text(p))))
            .collect(),
    ) + 2;
    let label_width = name_width + side;
    let groups = roster.groups.len();
    let wanted = label_width + COLUMN * u16::try_from(groups + 1).unwrap_or(u16::MAX) + 4;
    let width = size.width.min(wanted.max(60));
    let lines_needed = 2 + people + 1 + places + 1 + 3 + 2;
    let height = size
        .height
        .min(u16::try_from(lines_needed).unwrap_or(u16::MAX));
    let area = Rect::new(
        size.x + (size.width - width) / 2,
        size.y + (size.height - height) / 2,
        width,
        height,
    );
    let inner = area.inner(Margin::new(1, 1));
    let room = inner.width.saturating_sub(label_width + 1 + COLUMN);
    let shown = usize::from(room / COLUMN).max(1);
    let first = matrix.column.saturating_sub(shown - 1);
    let top = inner.y + 2;
    let lines = (0..people + places)
        .map(|index| {
            let gap = u16::from(index >= people);
            (index, top + u16::try_from(index).unwrap_or(u16::MAX) + gap)
        })
        .filter(|(_, y)| *y < inner.bottom().saturating_sub(4))
        .collect();
    Geometry {
        area,
        inner,
        name_width,
        label_width,
        column_width: COLUMN,
        first,
        shown,
        lines,
    }
}

fn profile_text(place: &teamzones_core::config::Place) -> String {
    place
        .profile
        .as_ref()
        .map_or_else(|| "no profile".to_owned(), |name| format!("{name} profile"))
}

impl Geometry {
    /// The screen column where group `index` starts, when it is drawn.
    fn column_x(&self, index: usize) -> Option<u16> {
        (self.first..self.first + self.shown)
            .contains(&index)
            .then(|| {
                let offset = u16::try_from(index - self.first).unwrap_or(0);
                self.inner.x + 1 + self.label_width + offset * self.column_width
            })
    }

    /// The group whose column holds screen column `x`.
    fn column_at(&self, x: u16, groups: usize) -> Option<usize> {
        (self.first..(self.first + self.shown).min(groups)).find(|index| {
            self.column_x(*index)
                .is_some_and(|left| (left..left + self.column_width).contains(&x))
        })
    }

    fn row_at(&self, y: u16) -> Option<usize> {
        self.lines
            .iter()
            .find(|(_, line)| *line == y)
            .map(|(index, _)| *index)
    }
}

/// Draws the overlay over the screen.
pub fn draw(buf: &mut Buffer, size: Rect, app: &App, matrix: &Matrix, look: Look) {
    let (theme, glyphs) = (look.theme, look.glyphs);
    let roster = app.roster();
    let at = geometry(app, matrix, size);
    Clear.render(at.area, buf);
    Block::bordered()
        .border_set(glyphs.rounded)
        .border_style(theme.fg(Role::Accent))
        .title(Span::styled(
            " groups ",
            theme.fg(Role::Text).add_modifier(Modifier::BOLD),
        ))
        .title_bottom(
            Line::from(Span::styled(
                " written to config.toml on close ",
                theme.fg(Role::Dim),
            ))
            .right_aligned(),
        )
        .render(at.area, buf);
    let inner = at.inner;
    let x = inner.x + 1;
    put(buf, inner, x, inner.y + 1, "asks", theme.fg(Role::Dim));
    for (index, group) in roster.groups.iter().enumerate() {
        let Some(left) = at.column_x(index) else {
            continue;
        };
        let number = if index < 9 {
            format!("{} ", index + 1)
        } else {
            String::new()
        };
        let room = usize::from(at.column_width - 1);
        let name = truncate_with_mark(&format!("{number}{}", group.name), room, ".");
        let style = if index == matrix.column {
            theme
                .fg(Role::Accent)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
        } else {
            theme.fg(Role::Text)
        };
        put(buf, inner, left, inner.y, &name, style);
        put(
            buf,
            inner,
            left + 2,
            inner.y + 1,
            asks(group),
            theme.fg(Role::Dim),
        );
    }
    let shown_end = (at.first + at.shown).min(roster.groups.len());
    if let Some(left) = at.column_x(shown_end.saturating_sub(1)) {
        let after = if roster.groups.is_empty() {
            x + at.label_width
        } else {
            left + at.column_width
        };
        put(buf, inner, after, inner.y, NEW, theme.fg(Role::Dim));
    }
    let all = rows(roster);
    let people = roster.people().count();
    if let Some((_, y)) = at.lines.iter().find(|(index, _)| *index == people) {
        put(buf, inner, x, y - 1, "places", theme.fg(Role::Dim));
    }
    for (index, y) in &at.lines {
        let Some(row) = all.get(*index).copied() else {
            continue;
        };
        let (mark, name, side) = match row {
            Row::Person(id) => {
                let person = roster.person(id);
                let mark = if id == PersonId::HOME {
                    glyphs.home
                } else {
                    " "
                };
                (mark, person.name.clone(), person.city.clone())
            }
            Row::Place(id) => {
                let place = roster.place(id);
                (" ", place.name.clone(), profile_text(place))
            }
        };
        put(buf, inner, x, *y, mark, theme.fg(Role::Marker));
        put(buf, inner, x + 2, *y, &name, theme.fg(Role::Text));
        put(
            buf,
            inner,
            x + at.name_width,
            *y,
            &side,
            theme.fg(Role::Dim),
        );
        for (column, group) in roster.groups.iter().enumerate() {
            let Some(left) = at.column_x(column) else {
                continue;
            };
            let (glyph, role) = match (row, member(group, row)) {
                (Row::Person(id), _) if id == PersonId::HOME => (glyphs.home, Role::Marker),
                (Row::Person(_), true) => (glyphs.person, Role::Marker),
                (Row::Place(_), true) => (glyphs.place, Role::Marker),
                (Row::Person(_) | Row::Place(_), false) => ("·", Role::Dim),
            };
            let glyph = glyphs.text(glyph);
            let mut style = theme.fg(role);
            if column == matrix.column && *index == matrix.row {
                style = style.patch(theme.selection);
            }
            put(buf, inner, left + 2, *y, &glyph, style);
        }
    }
    let bottom = inner.bottom().saturating_sub(3);
    let room = usize::from(inner.width.saturating_sub(10));
    let group = roster.groups.get(matrix.column);
    let lines: [(String, String); 3] = [
        (
            group.map_or_else(String::new, |group| group.name.clone()),
            group.map_or_else(String::new, |group| group_line(app, group)),
        ),
        (
            "cli".to_owned(),
            group.map_or_else(String::new, |group| cli_call(app, group)),
        ),
        match &matrix.naming {
            Some((_, input)) => ("name".to_owned(), format!("{input}_")),
            None => ("cursor".to_owned(), cursor_text(app, matrix)),
        },
    ];
    for ((label, text), y) in lines.iter().zip(bottom..) {
        let label = truncate_with_mark(label, 8, ".");
        put(buf, inner, x, y, &label, theme.fg(Role::Dim));
        let text = truncate_with_mark(&glyphs.text(text), room, &glyphs.text("…"));
        put(buf, inner, x + 9, y, &text, theme.fg(Role::Text));
    }
}

/// The messages a pointer event stands for while the overlay is open: a click outside closes it,
/// a click on a cell moves there and toggles it, and a column's name is used on a click, moved
/// on a drag, renamed on a right click and closed on a middle click.
pub fn pointer(app: &App, matrix: &Matrix, size: Rect, pointer: Pointer) -> Vec<Msg> {
    let at = geometry(app, matrix, size);
    let here = Position::new(pointer.x, pointer.y);
    let groups = app.roster().groups.len();
    let column = at.column_at(pointer.x, groups);
    let key = |c: char| Msg::Key(Key::Char(c));
    let on = |column: usize| Msg::MatrixCell(matrix.row, column);
    match pointer.kind {
        Kind::Down if !at.area.contains(here) => vec![Msg::Key(Key::Esc)],
        Kind::Down if pointer.y == at.inner.y => column
            .map(|column| Msg::MatrixHold(Some(column)))
            .into_iter()
            .collect(),
        Kind::Down => match (at.row_at(pointer.y), column) {
            (Some(row), Some(column)) => vec![Msg::MatrixCell(row, column), key(' ')],
            (Some(_) | None, Some(_) | None) => Vec::new(),
        },
        Kind::Drag => match (matrix.held, column) {
            (Some((from, _)), Some(to)) if to != from => {
                let c = if to < from { '<' } else { '>' };
                let steps = from.abs_diff(to);
                std::iter::repeat_n(key(c), steps)
                    .chain([Msg::MatrixDragged])
                    .collect()
            }
            (Some(_) | None, Some(_) | None) => Vec::new(),
        },
        Kind::Up => match matrix.held {
            Some((_, false)) => vec![Msg::MatrixHold(None), Msg::Key(Key::Enter)],
            Some((_, true)) => vec![Msg::MatrixHold(None)],
            None => Vec::new(),
        },
        Kind::Right => column.map_or_else(Vec::new, |column| vec![on(column), key('R')]),
        Kind::Middle => column.map_or_else(Vec::new, |column| vec![on(column), key('x')]),
        Kind::ScrollUp | Kind::ScrollDown | Kind::ScrollLeft | Kind::ScrollRight | Kind::Other => {
            Vec::new()
        }
    }
}
