//! The pure update: a message in, the next state and the effects to run out.

use chrono::{DateTime, Utc};
use teamzones_core::clock::{Range, Slot, When};
use teamzones_core::config::PersonId;
use teamzones_core::copy::{clock_text, span_text};
use teamzones_core::fit::{NoFit, blocker, next_run, previous_run, slot_fits};
use teamzones_core::question::{Marks, Row, person_band, place_level};
use teamzones_core::time::Moment;

use super::app::{App, Cursor, Lanes, Lens, Mode, Visual, base_marks, next_style, style_name};
use super::edit;
use super::editor::Loaded;
use super::keys::{Action, Context, Key, action_for};
use super::layout::MapView;
use super::map;
use super::matrix;
use super::mouse::{self, Drag, Pointer};
use super::notice::{Notice, TIMEOUT};
use super::picker;
use super::prompt;
use super::say::duration_text;
use super::undo::{Snapshot, Undo};

/// Slots in an hour and in a day, for the coarse moves.
const HOUR: i64 = 4;
const DAY: i64 = 96;

/// Something that happened: a key, a clock tick, a copy's result, or a reloaded config.
#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    Key(Key),
    Tick(DateTime<Utc>),
    Copied {
        key: String,
        text: String,
        result: Result<(), String>,
    },
    /// The config read afresh by `r` or after `E`, or the notice text saying why not.
    Reloaded {
        key: String,
        result: Result<Box<Loaded>, String>,
    },
    /// The terminal is now this many columns and rows.
    Resize(u16, u16),
    /// Whether switching mouse capture to `on` worked.
    Captured {
        on: bool,
        result: Result<(), String>,
    },
    /// A pointer event, which stands for the messages its keymap row's key would send.
    Mouse(Pointer),
    /// Whether writing the config over `over` worked.
    Wrote {
        key: String,
        over: String,
        result: Result<(), String>,
    },
    /// Focus this row.
    Focus(Row),
    /// Put the cursor on this slot, dropping any range.
    Cursor(Slot),
    /// Move the range to the start of the fitting run holding this slot.
    Run(Slot),
    /// The drag in progress, or none once the button is released.
    Drag(Option<Drag>),
    /// Put the matrix cursor on this row and column.
    MatrixCell(usize, usize),
    /// Hold the left button on this matrix column's name, or let go.
    MatrixHold(Option<usize>),
    /// The held matrix column has been dragged.
    MatrixDragged,
    /// Add a person or a place at this latitude and longitude, from a double click on the map.
    AddAt(f64, f64),
    /// Open the drift tag's menu.
    DriftMenu,
    /// The left button went down at this cell at this point on the loop's clock, for spotting a
    /// double click.
    Clicked(u16, u16, chrono::TimeDelta),
}

/// Work the effects layer does on the update's behalf.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Effect {
    Copy {
        key: String,
        text: String,
    },
    /// Turn mouse capture on or off.
    CaptureMouse(bool),
    /// Read the config file afresh.
    Reload,
    /// Suspend the TUI, run the editor on the config file and reload it, recapturing the mouse
    /// afterwards when `mouse`.
    Edit {
        mouse: bool,
    },
    /// Replace the config file's text, `over` being what the file should still hold, so an edit
    /// made outside the app is never written over.
    Write {
        key: String,
        text: String,
        over: String,
    },
}

/// The context whose bindings read the next key: a modal, range mode while drawing, or main.
pub fn context(app: &App) -> Context {
    match &app.mode {
        Mode::Normal if app.visual.is_some() => Context::Range,
        Mode::Normal => Context::Main,
        Mode::Prompt(_) | Mode::NewGroup(_) => Context::Prompt,
        Mode::Matrix(matrix) if matrix.naming.is_some() => Context::Prompt,
        Mode::Matrix(_) => Context::Matrix,
        Mode::Picker(_) => Context::Prompt,
        Mode::AddMenu(_) => Context::AddMenu,
        Mode::DriftMenu => Context::DriftMenu,
        Mode::Map => Context::Map,
        Mode::CopyMenu => Context::CopyMenu,
        Mode::SortMenu => Context::SortMenu,
        Mode::Help(..) => Context::Help,
    }
}

