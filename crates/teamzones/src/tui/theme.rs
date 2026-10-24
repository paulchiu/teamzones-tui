//! Semantic colour roles, resolved once from `[theme]` and the terminal's environment.

use std::collections::BTreeMap;

use ratatui::style::{Color, Modifier, Style};
use teamzones_core::config::{self, Scheme};

/// A semantic role: what a colour means, never which colour it is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Text,
    Dim,
    Accent,
    Marker,
    Ok,
    Warn,
    #[expect(
        dead_code,
        reason = "a theme role with no painter until write-back errors land"
    )]
    Danger,
    Day,
    Night,
    Dawn,
    Work,
    Quiet,
    #[expect(dead_code, reason = "the map paints the night tint")]
    NightTint,
}

impl Role {
    /// The role's key in `[theme]`.
    fn key(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Dim => "dim",
            Self::Accent => "accent",
            Self::Marker => "marker",
            Self::Ok => "ok",
            Self::Warn => "warn",
            Self::Danger => "danger",
            Self::Day => "day",
            Self::Night => "night",
            Self::Dawn => "dawn",
            Self::Work => "work",
            Self::Quiet => "quiet",
            Self::NightTint => "night_tint",
        }
    }
}

/// What the terminal says about colour: `NO_COLOR`, and whether it renders 24-bit colour.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TerminalEnv {
    pub no_color: bool,
    pub truecolor: bool,
    /// `TEAMZONES_DEBUG=1`, which turns on the debug keys.
    pub debug: bool,
}

impl TerminalEnv {
    /// Reads `NO_COLOR` (set and non-empty), `COLORTERM` (`truecolor` or `24bit`) and
    /// `TEAMZONES_DEBUG`.
    pub fn from_env() -> Self {
        let set = |name: &str| std::env::var_os(name).is_some_and(|value| !value.is_empty());
        let colorterm = std::env::var("COLORTERM").unwrap_or_default();
        Self {
            no_color: set("NO_COLOR"),
            truecolor: matches!(colorterm.as_str(), "truecolor" | "24bit"),
            debug: debug_from(std::env::var("TEAMZONES_DEBUG").ok().as_deref()),
        }
    }
}

/// Whether a `TEAMZONES_DEBUG` value turns on the debug keys: only exactly `1` does.
pub fn debug_from(value: Option<&str>) -> bool {
    value == Some("1")
}

/// A resolved theme: a scheme and its overrides, read per role, plus the selection style.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Theme {
    scheme: Scheme,
    truecolor: bool,
    overrides: BTreeMap<String, String>,
    pub selection: Style,
}

fn rgb(hex: &str) -> Option<Color> {
    let channel = |range: std::ops::Range<usize>| {
        hex.get(range)
            .and_then(|digits| u8::from_str_radix(digits, 16).ok())
    };
    Some(Color::Rgb(channel(1..3)?, channel(3..5)?, channel(5..7)?))
}

fn dark(role: Role, truecolor: bool) -> Color {
    match role {
        Role::Text => Color::Reset,
        Role::Dim => Color::DarkGray,
        Role::Accent => Color::LightBlue,
        Role::Marker if truecolor => Color::Rgb(0xfb, 0x92, 0x3c),
        Role::Marker | Role::Danger => Color::LightRed,
        Role::Ok => Color::LightGreen,
        Role::Warn => Color::LightYellow,
        Role::Day => Color::Yellow,
        Role::Night | Role::Quiet => Color::Blue,
        Role::Dawn => Color::LightMagenta,
        Role::Work => Color::Green,
        Role::NightTint if truecolor => Color::Rgb(0x16, 0x1b, 0x33),
        Role::NightTint => Color::Reset,
    }
}

fn light(role: Role) -> Color {
    let hex = match role {
        Role::Text => "#24292f",
        Role::Dim => "#6e7781",
        Role::Accent => "#0969da",
        Role::Marker => "#bc4c00",
        Role::Ok | Role::Work => "#1a7f37",
        Role::Warn => "#9a6700",
        Role::Danger => "#cf222e",
        Role::Day => "#b08800",
        Role::Night | Role::Quiet => "#0550ae",
        Role::Dawn => "#8250df",
        Role::NightTint => "#e8ecf5",
    };
    rgb(hex).unwrap_or(Color::Reset)
}

