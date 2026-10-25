//! The one write path for `config.toml`: format-preserving edits that are re-parsed before an
//! atomic replace, plus the commented template a new file starts from.

use std::io::Write;
use std::path::Path;

use chrono::NaiveDate;
use chrono_tz::Tz;
use teamzones_core::config::{Config, ConfigError, Rule};
use thiserror::Error;
use toml_edit::{Array, ArrayOfTables, DocumentMut, Item, Table, Value, value};

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
    #[error("{}: changed outside teamzones since it was read, so it was left alone (r reloads it)", .0.display())]
    Changed(std::path::PathBuf),
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

/// Replaces the config at `path` with `text`, but only while the file still holds `over`, so an
/// edit made outside the app since it read the file is never written over.
///
/// # Errors
///
/// [`WriteError::Changed`] when the file no longer holds `over`, or any read or write error.
pub fn replace_if_unchanged(path: &Path, over: &str, text: &str) -> Result<(), WriteError> {
    if crate::paths::read(path)?.as_deref() != Some(over) {
        return Err(WriteError::Changed(path.to_owned()));
    }
    replace_atomically(path, text).map_err(|source| WriteError::Io {
        path: path.to_owned(),
        source,
    })
}

/// `text` after `edit`, with every byte the edit leaves alone kept as it was.
///
/// # Errors
///
/// The [`toml_edit::TomlError`] when `text` is not TOML.
pub fn edited(
    text: &str,
    edit: impl FnOnce(&mut DocumentMut),
) -> Result<String, toml_edit::TomlError> {
    let mut document: DocumentMut = text.parse()?;
    edit(&mut document);
    Ok(document.to_string())
}

/// What a group lists, by name, and its `where` when that changes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Members {
    pub people: Vec<String>,
    pub places: Vec<String>,
    pub rule: Option<Rule>,
}

fn groups_mut(document: &mut DocumentMut) -> Option<&mut ArrayOfTables> {
    document.get_mut("group")?.as_array_of_tables_mut()
}