/// Applies `msg`, then re-clamps focus onto a row that is still there.
pub fn update(app: App, msg: Msg) -> (App, Vec<Effect>) {
    let rows_before = app.focus_rows();
    let (mut next, effects) = match msg {
        Msg::Key(key) => on_key(app, key),
        Msg::Tick(now) => tick(app, now),
        Msg::Copied { key, text, result } => {
            let words = match result {
                Ok(()) => "copied".to_owned(),
                Err(_) => format!("clipboard unavailable: {text}"),
            };
            let now = app.now;
            (with_notice(app, &key, &words, now), Vec::new())
        }
        Msg::Resize(cols, rows) => (resized(app, cols, rows), Vec::new()),
        Msg::Mouse(pointer) => {
            let msgs = mouse::msgs(&app, pointer);
            msgs.into_iter()
                .fold((app, Vec::new()), |(app, mut effects), msg| {
                    let (next, more) = update(app, msg);
                    effects.extend(more);
                    (next, effects)
                })
        }
        Msg::Focus(row) => (App { focus: row, ..app }, Vec::new()),
        Msg::Cursor(slot) => (
            App {
                cursor: Cursor::At(slot),
                range: None,
                visual: None,
                ..app
            },
            Vec::new(),
        ),
        Msg::Run(slot) => (run_at(app, slot), Vec::new()),
        Msg::Drag(drag) => (App { drag, ..app }, Vec::new()),
        Msg::MatrixCell(row, column) => (matrix::place_cursor(app, row, column), Vec::new()),
        Msg::MatrixHold(held) => (matrix::hold(app, held), Vec::new()),
        Msg::MatrixDragged => (matrix::dragged(app), Vec::new()),
        Msg::DriftMenu => (
            App {
                mode: Mode::DriftMenu,
                ..app
            },
            Vec::new(),
        ),
        Msg::AddAt(lat, lon) => (
            App {
                mode: Mode::AddMenu(Some((lat, lon))),
                ..app
            },
            Vec::new(),
        ),
        Msg::Clicked(x, y, at) => (
            App {
                last_click: Some((x, y, at)),
                ..app
            },
            Vec::new(),
        ),
        Msg::Wrote { key, over, result } => match result {
            Ok(()) => (app, Vec::new()),
            Err(error) => {
                let now = app.now;
                let kept = App {
                    disk: Some(over),
                    ..app
                };
                let text = format!("not saved: {error}");
                (with_notice(kept, &key, &text, now), Vec::new())
            }
        },
        Msg::Captured { on, result } => {
            let now = app.now;
            match result {
                Ok(()) => (app, Vec::new()),
                Err(error) => {
                    let kept = App { mouse: !on, ..app };
                    let text = format!("mouse capture failed: {error}");
                    (with_notice(kept, "M", &text, now), Vec::new())
                }
            }
        }
        Msg::Reloaded { key, result } => (edit::reloaded(app, &key, result), Vec::new()),
    };
    next.focus = clamp_focus(&rows_before, &next.focus_rows(), next.focus);
    (next, effects)
}

/// The app on a `cols` by `rows` terminal: the map's choice and view start afresh, and map mode
/// ends when the map is no longer drawn.
fn resized(app: App, cols: u16, rows: u16) -> App {
    let app = App {
        size: (cols, rows),
        map_flipped: false,
        camera: map::framing(&app),
        ..app
    };
    if app.mode == Mode::Map && !map::is_drawn(&app) {
        normal(app)
    } else {
        app
    }
}

/// Keeps focus on its row when that row is still there, else on the row now at its old index.
fn clamp_focus(before: &[Row], after: &[Row], focus: Row) -> Row {
    if after.contains(&focus) {
        return focus;
    }
    let index = before.iter().position(|row| *row == focus).unwrap_or(0);
    after
        .get(index.min(after.len().saturating_sub(1)))
        .copied()
        .unwrap_or(Row::Person(PersonId::HOME))
}

/// Samples `now`, expiring notices and writing a pending close once its window has passed.
fn tick(app: App, now: DateTime<Utc>) -> (App, Vec<Effect>) {
    let notice = app.notice.filter(|notice| notice.visible(now));
    let style_flash = app.style_flash.filter(|flash| flash.visible(now));
    let app = App {
        now,
        notice,
        style_flash,
        ..app
    };
    match app.flush_at {
        Some(due) if due <= now => edit::flush(app, "x"),
        Some(_) | None => (app, Vec::new()),
    }
}

