//! The TUI: an Elm-style app over `teamzones-core`, drawn with ratatui.
//!
//! `update` and `view` are pure; this module is the shell that reads keys, samples the clock
//! and runs effects.

mod app;
mod clocks;
mod edit;
mod editor;
mod fold;
mod footer;
mod glyphs;
mod help;
mod keys;
mod layout;
mod map;
mod matrix;
mod mouse;
mod notice;
mod picker;
mod prompt;
mod say;
mod theme;
mod undo;
mod update;
mod view;

use std::path::Path;
use std::time::Duration;

use chrono::{DateTime, Timelike, Utc};
use chrono_tz::Tz;
use crossterm::event::{self, Event, KeyEventKind};
use ratatui::Terminal;
use ratatui::backend::Backend;
use teamzones_core::clock::Clock;
use teamzones_core::config::Config;

use crate::clipboard::Clipboard;
use app::{App, Cursor};
use glyphs::Glyphs;
pub use theme::TerminalEnv;
use theme::Theme;
use update::{Effect, Msg, update};
use view::Look;

/// The longest the loop sleeps when nothing on screen can change by itself.
const IDLE: Duration = Duration::from_secs(3600);

/// The terminal modes the app switches while it runs, behind a seam so tests can watch them.
pub trait Modes {
    /// Turns mouse capture on or off.
    ///
    /// # Errors
    ///
    /// The terminal write failing.
    fn capture_mouse(&mut self, on: bool) -> std::io::Result<()>;

    /// Hands the terminal back for another program: releases the mouse, raw mode and the
    /// alternate screen.
    ///
    /// # Errors
    ///
    /// The terminal write failing.
    fn suspend(&mut self) -> std::io::Result<()>;

    /// Takes the terminal again after [`Modes::suspend`], recapturing the mouse when `mouse`.
    ///
    /// # Errors
    ///
    /// The terminal write failing.
    fn resume(&mut self, mouse: bool) -> std::io::Result<()>;
}

/// Where terminal events come from, behind a seam so tests can feed them.
pub trait Events {
    /// Whether an event is ready within `timeout`.
    ///
    /// # Errors
    ///
    /// The terminal read failing.
    fn poll(&mut self, timeout: Duration) -> std::io::Result<bool>;

    /// The next event, waiting for one.
    ///
    /// # Errors
    ///
    /// The terminal read failing.
    fn read(&mut self) -> std::io::Result<Event>;
}

/// Runs a program in the foreground, behind a seam so tests never start an editor.
pub trait Opener {
    /// Runs `program` with `args` until it exits, returning whether it exited successfully.
    ///
    /// # Errors
    ///
    /// The program failing to start.
    fn open(&mut self, program: &str, args: &[String]) -> std::io::Result<bool>;
}

/// What effects reach: the clipboard, the terminal, the config file and the editor.
pub struct Io<'a> {
    pub clipboard: &'a mut dyn Clipboard,
    pub modes: &'a mut dyn Modes,
    pub events: &'a mut dyn Events,
    pub opener: &'a mut dyn Opener,
    pub config: &'a Path,
    /// The zone home takes when there is no config file.
    pub system_zone: Tz,
    /// `$VISUAL` and `$EDITOR`, read once at start.
    pub editors: (Option<String>, Option<String>),
    /// Set once another program has drawn on the terminal, so the next frame is drawn whole.
    pub repaint: bool,
}

/// Runs one effect and returns the message carrying its result.
fn run_effect(effect: Effect, io: &mut Io) -> Msg {
    match effect {
        Effect::CaptureMouse(on) => Msg::Captured {
            on,
            result: io
                .modes
                .capture_mouse(on)
                .map_err(|error| error.to_string()),
        },
        Effect::Copy { key, text } => Msg::Copied {
            result: io.clipboard.copy(&text),
            key,
            text,
        },
        Effect::Write { key, text, over } => Msg::Wrote {
            result: crate::write::replace_if_unchanged(io.config, &over, &text)
                .map_err(|error| error.to_string()),
            key,
            over,
        },
        Effect::Reload => Msg::Reloaded {
            key: "r".to_owned(),
            result: editor::reload(io.config, io.system_zone),
        },
        Effect::Edit { mouse } => {
            io.repaint = true;
            Msg::Reloaded {
                key: "E".to_owned(),
                result: editor::hand_off(io, mouse),
            }
        }
    }
}