/// Whether the list at `key` already names exactly `names`, ignoring case.
fn lists(table: &Table, key: &str, names: &[String]) -> bool {
    let listed: Vec<&str> = table
        .get(key)
        .and_then(Item::as_array)
        .map(|array| array.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    listed.len() == names.len()
        && listed
            .iter()
            .zip(names)
            .all(|(listed, name)| listed.eq_ignore_ascii_case(name))
}

/// Moves `key` to sit directly after the first of `after` the table has, or last without one.
fn place_after(table: &mut Table, key: &str, after: &[&str]) {
    let mut order: Vec<String> = table
        .iter()
        .map(|(name, _)| name.to_owned())
        .filter(|name| name != key)
        .collect();
    let at = after
        .iter()
        .find_map(|after| order.iter().position(|name| name == after))
        .map_or(order.len(), |index| index + 1);
    order.insert(at, key.to_owned());
    let rank = |name: &str| order.iter().position(|known| known == name);
    table.sort_values_by(|a, _, b, _| rank(a.get()).cmp(&rank(b.get())));
}

/// Sets `key` to `new`, keeping the decor (spacing and any comment) of the value it replaces,
/// or adding it directly after the first of `after` the table has.
fn set_value(table: &mut Table, key: &str, new: Value, after: &[&str]) {
    match table.get_mut(key).and_then(Item::as_value_mut) {
        Some(old) => {
            let decor = old.decor().clone();
            *old = new;
            *old.decor_mut() = decor;
        }
        None => {
            table.insert(key, Item::Value(new));
            place_after(table, key, after);
        }
    }
}

/// Sets the list at `key` to `names` unless it already says so; an empty list removes the key.
fn set_list(table: &mut Table, key: &str, names: &[String], after: &[&str]) {
    if lists(table, key, names) {
        return;
    }
    if names.is_empty() {
        table.remove(key);
        return;
    }
    let array: Array = names.iter().map(String::as_str).collect();
    set_value(table, key, Value::Array(array), after);
}

fn set_members(table: &mut Table, members: &Members) {
    set_list(table, "people", &members.people, &["aliases", "name"]);
    set_list(
        table,
        "places",
        &members.places,
        &["people", "aliases", "name"],
    );
    if let Some(rule) = members.rule {
        let after = ["places", "people", "aliases", "name"];
        set_value(table, "where", Value::from(rule.word()), &after);
    }
}

/// Saves the members of the group at `index`, touching only the keys whose value changes.
pub fn save_group(document: &mut DocumentMut, index: usize, members: &Members) {
    if let Some(table) = groups_mut(document).and_then(|groups| groups.get_mut(index)) {
        set_members(table, members);
    }
}

/// Removes the group at `index`, with any comment above it.
pub fn remove_group(document: &mut DocumentMut, index: usize) {
    let emptied = groups_mut(document).is_some_and(|groups| {
        if index < groups.len() {
            groups.remove(index);
        }
        groups.is_empty()
    });
    if emptied {
        document.remove("group");
    }
}

/// Adds a group after the last one, listing `members`.
pub fn push_group(document: &mut DocumentMut, name: &str, members: &Members) {
    let mut table = Table::new();
    table.decor_mut().set_prefix("\n");
    table.insert("name", value(name));
    set_members(&mut table, members);
    let groups = document
        .entry("group")
        .or_insert_with(|| Item::ArrayOfTables(ArrayOfTables::new()));
    if let Some(groups) = groups.as_array_of_tables_mut() {
        groups.push(table);
    }
}

/// Renames the group at `index` to `name`, keeping its old name as an alias so expressions and
/// scripts that use it still work.
pub fn rename_group(document: &mut DocumentMut, index: usize, name: &str) {
    let Some(table) = groups_mut(document).and_then(|groups| groups.get_mut(index)) else {
        return;
    };
    let Some(old) = table.get("name").and_then(Item::as_str).map(str::to_owned) else {
        return;
    };
    set_value(table, "name", Value::from(name), &[]);
    let aliases: Vec<String> = table
        .get("aliases")
        .and_then(Item::as_array)
        .map(|array| {
            array
                .iter()
                .filter_map(Value::as_str)
                .filter(|alias| !alias.eq_ignore_ascii_case(name))
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default();
    let known = aliases.iter().any(|alias| alias.eq_ignore_ascii_case(&old));
    let aliases: Vec<String> = aliases.into_iter().chain((!known).then_some(old)).collect();
    set_list(table, "aliases", &aliases, &["name"]);
}

/// Moves the group at `from` to `to`, each table taking the place in the file the group there
/// held, so the file's order is the new order.
pub fn move_group(document: &mut DocumentMut, from: usize, to: usize) {
    let Some(groups) = groups_mut(document) else {
        return;
    };
    if from >= groups.len() || to >= groups.len() {
        return;
    }
    let mut tables: Vec<Table> = (0..groups.len()).map(|_| groups.remove(0)).collect();
    let positions: Vec<Option<isize>> = tables.iter().map(Table::position).collect();
    let moved = tables.remove(from);
    tables.insert(to, moved);
    for (mut table, position) in tables.into_iter().zip(positions) {
        table.set_position(position);
        groups.push(table);
    }
}

/// The table of the person at `index` in config order, home being 0.
fn person_mut(document: &mut DocumentMut, index: usize) -> Option<&mut Table> {
    match index.checked_sub(1) {
        None => document.get_mut("home")?.as_table_mut(),
        Some(index) => document
            .get_mut("person")?
            .as_array_of_tables_mut()?
            .get_mut(index),
    }
}

fn place_mut(document: &mut DocumentMut, index: usize) -> Option<&mut Table> {
    document
        .get_mut("place")?
        .as_array_of_tables_mut()?
        .get_mut(index)
}

/// A row's table, by its kind and index in config order (home is person 0).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Entry {
    Person(usize),
    Place(usize),
}

impl Entry {
    fn table_mut(self, document: &mut DocumentMut) -> Option<&mut Table> {
        match self {
            Self::Person(index) => person_mut(document, index),
            Self::Place(index) => place_mut(document, index),
        }
    }

    /// The group list that names this kind of row.
    fn list(self) -> &'static str {
        match self {
            Self::Person(_) => "people",
            Self::Place(_) => "places",
        }
    }
}

/// The text of the entry's `key` as config writes it: a string as is, and `at` as `lat, lon`.
pub fn entry_value(text: &str, entry: Entry, key: &str) -> Option<String> {
    let mut document: DocumentMut = text.parse().ok()?;
    let item = entry.table_mut(&mut document)?.get(key)?.clone();
    match item.as_array() {
        Some(pair) => Some(
            pair.iter()
                .map(|n| {
                    n.as_float()
                        .map_or_else(|| n.to_string(), |f| f.to_string())
                })
                .collect::<Vec<_>>()
                .join(", "),
        ),
        None => item.as_str().map(str::to_owned),
    }
}

/// A field to write on an entry: a value, or `None` to remove the key.
pub type Field = (&'static str, Option<Value>);

fn set_fields(table: &mut Table, fields: &[Field]) {
    for (key, value) in fields {
        match value {
            Some(new) => set_value(table, key, new.clone(), &[]),
            None => {
                table.remove(key);
            }
        }
    }
}

/// Adds a `[[person]]` or `[[place]]` after the last one of its kind, holding `fields`.
pub fn push_entry(document: &mut DocumentMut, place: bool, fields: &[Field]) {
    let mut table = Table::new();
    table.decor_mut().set_prefix("\n");
    set_fields(&mut table, fields);
    let key = if place { "place" } else { "person" };
    let entries = document
        .entry(key)
        .or_insert_with(|| Item::ArrayOfTables(ArrayOfTables::new()));
    if let Some(entries) = entries.as_array_of_tables_mut() {
        entries.push(table);
    }
}

/// Writes `fields` on the entry, renaming it in every group's list when its name changes.
pub fn edit_entry(document: &mut DocumentMut, entry: Entry, old: &str, fields: &[Field]) {
    let renamed = fields
        .iter()
        .find(|(key, _)| *key == "name")
        .and_then(|(_, value)| value.as_ref()?.as_str().map(str::to_owned));
    if let Some(table) = entry.table_mut(document) {
        set_fields(table, fields);
    }
    if let Some(new) = renamed {
        rename_listed(document, entry.list(), old, Some(&new));
    }
}

/// Replaces `old` in every group's `list` with `new`, or removes it with `None`.
fn rename_listed(document: &mut DocumentMut, list: &str, old: &str, new: Option<&str>) {
    let Some(groups) = groups_mut(document) else {
        return;
    };
    for table in groups.iter_mut() {
        let Some(names) = table.get(list).and_then(Item::as_array) else {
            continue;
        };
        let names: Vec<String> = names
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect();
        if !names.iter().any(|name| name.eq_ignore_ascii_case(old)) {
            continue;
        }
        let names: Vec<String> = names
            .into_iter()
            .filter_map(|name| {
                if name.eq_ignore_ascii_case(old) {
                    new.map(str::to_owned)
                } else {
                    Some(name)
                }
            })
            .collect();
        let after = ["people", "aliases", "name"];
        set_list(table, list, &names, &after);
    }
}

/// Removes the entry and its name from every group that lists it. Home cannot be removed.
pub fn remove_entry(document: &mut DocumentMut, entry: Entry, name: &str) {
    let (key, index) = match entry {
        Entry::Person(0) => return,
        Entry::Person(index) => ("person", index - 1),
        Entry::Place(index) => ("place", index),
    };
    let emptied = document
        .get_mut(key)
        .and_then(Item::as_array_of_tables_mut)
        .is_some_and(|entries| {
            if index < entries.len() {
                entries.remove(index);
            }
            entries.is_empty()
        });
    if emptied {
        document.remove(key);
    }
    rename_listed(document, entry.list(), name, None);
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

    /// The template's `[[copy]]` tables are read on their own, each re-keyed in upper case so it
    /// joins the built-ins as a new format instead of overlaying one, which would hide a gap.
    #[test]
    fn the_uncommented_copy_tables_are_exactly_the_built_in_formats() {
        let builtin = teamzones_core::copy::Formats::builtin();
        let mut document: DocumentMut = uncommented().parse().unwrap();
        let tables = document["copy"].as_array_of_tables_mut().unwrap();
        let fields = |table: &Table| -> Vec<String> {
            table.iter().map(|(field, _)| field.to_owned()).collect()
        };
        let shape = ["key", "label", "who", "date", "names", "places"];
        for (table, format) in tables.iter_mut().zip(builtin.all()) {
            let mut expected = shape.map(str::to_owned).to_vec();
            if format.key == builtin.default_format().key {
                expected.push("default".to_owned());
            }
            assert_eq!(fields(table), expected, "{}", format.key);
            table["key"] = value(format.key.to_ascii_uppercase().to_string());
        }
        let keys: String = tables
            .iter()
            .filter_map(|table| table["key"].as_str())
            .collect();
        let builtin_keys: String = builtin.all().iter().map(|f| f.key).collect();
        assert_eq!(keys, builtin_keys.to_ascii_uppercase());
        let config = Config::parse(&document.to_string()).unwrap();
        let (kept, read) = config.copy.all().split_at(builtin.all().len());
        assert_eq!(kept, builtin.all());
        let rekeyed: Vec<_> = read
            .iter()
            .map(|format| teamzones_core::copy::Format {
                key: format.key.to_ascii_lowercase(),
                ..format.clone()
            })
            .collect();
        assert_eq!(rekeyed, builtin.all());
        assert_eq!(
            config.copy.default_format().key,
            builtin.default_format().key.to_ascii_uppercase()
        );
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