fn with_notice(app: App, key: &str, text: &str, now: DateTime<Utc>) -> App {
    App {
        notice: Some(Notice::new(key, text, now)),
        ..app
    }
}

fn on_key(app: App, key: Key) -> (App, Vec<Effect>) {
    let Some(action) = format_key(&app, key).or_else(|| action_for(context(&app), key)) else {
        return (app, Vec::new());
    };
    let app = App {
        notice: None,
        ..app
    };
    act(app, action, key)
}

/// In the copy-as dialog, a key that names one of the config's formats copies that format.
fn format_key(app: &App, key: Key) -> Option<Action> {
    let Key::Char(c) = key else { return None };
    let in_dialog = context(app) == Context::CopyMenu;
    (in_dialog && app.config.copy.get(c).is_some()).then_some(Action::CopyAs(c))
}

fn copy(key: &str, text: String) -> Vec<Effect> {
    vec![Effect::Copy {
        key: key.to_owned(),
        text,
    }]
}

fn normal(app: App) -> App {
    App {
        mode: Mode::Normal,
        ..app
    }
}

fn act(app: App, action: Action, key: Key) -> (App, Vec<Effect>) {
    let only = |app| (app, Vec::new());
    match action {
        Action::Earlier(slots) => only(shift(app, -slots)),
        Action::Later(slots) => only(shift(app, slots)),
        Action::PreviousDay => only(shift(app, -DAY)),
        Action::NextDay => only(shift(app, DAY)),
        Action::Now => only(App {
            cursor: Cursor::Now,
            range: None,
            visual: None,
            ..app
        }),
        Action::TypeTime => only(App {
            mode: Mode::Prompt(String::new()),
            ..app
        }),
        Action::Visual => only(visual(app)),
        Action::Commit => only(commit(app)),
        Action::SwapEnd => only(App {
            visual: app.visual.map(|v| Visual {
                anchor: v.active,
                active: v.anchor,
                ..v
            }),
            ..app
        }),
        Action::StartEarlier => only(edge(app, -1, 0)),
        Action::StartLater => only(edge(app, 1, 0)),
        Action::EndEarlier => only(edge(app, 0, -1)),
        Action::EndLater => only(edge(app, 0, 1)),
        Action::NextFit => only(fit_step(app, Step::Next)),
        Action::PreviousFit => only(fit_step(app, Step::Previous)),
        Action::Unwind => only(unwind(app)),
        Action::FocusUp => only(focus_by(app, -1)),
        Action::FocusDown => only(focus_by(app, 1)),
        Action::FocusFirst => only(focus_by(app, isize::MIN)),
        Action::FocusLast => only(focus_by(app, isize::MAX)),
        Action::ToggleMark => only(toggle_mark(app)),
        Action::ClearMarks => only(App {
            marks: Marks {
                people: [PersonId::HOME].into(),
                places: Default::default(),
                clocks: Vec::new(),
                rule: app.marks.rule,
            },
            ..app
        }),
        Action::Group(index) => only(switch(app, Lens::Group(index), key)),
        Action::All => only(switch(app, Lens::All, key)),
        Action::Undo => match app.mode {
            Mode::Matrix(_) => only(matrix::step_back(app)),
            Mode::Normal
            | Mode::Prompt(_)
            | Mode::NewGroup(_)
            | Mode::CopyMenu
            | Mode::SortMenu
            | Mode::Help(..)
            | Mode::AddMenu(_)
            | Mode::Picker(_)
            | Mode::DriftMenu
            | Mode::Map => undo(app),
        },
        Action::Save => match app.lens {
            Lens::Group(_) => edit::save(normal(app)),
            Lens::All => only(new_group(app)),
        },
        Action::Revert => only(normal(App {
            marks: base_marks(app.roster(), app.lens),
            ..app
        })),
        Action::NewGroup => only(new_group(app)),
        Action::CloseGroup => edit::close(app),
        Action::CycleWhere => only(edit::cycle_where(app)),
        Action::Copy => {
            let text = app.say_line();
            (app, copy("y", text))
        }
        Action::CopyMenu => only(App {
            mode: Mode::CopyMenu,
            ..app
        }),
        Action::CopyAs(format) => match app.copy_as(format) {
            Some(text) => (normal(app), copy("c", text)),
            None => only(app),
        },
        Action::Lanes => only(App {
            lanes: match app.lanes {
                Lanes::Hours => Lanes::Plain,
                Lanes::Plain => Lanes::Hours,
            },
            ..app
        }),
        Action::SortMenu => only(App {
            mode: Mode::SortMenu,
            ..app
        }),
        Action::SortBy(sort) => only(normal(App { sort, ..app })),
        Action::Help => only(match app.mode.clone() {
            Mode::Help(_, back) => App { mode: *back, ..app },
            mode @ (Mode::Normal
            | Mode::Prompt(_)
            | Mode::NewGroup(_)
            | Mode::CopyMenu
            | Mode::SortMenu
            | Mode::Matrix(_)
            | Mode::AddMenu(_)
            | Mode::Picker(_)
            | Mode::DriftMenu
            | Mode::Map) => App {
                mode: Mode::Help(context(&app), Box::new(mode)),
                ..app
            },
        }),
        Action::Quit => {
            let (app, effects) = edit::flush(app, "q");
            (
                App {
                    running: false,
                    ..app
                },
                effects,
            )
        }
        Action::Type(c) => only(edit_prompt(app, Some(c))),
        Action::Erase => only(edit_prompt(app, None)),
        Action::Submit => match &app.mode {
            Mode::NewGroup(name) => {
                let name = name.clone();
                edit::save_as(normal(app), &name)
            }
            Mode::Matrix(_) => only(matrix::submit_name(app)),
            Mode::Picker(_) => picker::submit(app),
            Mode::Normal
            | Mode::Prompt(_)
            | Mode::CopyMenu
            | Mode::SortMenu
            | Mode::Help(..)
            | Mode::AddMenu(_)
            | Mode::DriftMenu
            | Mode::Map => only(submit(app)),
        },
        Action::AddMenu => only(App {
            mode: Mode::AddMenu(None),
            ..app
        }),
        Action::AddRow(place) => {
            let point = match app.mode {
                Mode::AddMenu(point) => point,
                Mode::Normal
                | Mode::Prompt(_)
                | Mode::NewGroup(_)
                | Mode::CopyMenu
                | Mode::SortMenu
                | Mode::Help(..)
                | Mode::Matrix(_)
                | Mode::Picker(_)
                | Mode::DriftMenu
                | Mode::Map => None,
            };
            only(picker::open_new(app, place, point))
        }
        Action::EditRow => only(picker::open_edit(app)),
        Action::DeleteRow => picker::delete(app),
        Action::Reload => {
            let (app, mut effects) = edit::flush(app, "x");
            effects.push(Effect::Reload);
            (app, effects)
        }
        Action::OpenEditor => {
            let (app, mut effects) = edit::flush(app, "x");
            effects.push(Effect::Edit { mouse: app.mouse });
            (app, effects)
        }
        Action::Matrix => matrix::open(app),
        Action::Cell(columns, rows) => {
            only(matrix::step(app, isize::from(columns), isize::from(rows)))
        }
        Action::ToggleCell => only(matrix::toggle(app)),
        Action::UseGroup => matrix::use_group(app),
        Action::CopyGroup => {
            let line = matrix::of(&app)
                .and_then(|m| app.roster().groups.get(m.column))
                .map(|group| matrix::group_line(&app, group));
            match line {
                Some(text) => (app, copy("y", text)),
                None => only(app),
            }
        }
        Action::AddGroup => only(matrix::start_naming(app, matrix::Naming::Add)),
        Action::RenameGroup => {
            let column = matrix::column(&app);
            only(matrix::start_naming(app, matrix::Naming::Rename(column)))
        }
        Action::MoveGroup(by) => only(matrix::move_column(app, isize::from(by))),
        Action::CloseColumn => only(matrix::close_column(app)),
        Action::FinishMatrix => matrix::finish(app, None),
        Action::CycleRangeStyle => {
            let range_style = next_style(app.range_style);
            let text = format!("range style: {}", style_name(range_style));
            only(App {
                range_style,
                style_flash: Some(Notice {
                    text,
                    raised_at: app.now,
                }),
                ..app
            })
        }
        Action::ToggleMouse => {
            let mouse = !app.mouse;
            (App { mouse, ..app }, vec![Effect::CaptureMouse(mouse)])
        }
        Action::FlipMap => only(App {
            map_flipped: !app.map_flipped,
            ..app
        }),
        Action::MapMode if map::is_drawn(&app) => only(App {
            mode: Mode::Map,
            ..app
        }),
        Action::MapMode => only(app),
        Action::Pan(across, down) => {
            let camera = map::panned(&app, (across, down));
            only(App { camera, ..app })
        }
        Action::Zoom(by) => {
            let camera = map::zoomed(&app, by);
            only(App { camera, ..app })
        }
        Action::ResetMap => {
            let camera = map::framing(&app);
            only(App { camera, ..app })
        }
    }
}