/// Runs `effects` in order, feeding each result back through the update.
fn run_effects(app: App, effects: Vec<Effect>, io: &mut Io) -> App {
    effects.into_iter().fold(app, |app, effect| {
        let (next, more) = update(app, run_effect(effect, io));
        run_effects(next, more, io)
    })
}

/// How long until something on screen changes by itself: a notice or the footer's range style
/// expiring, a pending close falling due, or the minute rolling over while viewing now.
fn wake_after(app: &App, now: DateTime<Utc>) -> Duration {
    let shown = |notice: &Option<notice::Notice>| {
        notice
            .as_ref()
            .filter(|notice| notice.visible(now))
            .map(|notice| notice.raised_at + notice::TIMEOUT - now)
    };
    let (notice, flash) = (shown(&app.notice), shown(&app.style_flash));
    let minute = (app.cursor == Cursor::Now && app.range.is_none())
        .then(|| chrono::TimeDelta::seconds(i64::from(60 - now.second())));
    let flush = app
        .flush_at
        .map(|due| (due - now).max(chrono::TimeDelta::zero()));
    [notice, flash, minute, flush]
        .into_iter()
        .flatten()
        .filter_map(|delta| delta.to_std().ok())
        .min()
        .unwrap_or(IDLE)
}

/// The message a terminal event stands for, if any, read `at` on the loop's monotonic clock.
fn msg_of(event: &Event, at: chrono::TimeDelta) -> Option<Msg> {
    match event {
        Event::Key(key) if key.kind == KeyEventKind::Press => {
            Some(Msg::Key(keys::Key::from_crossterm(*key)))
        }
        Event::Resize(cols, rows) => Some(Msg::Resize(*cols, *rows)),
        Event::Mouse(event) => Some(Msg::Mouse(mouse::Pointer::from_crossterm(*event, at))),
        Event::Key(_) | Event::FocusGained | Event::FocusLost | Event::Paste(_) => None,
    }
}

fn terminal_error(error: impl std::error::Error + Send + Sync + 'static) -> std::io::Error {
    std::io::Error::other(error)
}

/// Draws, reads one event and applies it, until the app stops running; returns the last state.
///
/// # Errors
///
/// Any terminal read or write error.
fn event_loop<B: Backend>(
    terminal: &mut Terminal<B>,
    mut app: App,
    clock: &Clock,
    env: TerminalEnv,
    io: &mut Io,
) -> std::io::Result<App>
where
    B::Error: Send + Sync + 'static,
{
    let started = std::time::Instant::now();
    loop {
        if std::mem::take(&mut io.repaint) {
            terminal.clear().map_err(terminal_error)?;
        }
        let theme = Theme::resolve(&app.config.theme, env);
        let look = Look {
            theme: &theme,
            glyphs: Glyphs::of(app.config.theme.glyphs),
        };
        terminal
            .draw(|frame| view::draw(frame, &app, look))
            .map_err(terminal_error)?;
        if !app.running {
            return Ok(app);
        }
        panic_when_asked();
        let input = if io.events.poll(wake_after(&app, clock.now()))? {
            let at =
                chrono::TimeDelta::from_std(started.elapsed()).unwrap_or(chrono::TimeDelta::MAX);
            msg_of(&io.events.read()?, at)
        } else {
            None
        };
        let (ticked, effects) = update(app, Msg::Tick(clock.now()));
        app = run_effects(ticked, effects, io);
        if let Some(msg) = input {
            let (next, effects) = update(app, msg);
            app = run_effects(next, effects, io);
        }
    }
}

/// The config file the TUI edits, and what it held when the TUI started.
pub struct File<'a> {
    pub path: &'a Path,
    pub text: Option<String>,
    pub system_zone: Tz,
}