fn mono(role: Role) -> Color {
    match role {
        Role::Dim | Role::Night | Role::Quiet | Role::NightTint => Color::DarkGray,
        Role::Text
        | Role::Accent
        | Role::Marker
        | Role::Ok
        | Role::Warn
        | Role::Danger
        | Role::Day
        | Role::Dawn
        | Role::Work => Color::Reset,
    }
}

impl Theme {
    /// The theme `[theme]` asks for, as this terminal can show it; `NO_COLOR` forces mono.
    pub fn resolve(theme: &config::Theme, env: TerminalEnv) -> Self {
        let scheme = if env.no_color {
            Scheme::Mono
        } else {
            theme.scheme
        };
        let custom = (scheme != Scheme::Mono)
            .then(|| theme.overrides.get("selection").and_then(|hex| rgb(hex)))
            .flatten();
        let selection = match (custom, scheme) {
            (Some(color), _) => Style::new().bg(color),
            (None, Scheme::Light) => Style::new().bg(rgb("#ddf4ff").unwrap_or(Color::Reset)),
            (None, Scheme::Dark | Scheme::Mono) => Style::new().add_modifier(Modifier::REVERSED),
        };
        Self {
            scheme,
            truecolor: env.truecolor,
            overrides: theme.overrides.clone(),
            selection,
        }
    }

    /// The role's override in hex when the scheme takes overrides, else the scheme's colour.
    pub fn color(&self, role: Role) -> Color {
        let custom = (self.scheme != Scheme::Mono)
            .then(|| self.overrides.get(role.key()).and_then(|hex| rgb(hex)))
            .flatten();
        custom.unwrap_or_else(|| match self.scheme {
            Scheme::Dark => dark(role, self.truecolor),
            Scheme::Light => light(role),
            Scheme::Mono => mono(role),
        })
    }

    /// Whether this theme draws no colour, as under `NO_COLOR` or `scheme = "mono"`.
    pub fn is_mono(&self) -> bool {
        self.scheme == Scheme::Mono
    }

    /// A foreground-only style for `role`.
    pub fn fg(&self, role: Role) -> Style {
        Style::new().fg(self.color(role))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_teamzones_debug_set_to_1_turns_on_debug() {
        assert!(debug_from(Some("1")));
        for value in [
            None,
            Some(""),
            Some("0"),
            Some("true"),
            Some("11"),
            Some(" 1"),
        ] {
            assert!(!debug_from(value), "{value:?}");
        }
    }

    #[test]
    fn no_color_forces_mono_over_the_configured_scheme_and_its_overrides() {
        let config = config::Theme {
            overrides: [("accent".to_owned(), "#123456".to_owned())].into(),
            ..config::Theme::default()
        };
        let theme = Theme::resolve(
            &config,
            TerminalEnv {
                no_color: true,
                truecolor: true,
                debug: false,
            },
        );
        let roles = [Role::Accent, Role::Warn, Role::Work, Role::Night];
        assert_eq!(roles.map(|role| theme.color(role)), roles.map(mono));
    }

    #[test]
    fn an_override_replaces_its_role_in_hex() {
        let config = config::Theme {
            overrides: [("accent".to_owned(), "#123456".to_owned())].into(),
            ..config::Theme::default()
        };
        let theme = Theme::resolve(&config, TerminalEnv::default());
        assert_eq!(theme.color(Role::Accent), Color::Rgb(0x12, 0x34, 0x56));
    }

    #[test]
    fn dark_marker_is_orange_only_with_truecolor() {
        let with = |truecolor| {
            Theme::resolve(
                &config::Theme::default(),
                TerminalEnv {
                    no_color: false,
                    truecolor,
                    debug: false,
                },
            )
            .color(Role::Marker)
        };
        assert_eq!(
            (with(true), with(false)),
            (Color::Rgb(0xfb, 0x92, 0x3c), Color::LightRed)
        );
    }
}