/// The cursor's slot for a move by `slots`: from exact now, forward snaps up and back snaps down.
fn moved(cursor: Cursor, now: DateTime<Utc>, slots: i64) -> Slot {
    match cursor {
        Cursor::At(slot) => slot.offset(slots),
        Cursor::Now if slots > 0 => Slot::first_after(now).offset(slots - 1),
        Cursor::Now => Slot::containing(now).offset(slots + 1),
    }
}

/// Moves the visual range's active end, else the range, else the cursor, by `slots`.
fn shift(app: App, slots: i64) -> App {
    match (app.visual, app.range) {
        (Some(v), _) => App {
            visual: Some(Visual {
                active: v.active.offset(slots),
                ..v
            }),
            ..app
        },
        (None, Some(range)) => App {
            range: Range::new(range.start().offset(slots), range.end().offset(slots)),
            cursor: Cursor::At(range.start().offset(slots)),
            ..app
        },
        (None, None) => App {
            cursor: Cursor::At(moved(app.cursor, app.now, slots)),
            ..app
        },
    }
}

fn visual(app: App) -> App {
    match app.visual {
        Some(_) => commit(app),
        None => {
            let (anchor, active) = app.range.map_or_else(
                || (app.start_slot(), app.start_slot()),
                |range| (range.start(), range.end().offset(-1)),
            );
            App {
                visual: Some(Visual {
                    anchor,
                    active,
                    before: (app.cursor, app.range),
                }),
                ..app
            }
        }
    }
}

