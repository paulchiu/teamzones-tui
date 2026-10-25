//! The state file: what the TUI remembers between runs, read strictly and written atomically.

use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use etcetera::BaseStrategy;
use thiserror::Error;
use toml_edit::DocumentMut;

/// Why the state file could not be found, read or parsed; the Display names the path.
#[derive(Debug, Error)]
pub enum StateError {
    #[error("no home directory to keep state in: {0}")]
    NoHome(String),
    #[error("{}: {source}", .path.display())]
    Unreadable {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("{}: {reason}", .path.display())]
    Invalid { path: PathBuf, reason: String },
}

/// What the state file held when the TUI started: its text, and the last group it names.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct State {
    pub text: Option<String>,
    pub last_group: Option<String>,
}

/// `$XDG_STATE_HOME/teamzones/state.toml`, else `~/.local/state/teamzones/state.toml`.
///
/// # Errors
///
/// [`StateError::NoHome`] when no home directory is available.
pub fn path() -> Result<PathBuf, StateError> {
    etcetera::choose_base_strategy()
        .map_err(|error| StateError::NoHome(error.to_string()))
        .and_then(|base| {
            base.state_dir()
                .ok_or_else(|| StateError::NoHome("no state directory".to_owned()))
        })
        .map(|dir| dir.join("teamzones").join("state.toml"))
}

/// The state at `path`: empty when there is no file.
///
/// # Errors
///
/// [`StateError::Unreadable`] or [`StateError::Invalid`] when a file is there but cannot be read
/// or parsed; it never falls back to empty.
pub fn load(path: &Path) -> Result<State, StateError> {
    match std::fs::read_to_string(path) {
        Ok(text) => parse(path, text),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(State::default()),
        Err(source) => Err(StateError::Unreadable {
            path: path.to_owned(),
            source,
        }),
    }
}

/// The state `text`, read from `path`, holds.
fn parse(path: &Path, text: String) -> Result<State, StateError> {
    let invalid = |reason: String| StateError::Invalid {
        path: path.to_owned(),
        reason,
    };
    let document: DocumentMut = text.parse().map_err(|error| invalid(format!("{error}")))?;
    let last_group = match document.get("last_group") {
        None => None,
        Some(item) => Some(
            item.as_str()
                .ok_or_else(|| invalid("last_group must be a string".to_owned()))?
                .to_owned(),
        ),
    };
    Ok(State {
        text: Some(text),
        last_group,
    })
}

/// The state file's text with `last_group` set to `group`, keeping every other key and comment.
///
/// # Errors
///
/// The text not parsing, which [`load`] has already ruled out for text it returned.
pub fn with_last_group(text: Option<&str>, group: &str) -> Result<String, toml_edit::TomlError> {
    let mut document: DocumentMut = text.unwrap_or_default().parse()?;
    document["last_group"] = toml_edit::value(group);
    Ok(document.to_string())
}

/// Writes `text` to `path` through a temporary file renamed over it, making the directory first.
///
/// # Errors
///
/// Any directory, write or rename error.
pub fn write(path: &Path, text: &str) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut temporary = path.as_os_str().to_owned();
    temporary.push(format!(".{}.tmp", std::process::id()));
    let temporary = PathBuf::from(temporary);
    std::fs::write(&temporary, text)
        .and_then(|()| std::fs::rename(&temporary, path))
        .inspect_err(|_| {
            let _ = std::fs::remove_file(&temporary);
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("teamzones-state-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_missing_state_file_is_empty_state() {
        let dir = scratch("missing");
        assert_eq!(load(&dir.join("state.toml")).unwrap(), State::default());
    }

    #[test]
    fn the_last_group_comes_back_as_written_and_other_keys_survive() {
        let dir = scratch("round-trip");
        let path = dir.join("nested").join("state.toml");
        let text = with_last_group(Some("# kept\nmap = \"hidden\"\n"), "deploy").unwrap();
        write(&path, &text).unwrap();
        let state = load(&path).unwrap();
        assert_eq!(state.last_group.as_deref(), Some("deploy"));
        assert_eq!(
            state.text.as_deref(),
            Some("# kept\nmap = \"hidden\"\nlast_group = \"deploy\"\n")
        );
        let again = with_last_group(state.text.as_deref(), "team").unwrap();
        assert_eq!(again, "# kept\nmap = \"hidden\"\nlast_group = \"team\"\n");
    }

    #[test]
    fn an_unreadable_or_unparsable_state_file_is_an_error_naming_the_path() {
        let dir = scratch("unreadable");
        let unreadable = load(&dir).unwrap_err().to_string();
        assert!(
            unreadable.starts_with(&format!("{}: ", dir.display())),
            "{unreadable}"
        );
        let bad = dir.join("state.toml");
        for text in ["last_group = ", "last_group = 4\n"] {
            std::fs::write(&bad, text).unwrap();
            let error = load(&bad).unwrap_err().to_string();
            assert!(
                error.starts_with(&format!("{}: ", bad.display())),
                "{error}"
            );
        }
    }
}
