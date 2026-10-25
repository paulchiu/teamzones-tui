//! The pure update: a message in, the next state and the effects to run out.

use chrono::{DateTime, Utc};
use teamzones_core::clock::{Range, Slot, When};
use teamzones_core::config::{Config, PersonId};
use teamzones_core::copy::{clock_text, span_text};
use teamzones_core::fit::{NoFit, blocker, next_run, previous_run};
use teamzones_core::question::{Marks, Row, person_band, place_level};
use teamzones_core::time::Moment;

use super::app::{
    App, Cursor, Lanes, Lens, Mode, Undo, Visual, base_marks, next_style, style_name,
};
use super::keys::{Action, Context, Key, action_for};
use super::notice::{Notice, TIMEOUT};
use super::prompt;
use super::say::duration_text;

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
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "the r key that sends it lands with write-back")
    )]
    Reload(Box<Config>),
}

/// Work the effects layer does on the update's behalf.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Effect {
    Copy { key: String, text: String },
}

/// The context whose bindings read the next key: a modal, range mode while drawing, or main.
pub fn context(app: &App) -> Context {
    match &app.mode {
        Mode::Normal if app.visual.is_some() => Context::Range,
        Mode::Normal => Context::Main,
        Mode::Prompt(_) => Context::Prompt,
        Mode::CopyMenu => Context::CopyMenu,
        Mode::SortMenu => Context::SortMenu,
        Mode::Help(_) => Context::Help,
    }
}

/// Applies `msg`, then re-clamps focus onto a row that is still there.
pub fn update(app: App, msg: Msg) -> (App, Vec<Effect>) {
    let rows_before = app.focus_rows();
    let (mut next, effects) = match msg {
        Msg::Key(key) => on_key(app, key),
        Msg::Tick(now) => (tick(app, now), Vec::new()),
        Msg::Copied { key, text, result } => {
            let words = match result {
                Ok(()) => "copied".to_owned(),
                Err(_) => format!("clipboard unavailable: {text}"),
            };
            let now = app.now;
            (with_notice(app, &key, &words, now), Vec::new())
        }
        Msg::Reload(config) => (
            App {
                config: *config,
                ..app
            },
            Vec::new(),
        ),
    };
    next.focus = clamp_focus(&rows_before, &next.focus_rows(), next.focus);
    (next, effects)
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

fn tick(app: App, now: DateTime<Utc>) -> App {
    let notice = app.notice.filter(|notice| notice.visible(now));
    let style_flash = app.style_flash.filter(|flash| flash.visible(now));
    App {
        now,
        notice,
        style_flash,
        ..app
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
        Action::Undo => only(undo(app)),
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
        Action::Help => only(App {
            mode: match &app.mode {
                Mode::Help(_) => Mode::Normal,
                Mode::Normal | Mode::Prompt(_) | Mode::CopyMenu | Mode::SortMenu => {
                    Mode::Help(context(&app))
                }
            },
            ..app
        }),
        Action::Quit => only(App {
            running: false,
            ..app
        }),
        Action::Type(c) => only(edit_prompt(app, Some(c))),
        Action::Erase => only(edit_prompt(app, None)),
        Action::Submit => only(submit(app)),
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
    match (&app.mode, app.visual, app.range) {
        (Mode::Prompt(_) | Mode::CopyMenu | Mode::SortMenu | Mode::Help(_), _, _) => normal(app),
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
                lens: app.lens,
                marks: app.marks.clone(),
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

fn undo(app: App) -> App {
    match app.undo.clone() {
        Some(undo) if app.now < undo.expires => {
            let restored = App {
                lens: undo.lens,
                marks: undo.marks,
                undo: None,
                ..app
            };
            let now = restored.now;
            let text = format!("restored {}", restored.expression());
            with_notice(restored, "u", &text, now)
        }
        Some(_) | None => App { undo: None, ..app },
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
    match &app.mode {
        Mode::Prompt(input) => App {
            mode: Mode::Prompt(prompt::edit(input, typed)),
            ..app
        },
        Mode::Normal | Mode::CopyMenu | Mode::SortMenu | Mode::Help(_) => app,
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