fn commit(app: App) -> App {
    match app.visual {
        Some(v) => App {
            range: Some(v.range()),
            cursor: Cursor::At(v.range().start()),
            visual: None,
            ..app
        },
        None => app,
    }
}

/// Moves the range's start and end edges, keeping at least one slot between them.
fn edge(app: App, start: i64, end: i64) -> App {
    if app.visual.is_some() {
        return app;
    }
    let range = app.range.unwrap_or(Range::single(app.start_slot()));
    match Range::new(range.start().offset(start), range.end().offset(end)) {
        Some(range) => App {
            range: Some(range),
            cursor: Cursor::At(range.start()),
            ..app
        },
        None => app,
    }
}

fn unwind(app: App) -> App {
    if app.mode == Mode::Normal && app.map_view() == MapView::Overlay {
        return App {
            map_flipped: false,
            ..app
        };
    }
    match (app.mode.clone(), app.visual, app.range) {
        (Mode::Help(_, back), _, _) => App { mode: *back, ..app },
        (Mode::Matrix(_), _, _) => matrix::edit_name(app, |_| None),
        (
            Mode::Prompt(_)
            | Mode::NewGroup(_)
            | Mode::CopyMenu
            | Mode::SortMenu
            | Mode::AddMenu(_)
            | Mode::Picker(_)
            | Mode::DriftMenu
            | Mode::Map,
            _,
            _,
        ) => normal(app),
        (Mode::Normal, Some(v), _) => App {
            cursor: v.before.0,
            range: v.before.1,
            visual: None,
            ..app
        },
        (Mode::Normal, None, Some(range)) => App {
            cursor: Cursor::At(range.start()),
            range: None,
            ..app
        },
        (Mode::Normal, None, None) => app,
    }
}

fn focus_by(app: App, step: isize) -> App {
    let rows = app.focus_rows();
    let index = rows.iter().position(|row| *row == app.focus).unwrap_or(0);
    let target = index
        .saturating_add_signed(step)
        .min(rows.len().saturating_sub(1));
    App {
        focus: rows.get(target).copied().unwrap_or(app.focus),
        ..app
    }
}

