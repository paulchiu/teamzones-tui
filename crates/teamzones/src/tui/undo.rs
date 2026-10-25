//! The one level of undo: the whole state an action replaced, kept as a value rather than a diff.

use chrono::{DateTime, Utc};
use teamzones_core::config::Config;
use teamzones_core::question::{Marks, Row};

use super::app::{App, Lens};

/// What an undo puts back: the config and the text it came from, any write still pending, and
/// the lens, marks and focus on screen.
#[derive(Clone, Debug, PartialEq)]
pub struct Snapshot {
    pub config: Config,
    pub text: Option<String>,
    pub flush_at: Option<DateTime<Utc>>,
    pub lens: Lens,
    pub marks: Marks,
    pub focus: Row,
}

impl Snapshot {
    pub fn of(app: &App) -> Self {
        Self {
            config: app.config.clone(),
            text: app.text.clone(),
            flush_at: app.flush_at,
            lens: app.lens,
            marks: app.marks.clone(),
            focus: app.focus,
        }
    }

    /// `app` with this state put back; the file on disk is the caller's to bring in line.
    pub fn restore(self, app: App) -> App {
        let Self {
            config,
            text,
            flush_at,
            lens,
            marks,
            focus,
        } = self;
        App {
            config,
            text,
            flush_at,
            lens,
            marks,
            focus,
            ..app
        }
    }
}

/// The one level of undo: the state before the last drop, write, close or delete, until it
/// expires 10 s after the action.
#[derive(Clone, Debug, PartialEq)]
pub struct Undo {
    pub before: Box<Snapshot>,
    pub expires: DateTime<Utc>,
}
