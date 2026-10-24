//! The one write path for `config.toml`: format-preserving edits that are re-parsed before an
//! atomic replace, plus the commented template a new file starts from.

use std::io::Write;
use std::path::Path;

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
    fn uncommenting_every_key_leaves_no_line_still_commented_but_prose() {
        let leftover: Vec<String> = uncommented()
            .lines()
            .filter(|line| line.starts_with('#') && !line.starts_with("##"))
            .map(str::to_owned)
            .collect();
        assert!(leftover.is_empty(), "{leftover:?}");
    }
}