fn toggle_mark(app: App) -> App {
    let mut marks = app.marks.clone();
    match app.focus {
        Row::Person(id) if id == PersonId::HOME => return app,
        Row::Person(id) => {
            if !marks.people.remove(&id) {
                marks.people.insert(id);
            }
        }
        Row::Place(id) => {
            if !marks.places.remove(&id) {
                marks.places.insert(id);
            }
        }
    }
    App { marks, ..app }
}

fn key_text(key: Key) -> String {
    match key {
        Key::Char(c) => c.to_string(),
        Key::Ctrl(_)
        | Key::Left
        | Key::Right
        | Key::Up
        | Key::Down
        | Key::ShiftLeft
        | Key::ShiftRight
        | Key::Enter
        | Key::Esc
        | Key::Backspace
        | Key::Tab
        | Key::Other => String::new(),
    }
}

/// Selects a lens, dropping any drift with a notice that `u` can undo.
fn switch(app: App, lens: Lens, key: Key) -> App {
    let exists = match lens {
        Lens::Group(index) => index < app.roster().groups.len(),
        Lens::All => true,
    };
    if !exists {
        return app;
    }
    let dropped = (!app.drift().is_empty()).then(|| {
        (
            app.expression(),
            Undo {
                before: Box::new(Snapshot::of(&app)),
                expires: app.now + TIMEOUT,
            },
        )
    });
    let marks = base_marks(app.roster(), lens);
    let now = app.now;
    let next = App {
        lens,
        marks,
        undo: dropped
            .as_ref()
            .map(|(_, undo)| undo.clone())
            .or(app.undo.clone()),
        ..app
    };
    match dropped {
        Some((expression, _)) => {
            let text = format!("dropped {expression}  u undo");
            with_notice(next, &key_text(key), &text, now)
        }
        None => next,
    }
}

/// Puts back the state before the last drop, write, close or delete, and the file as it was
/// then: its text, or, while a close still waited, what the file held before that close.
fn undo(app: App) -> (App, Vec<Effect>) {
    match app.undo.clone() {
        Some(undo) if app.now < undo.expires => {
            let disk = undo.before.disk.clone();
            let restored = undo.before.restore(App { undo: None, ..app });
            let now = restored.now;
            let text = format!("restored {}", restored.expression());
            let restored = with_notice(restored, "u", &text, now);
            match restored.flush_at {
                Some(_) => edit::write_back(restored, disk, "u"),
                None => edit::flush(restored, "u"),
            }
        }
        Some(_) | None => (App { undo: None, ..app }, Vec::new()),
    }
}

#[derive(Clone, Copy)]
enum Step {
    Next,
    Previous,
}

impl Step {
    /// The key that steps this way, and the word its notice uses.
    fn key_and_word(self) -> (&'static str, &'static str) {
        match self {
            Self::Next => ("n", "next"),
            Self::Previous => ("N", "previous"),
        }
    }
}

/// Why a fitting run ends at `slot`, as in `AU peak from 5:30pm`.
fn ends_because(app: &App, slot: Slot) -> Option<String> {
    let roster = app.roster();
    let from = clock_text(&slot.start().with_timezone(&roster.axis()));
    blocker(roster, &app.marks, slot).map(|row| match row {
        Row::Person(id) => format!(
            "{} {} from {from}",
            roster.person(id).name,
            person_band(roster, id, slot).word()
        ),
        Row::Place(id) => format!(
            "{} {} from {from}",
            roster.place(id).name,
            place_level(roster, id, slot)
                .map_or("closed", |level| roster.place(id).labels().word(level))
        ),
    })
}

/// `n` and `N`: the next or previous fitting run, keeping the range's length when the run allows.
fn fit_step(app: App, step: Step) -> App {
    let length = app
        .shown_range()
        .map_or(HOUR, |range| range.end().since(range.start()));
    let from = app.start_slot();
    let found = match step {
        Step::Next => next_run(app.roster(), &app.marks, from),
        Step::Previous => previous_run(app.roster(), &app.marks, from),
    };
    let now = app.now;
    match found {
        Ok(run) => landed(app, step, run, length, now),
        Err(no_fit) => missed(app, step, no_fit, now),
    }
}