/// Opens the TUI on `config` until the user quits, restoring the terminal on the way out; starts
/// on the group the state names, with the marks it kept, and returns the group and marks to
/// remember.
///
/// # Errors
///
/// Any terminal read or write error.
pub fn run(
    config: Config,
    file: File,
    state: &crate::state::State,
    clock: Clock,
    clipboard: &mut dyn Clipboard,
    env: TerminalEnv,
) -> std::io::Result<(String, Vec<crate::state::LensMarks>)> {
    let mut terminal = ratatui::try_init().inspect_err(|_| restore())?;
    hook_restore_on_panic();
    let mut modes = Stdout;
    let app = App::new(config, clock.now())
        .with_source(file.text)
        .on_group(state.last_group.as_deref(), &state.marks);
    if let Err(error) = modes.capture_mouse(app.mouse) {
        drop(terminal);
        restore();
        return Err(error);
    }
    let app = match terminal.size() {
        Ok(size) => update(app, Msg::Resize(size.width, size.height)).0,
        Err(_) => app,
    };
    let mut io = Io {
        clipboard,
        modes: &mut modes,
        events: &mut Crossterm,
        opener: &mut Foreground,
        config: file.path,
        system_zone: file.system_zone,
        editors: (std::env::var("VISUAL").ok(), std::env::var("EDITOR").ok()),
        repaint: false,
    };
    let result = event_loop(&mut terminal, app, &clock, env, &mut io)
        .map(|app| (app.lens_name(), app.remembered()));
    drop(terminal);
    restore();
    result
}

/// The real terminal: modes switch by writing crossterm commands to stdout.
struct Stdout;

impl Modes for Stdout {
    fn capture_mouse(&mut self, on: bool) -> std::io::Result<()> {
        let mut out = std::io::stdout();
        if on {
            crossterm::execute!(out, crossterm::event::EnableMouseCapture)
        } else {
            crossterm::execute!(out, crossterm::event::DisableMouseCapture)
        }
    }

    fn suspend(&mut self) -> std::io::Result<()> {
        self.capture_mouse(false)?;
        crossterm::terminal::disable_raw_mode()?;
        crossterm::execute!(
            std::io::stdout(),
            crossterm::terminal::LeaveAlternateScreen,
            crossterm::cursor::Show
        )
    }

    fn resume(&mut self, mouse: bool) -> std::io::Result<()> {
        crossterm::terminal::enable_raw_mode()?;
        crossterm::execute!(std::io::stdout(), crossterm::terminal::EnterAlternateScreen)?;
        self.capture_mouse(mouse)
    }
}

/// The real terminal's events, read through crossterm.
struct Crossterm;

impl Events for Crossterm {
    fn poll(&mut self, timeout: Duration) -> std::io::Result<bool> {
        event::poll(timeout)
    }

    fn read(&mut self) -> std::io::Result<Event> {
        event::read()
    }
}

/// Runs a program attached to this terminal.
struct Foreground;

impl Opener for Foreground {
    fn open(&mut self, program: &str, args: &[String]) -> std::io::Result<bool> {
        std::process::Command::new(program)
            .args(args)
            .status()
            .map(|status| status.success())
    }
}

/// Panics once the screen is up when `TEAMZONES_TEST_PANIC` is set, so the pty test can watch the
/// panic hook restore the terminal; debug builds only, so a release binary never reads it.
#[cfg(debug_assertions)]
fn panic_when_asked() {
    #[expect(clippy::panic, reason = "the panic is the test hook's whole purpose")]
    if std::env::var_os("TEAMZONES_TEST_PANIC").is_some() {
        panic!("TEAMZONES_TEST_PANIC is set");
    }
}

#[cfg(not(debug_assertions))]
fn panic_when_asked() {}

/// Releases everything the TUI enables: mouse capture, then raw mode and the alternate screen.
fn restore() {
    let _ = Stdout.capture_mouse(false);
    ratatui::restore();
}

/// Chains a restore onto the panic hook ratatui installed, so a panic releases the mouse too.
fn hook_restore_on_panic() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        restore();
        previous(info);
    }));
}

#[cfg(test)]
mod tests;
