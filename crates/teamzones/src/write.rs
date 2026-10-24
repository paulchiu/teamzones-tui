//! The one write path for `config.toml`: format-preserving edits that are re-parsed before an
//! atomic replace, plus the commented template a new file starts from.

use std::io::Write;
use std::path::Path;

use chrono::NaiveDate;
use chrono_tz::Tz;
use teamzones_core::config::{Config, ConfigError};
use thiserror::Error;
use toml_edit::{DocumentMut, Item, Table, value};

/// Why an edit to the config was refused; the Display names the path where there is one.
#[derive(Debug, Error)]
pub enum WriteError {
    #[error("{}: no config file to edit, so run teamzones config init and add a [home] first", .0.display())]
    Absent(std::path::PathBuf),
    #[error(transparent)]
    Load(#[from] crate::paths::LoadError),
    #[error("{}: {source}", .path.display())]
    Toml {
        path: std::path::PathBuf,
        source: toml_edit::TomlError,
    },
    #[error("{}: the edited file would not load, so it was left alone: {source}", .path.display())]
    Rejected {
        path: std::path::PathBuf,
        source: ConfigError,
    },
    #[error("{}: {source}", .path.display())]
    Io {
        path: std::path::PathBuf,
        source: std::io::Error,
    },
}

/// The commented template `config init` and `config edit` seed a new file with.
pub const TEMPLATE: &str = include_str!("template.toml");

/// Writes `text` to a new file at `path`, creating its directory, and never over an existing file.
///
/// # Errors
///
/// [`std::io::ErrorKind::AlreadyExists`] when there is a file at `path`, or any other I/O error.
pub fn create_new(path: &Path, text: &str) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?
        .write_all(text.as_bytes())
}

/// `until` as a TOML local date.
///
/// # Errors
///
/// A [`toml_edit::DatetimeParseError`] for a year TOML cannot write, such as one past 9999.
pub fn toml_date(until: NaiveDate) -> Result<toml_edit::Datetime, toml_edit::DatetimeParseError> {
    until.to_string().parse()
}

/// Sets `[travel]` to `here` until `until`, in place if the table exists, else appended after a
/// blank line.
pub fn set_travel(document: &mut DocumentMut, here: &str, until: toml_edit::Datetime) {
    let travel = document.entry("travel").or_insert_with(|| {
        let mut table = Table::new();
        table.decor_mut().set_prefix("\n");
        Item::Table(table)
    });
    travel["here"] = value(here);
    travel["until"] = value(until);
}

/// Removes `[travel]`, if there is one.
pub fn clear_travel(document: &mut DocumentMut) {
    document.remove("travel");
}

/// Applies `edit` to the config at `path`, keeping every byte it does not touch, then re-parses the
/// result and replaces the file atomically. A file that is absent, unreadable or invalid is never
/// written.
///
/// # Errors
///
/// A [`WriteError`] when the file is absent, cannot be read or loaded, or the result would not load.
pub fn edit_config(
    path: &Path,
    home_zone: Tz,
    edit: impl FnOnce(&mut DocumentMut),
) -> Result<Config, WriteError> {
    let text = crate::paths::read(path)?.ok_or_else(|| WriteError::Absent(path.to_owned()))?;
    Config::load(Some(&text), home_zone).map_err(|source| crate::paths::LoadError::Invalid {
        path: path.to_owned(),
        source,
    })?;
    let mut document: DocumentMut = text.parse().map_err(|source| WriteError::Toml {
        path: path.to_owned(),
        source,
    })?;
    edit(&mut document);
    let edited = document.to_string();
    let config = Config::load(Some(&edited), home_zone).map_err(|source| WriteError::Rejected {
        path: path.to_owned(),
        source,
    })?;
    replace_atomically(path, &edited).map_err(|source| WriteError::Io {
        path: path.to_owned(),
        source,
    })?;
    Ok(config)
}

