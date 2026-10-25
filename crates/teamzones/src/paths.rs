//! Where config lives, the system zone, and reading config so that absent never passes for unreadable.

use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use chrono_tz::Tz;
use etcetera::BaseStrategy;
use teamzones_core::config::{Config, ConfigError};
use thiserror::Error;

/// Why config could not be found or loaded; the Display names the path.
#[derive(Debug, Error)]
pub enum LoadError {
    #[error("no home directory to find config in: {0}")]
    NoHome(String),
    #[error("{}: {source}", .path.display())]
    Unreadable {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("{}: {source}", .path.display())]
    Invalid { path: PathBuf, source: ConfigError },
}

/// `TEAMZONES_CONFIG`, else `$XDG_CONFIG_HOME/teamzones/config.toml`, else `~/.config/teamzones/config.toml`.
///
/// # Errors
///
/// [`LoadError::NoHome`] when neither the override nor a home directory is available.
pub fn config_path() -> Result<PathBuf, LoadError> {
    match std::env::var_os("TEAMZONES_CONFIG") {
        Some(path) => Ok(PathBuf::from(path)),
        None => etcetera::choose_base_strategy()
            .map(|base| base.config_dir().join("teamzones").join("config.toml"))
            .map_err(|error| LoadError::NoHome(error.to_string())),
    }
}

/// The system's IANA zone, or UTC when it cannot be read or is unknown to chrono-tz.
pub fn system_zone() -> Tz {
    iana_time_zone::get_timezone()
        .ok()
        .and_then(|name| name.parse().ok())
        .unwrap_or(Tz::UTC)
}

/// The file's text, or `None` when there is no file at `path`.
///
/// # Errors
///
/// [`LoadError::Unreadable`] when a file is there but cannot be read.
pub fn read(path: &Path) -> Result<Option<String>, LoadError> {
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(Some(text)),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(source) => Err(LoadError::Unreadable {
            path: path.to_owned(),
            source,
        }),
    }
}

/// The config at `path`: the defaults with home in `system_zone` when there is no file.
///
/// # Errors
///
/// [`LoadError::Unreadable`] or [`LoadError::Invalid`] when a file is there but cannot be read or
/// parsed; it never falls back to the defaults.
pub fn load(path: &Path, system_zone: Tz) -> Result<Config, LoadError> {
    load_text(path, read(path)?.as_deref(), system_zone)
}

/// The config `text` read from `path` holds, or the defaults with home in `system_zone` for no file.
///
/// # Errors
///
/// [`LoadError::Invalid`], naming `path`, when the text does not parse.
pub fn load_text(path: &Path, text: Option<&str>, system_zone: Tz) -> Result<Config, LoadError> {
    Config::load(text, system_zone).map_err(|source| LoadError::Invalid {
        path: path.to_owned(),
        source,
    })
}
