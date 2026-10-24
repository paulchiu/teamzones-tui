//! The TUI: an Elm-style app over `teamzones-core`, drawn with ratatui.
//!
//! `update` and `view` are pure; this module is the shell that reads keys, samples the clock
//! and runs effects.

mod app;
mod footer;
mod glyphs;
mod help;
mod keys;
mod layout;
mod notice;
mod prompt;
mod say;
mod theme;
mod update;
mod view;

use std::time::Duration;

use chrono::{DateTime, Timelike, Utc};
use crossterm::event::{self, Event, KeyEventKind};
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

/// Runs one effect and returns the message carrying its result.
fn run_effect(effect: Effect, clipboard: &mut dyn Clipboard) -> Msg {
    match effect {
        Effect::Copy { key, text } => Msg::Copied {
            result: clipboard.copy(&text),
            key,
            text,
        },
    }
}

/// How long until something on screen changes by itself: a notice expiring, or the minute
/// rolling over while viewing now.
fn wake_after(app: &App, now: DateTime<Utc>) -> Duration {
    let notice = app
        .notice
        .as_ref()
        .filter(|notice| notice.visible(now))
        .map(|notice| notice.raised_at + notice::TIMEOUT - now);
    let minute = (app.cursor == Cursor::Now && app.range.is_none())
        .then(|| chrono::TimeDelta::seconds(i64::from(60 - now.second())));
    [notice, minute]
        .into_iter()
        .flatten()
        .filter_map(|delta| delta.to_std().ok())
        .min()
        .unwrap_or(IDLE)
}

/// Opens the TUI on `config` until the user quits, restoring the terminal on the way out.
///
/// # Errors
///
/// Any terminal read or write error.
pub fn run(
    config: Config,
    clock: Clock,
    clipboard: &mut dyn Clipboard,
    env: TerminalEnv,
) -> std::io::Result<()> {
    let theme = Theme::resolve(&config.theme, env);
    let look = Look {
        theme: &theme,
        glyphs: Glyphs::of(config.theme.glyphs),
    };
    let mut terminal = ratatui::init();
    let mut app = App::new(config, clock.now());
    let result = loop {
        if let Err(error) = terminal.draw(|frame| view::draw(frame, &app, look)) {
            break Err(error);
        }
        if !app.running {
            break Ok(());
        }
        let input = match event::poll(wake_after(&app, clock.now())) {
            Ok(true) => match event::read() {
                Ok(Event::Key(key)) if key.kind == KeyEventKind::Press => {
                    Some(Msg::Key(keys::Key::from_crossterm(key)))
                }
                Ok(_) => None,
                Err(error) => break Err(error),
            },
            Ok(false) => None,
            Err(error) => break Err(error),
        };
        app = update(app, Msg::Tick(clock.now())).0;
        if let Some(msg) = input {
            let (next, effects) = update(app, msg);
            app = effects.into_iter().fold(next, |app, effect| {
                update(app, run_effect(effect, clipboard)).0
            });
        }
    };
    ratatui::restore();
    result
}

#[cfg(test)]
mod tests;