/// Writes `text` to a temporary file beside the file `path` resolves to, with that file's
/// permissions, then renames it over the file, so a symlinked config stays a symlink.
fn replace_atomically(path: &Path, text: &str) -> std::io::Result<()> {
    let target = std::fs::canonicalize(path)?;
    let permissions = std::fs::metadata(&target)?.permissions();
    let mut temporary = target.as_os_str().to_owned();
    temporary.push(format!(".{}.tmp", std::process::id()));
    let temporary = std::path::PathBuf::from(temporary);
    std::fs::write(&temporary, text)
        .and_then(|()| std::fs::set_permissions(&temporary, permissions))
        .and_then(|()| std::fs::rename(&temporary, &target))
        .inspect_err(|_| {
            let _ = std::fs::remove_file(&temporary);
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use teamzones_core::config::Config;

    const MELBOURNE: chrono_tz::Tz = chrono_tz::Australia::Melbourne;

    #[test]
    fn the_template_as_written_is_the_same_config_as_no_file() {
        assert_eq!(
            Config::load(Some(TEMPLATE), MELBOURNE).unwrap(),
            Config::load(None, MELBOURNE).unwrap()
        );
    }

    /// Strips `# ` from every key line and leaves `## ` prose alone.
    fn uncommented() -> String {
        TEMPLATE
            .lines()
            .map(|line| {
                if line.starts_with("##") {
                    line
                } else {
                    line.strip_prefix("# ").unwrap_or(line)
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn uncommenting_every_key_gives_a_valid_config_with_the_defaults_it_names() {
        let config = Config::parse(&uncommented()).unwrap();
        let defaults = Config::load(None, MELBOURNE).unwrap();
        assert_eq!(config.roster.daylight, defaults.roster.daylight);
        assert_eq!(config.map, defaults.map);
        assert_eq!(config.theme, defaults.theme);
    }

    #[test]
    fn the_uncommented_home_days_and_group_where_are_the_defaults_they_name() {
        let full = uncommented();
        let without_days = full.replacen("days = \"Mon-Fri\"\n", "", 1);
        let without = without_days.replacen("where = \"open\"\n", "", 1);
        assert!(
            full.find("days =") < full.find("[[person]]"),
            "home's days goes first"
        );
        assert!(
            without_days != full && without != without_days,
            "both lines were removed"
        );
        assert_eq!(
            Config::parse(&full).unwrap(),
            Config::parse(&without).unwrap()
        );
    }

    #[test]
    fn uncommenting_every_key_leaves_no_line_still_commented_but_prose() {
        let leftover: Vec<String> = uncommented()
            .lines()
            .filter(|line| line.starts_with('#') && !line.starts_with("##"))
            .map(str::to_owned)
            .collect();
        assert!(leftover.is_empty(), "{leftover:?}");
    }

    /// A fresh directory holding a copy of the fixture at `config.toml`.
    fn fixture_dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir()
            .join("teamzones-write-tests")
            .join(format!("{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let fixture = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../teamzones-core/tests/fixtures/sample.toml"
        );
        std::fs::copy(fixture, dir.join("config.toml")).unwrap();
        dir
    }

    fn go_to_london(document: &mut DocumentMut) {
        set_travel(
            document,
            "London",
            toml_date(NaiveDate::from_ymd_opt(2026, 11, 1).unwrap()).unwrap(),
        );
    }

    #[cfg(unix)]
    #[test]
    fn an_edit_through_a_symlink_writes_the_target_and_keeps_the_link() {
        let dir = fixture_dir("symlink");
        let link = dir.join("link.toml");
        std::os::unix::fs::symlink(dir.join("config.toml"), &link).unwrap();
        edit_config(&link, MELBOURNE, go_to_london).unwrap();
        assert!(
            std::fs::symlink_metadata(&link)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        let target = std::fs::read_to_string(dir.join("config.toml")).unwrap();
        assert!(target.contains("[travel]"), "{target}");
    }

    #[cfg(unix)]
    #[test]
    fn an_edit_keeps_the_file_mode() {
        use std::os::unix::fs::PermissionsExt;
        let path = fixture_dir("mode").join("config.toml");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        edit_config(&path, MELBOURNE, go_to_london).unwrap();
        let mode = std::fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
    }

    #[test]
    fn a_date_toml_can_hold_converts_and_one_it_cannot_is_refused() {
        let date = toml_date(NaiveDate::from_ymd_opt(2026, 11, 1).unwrap()).unwrap();
        assert_eq!(date.to_string(), "2026-11-01");
        assert!(toml_date(NaiveDate::from_ymd_opt(70_000, 1, 1).unwrap()).is_err());
    }

    #[test]
    fn setting_travel_again_rewrites_the_table_where_it_already_is() {
        let text = "[travel]\nhere = \"London\" # trip\nuntil = 2026-11-01\n\n[theme]\nscheme = \"dark\"\n";
        let mut document: DocumentMut = text.parse().unwrap();
        set_travel(
            &mut document,
            "Paris",
            toml_date(NaiveDate::from_ymd_opt(2026, 12, 5).unwrap()).unwrap(),
        );
        assert_eq!(
            document.to_string(),
            "[travel]\nhere = \"Paris\"\nuntil = 2026-12-05\n\n[theme]\nscheme = \"dark\"\n"
        );
    }
}