fn landed(app: App, step: Step, run: Range, length: i64, now: DateTime<Utc>) -> App {
    let (key, word) = step.key_and_word();
    let run_length = run.end().since(run.start());
    let kept = length.min(run_length);
    let range = Range::new(run.start(), run.start().offset(kept)).unwrap_or(run);
    let axis = app.roster().axis();
    let (start, end) = (
        range.start().start().with_timezone(&axis),
        range.end().start().with_timezone(&axis),
    );
    let mut text = format!(
        "{word} fit for {}: {} {} {}",
        app.expression(),
        start.format("%a"),
        span_text(&start, &end),
        start.format("%Z")
    );
    if run_length < length {
        let why = ends_because(&app, run.end())
            .map(|why| format!(" ({why})"))
            .unwrap_or_default();
        text.push_str(&format!(
            ", range shortened from {} to {}{why}",
            duration_text(length),
            duration_text(run_length)
        ));
    }
    let next = App {
        range: Some(range),
        cursor: Cursor::At(range.start()),
        visual: None,
        ..app
    };
    with_notice(next, key, &text, now)
}

/// The fitting run holding `slot`, found by walking out from it a day either way.
fn run_around(app: &App, slot: Slot) -> Option<Range> {
    let fits = |slot: Slot| slot_fits(app.roster(), &app.marks, slot);
    if !fits(slot) {
        return None;
    }
    let start = (1..=DAY)
        .map(|step| slot.offset(-step))
        .take_while(|slot| fits(*slot))
        .last()
        .unwrap_or(slot);
    let end = (1..=DAY)
        .map(|step| slot.offset(step))
        .find(|slot| !fits(*slot))
        .unwrap_or(slot.offset(DAY));
    Range::new(start, end)
}

/// A click on a fitting run: the range moves to the run's start, shortened to the run if needed.
fn run_at(app: App, slot: Slot) -> App {
    let length = app
        .shown_range()
        .map_or(HOUR, |range| range.end().since(range.start()));
    let now = app.now;
    match run_around(&app, slot) {
        Some(run) => landed(app, Step::Next, run, length, now),
        None => app,
    }
}

fn missed(app: App, step: Step, no_fit: NoFit, now: DateTime<Utc>) -> App {
    let next = App {
        range: None,
        cursor: Cursor::At(no_fit.at),
        visual: None,
        ..app
    };
    with_notice(next, step.key_and_word().0, &no_fit.to_string(), now)
}

fn edit_prompt(app: App, typed: Option<char>) -> App {
    let mode = match &app.mode {
        Mode::Prompt(input) => Mode::Prompt(prompt::edit(input, typed)),
        Mode::NewGroup(input) => Mode::NewGroup(prompt::edit(input, typed)),
        Mode::Matrix(_) => {
            return matrix::edit_name(app, |input| Some(prompt::edit(input, typed)));
        }
        Mode::Picker(_) => return picker::edit_field(app, |input| prompt::edit(input, typed)),
        Mode::Normal
        | Mode::CopyMenu
        | Mode::SortMenu
        | Mode::Help(..)
        | Mode::AddMenu(_)
        | Mode::DriftMenu
        | Mode::Map => return app,
    };
    App { mode, ..app }
}

/// `+`, or `w` on `0 all`: opens the strip's name prompt, offering a name from the expression.
fn new_group(app: App) -> App {
    App {
        mode: Mode::NewGroup(edit::offered_name(&app)),
        ..app
    }
}

fn submit(app: App) -> App {
    let Mode::Prompt(input) = &app.mode else {
        return app;
    };
    let now = app.now;
    match prompt::submit(input, app.roster(), now) {
        Ok(Moment::Now) => normal(App {
            cursor: Cursor::Now,
            range: None,
            visual: None,
            ..app
        }),
        Ok(Moment::Next) => fit_step(normal(app), Step::Next),
        Ok(Moment::Fixed(When::At(instant))) => normal(App {
            cursor: Cursor::At(Slot::containing(instant)),
            range: None,
            visual: None,
            ..app
        }),
        Ok(Moment::Fixed(When::Over(range))) => normal(App {
            cursor: Cursor::At(range.start()),
            range: Some(range),
            visual: None,
            ..app
        }),
        Err(error) => with_notice(normal(app), "t", &error, now),
    }
}
