//! Reloading config, and handing the terminal to `$EDITOR` and back.
//!
//! The event loop reads keys on the same thread that runs the editor, so the reader is parked
//! for as long as the editor runs. What the terminal buffered meanwhile, and anything it sends
//! within [`GRACE`] of the editor exiting (late replies to the editor's own queries), is drained
//! before the loop reads again, so none of it reaches the update as keys.

use std::path::Path;
use std::time::Duration;

use chrono_tz::Tz;
use teamzones_core::config::Config;

use super::{Events, Io};

/// How long after the editor exits input is still treated as the editor's.
pub const GRACE: Duration = Duration::from_millis(20);

/// A config read back from its file, and the text it was parsed from (`None` with no file).
#[derive(Clone, Debug, PartialEq)]
pub struct Loaded {
    pub config: Config,
    pub text: Option<String>,
}

/// The config at `path` read afresh.
///
/// # Errors
///
/// The notice text for a file that cannot be read or does not load, saying the last good config
/// is kept.
pub fn reload(path: &Path, system_zone: Tz) -> Result<Box<Loaded>, String> {
    let kept =
        |error: &dyn std::fmt::Display| format!("config error {error}, kept the last good config");
    let text = crate::paths::read(path).map_err(|error| kept(&error))?;
    let config = Config::load(text.as_deref(), system_zone).map_err(|error| kept(&error))?;
    Ok(Box::new(Loaded { config, text }))
}

/// Discards every event already buffered and any that arrives within `grace` of the last one.
///
/// # Errors
///
/// The terminal read failing.
pub fn drain(events: &mut dyn Events, grace: Duration) -> std::io::Result<()> {
    while events.poll(grace)? {
        events.read()?;
    }
    Ok(())
}

/// Suspends the TUI, runs the editor on the config (seeding the template when there is no file),
/// takes the terminal back whatever the editor did, drains its input, then reloads.
///
/// # Errors
///
/// The notice text when the template cannot be written, the editor cannot start or fails, the
/// terminal cannot be taken back, or the edited file does not load.
pub fn hand_off(io: &mut Io, mouse: bool) -> Result<Box<Loaded>, String> {
    let path = io.config;
    if crate::paths::read(path)
        .map_err(|error| error.to_string())?
        .is_none()
    {
        crate::write::create_new(path, crate::write::TEMPLATE)
            .map_err(|error| format!("{}: {error}", path.display()))?;
    }
    let (visual, editor) = &io.editors;
    let (program, args) = crate::cli::editor_command(visual.as_deref(), editor.as_deref(), path);
    let ran = match io.modes.suspend() {
        Ok(()) => io
            .opener
            .open(&program, &args)
            .map_err(|error| format!("could not run {program}: {error}")),
        Err(error) => Err(format!("could not leave the screen: {error}")),
    };
    let resumed = io.modes.resume(mouse);
    drain(io.events, GRACE).map_err(|error| error.to_string())?;
    resumed.map_err(|error| format!("could not take the screen back: {error}"))?;
    if ran? {
        reload(path, io.system_zone)
    } else {
        Err(format!("{program} failed, so config was not reloaded"))
    }
}
