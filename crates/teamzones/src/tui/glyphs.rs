//! The two glyph sets: full Unicode, and plain ASCII for terminals and fonts without it.

use ratatui::symbols::border;
use teamzones_core::schedule::{Band, Level};

/// Every glyph the screen draws, so the ASCII set can replace each one.
#[derive(Debug, PartialEq, Eq)]
pub struct Glyphs {
    pub home: &'static str,
    pub person: &'static str,
    pub person_other: &'static str,
    pub place: &'static str,
    pub place_other: &'static str,
    pub night: char,
    pub day: char,
    pub dawn: char,
    pub work: char,
    pub off: char,
    pub quiet: char,
    pub off_peak: char,
    pub peak: char,
    pub answer_none: char,
    pub answer_fill: [char; 7],
    pub answer_all: char,
    pub track: char,
    pub track_fit: char,
    pub track_partial: char,
    pub cursor: char,
    pub range_start: char,
    pub range_end: char,
    pub answer: &'static str,
    pub pointer: &'static str,
    pub detail: &'static str,
    pub back: &'static str,
    pub tick: char,
    pub ellipsis: &'static str,
    pub line: border::Set<'static>,
    pub rounded: border::Set<'static>,
    pub ascii: bool,
}

pub const FULL: Glyphs = Glyphs {
    home: "⌂",
    person: "●",
    person_other: "○",
    place: "◆",
    place_other: "◇",
    night: '░',
    day: '▒',
    dawn: '▚',
    work: '▓',
    off: '╌',
    quiet: '▁',
    off_peak: '▄',
    peak: '█',
    answer_none: '·',
    answer_fill: ['⡀', '⡄', '⡆', '⡇', '⣇', '⣧', '⣷'],
    answer_all: '━',
    track: '─',
    track_fit: '═',
    track_partial: '╌',
    cursor: '┃',
    range_start: '┣',
    range_end: '┫',
    answer: "≡",
    pointer: "▸",
    detail: "▶",
    back: "◂",
    tick: '·',
    ellipsis: "…",
    line: border::PLAIN,
    rounded: border::ROUNDED,
    ascii: false,
};

const ASCII_BORDER: border::Set<'static> = border::Set {
    top_left: "+",
    top_right: "+",
    bottom_left: "+",
    bottom_right: "+",
    vertical_left: "|",
    vertical_right: "|",
    horizontal_top: "-",
    horizontal_bottom: "-",
};

pub const ASCII: Glyphs = Glyphs {
    home: "^",
    person: "*",
    person_other: "o",
    place: "@",
    place_other: "-",
    night: '.',
    day: ':',
    dawn: '+',
    work: '#',
    off: ',',
    quiet: '_',
    off_peak: '=',
    peak: 'X',
    answer_none: '.',
    answer_fill: ['-'; 7],
    answer_all: '=',
    track: '-',
    track_fit: '=',
    track_partial: '~',
    cursor: '|',
    range_start: '[',
    range_end: ']',
    answer: "=",
    pointer: ">",
    detail: ">",
    back: "<",
    tick: '.',
    ellipsis: "...",
    line: ASCII_BORDER,
    rounded: ASCII_BORDER,
    ascii: true,
};

impl Glyphs {
    /// The set config asks for.
    pub fn of(set: teamzones_core::config::Glyphs) -> &'static Self {
        match set {
            teamzones_core::config::Glyphs::Full => &FULL,
            teamzones_core::config::Glyphs::Ascii => &ASCII,
        }
    }

    pub fn band(&self, band: Band) -> char {
        match band {
            Band::Night => self.night,
            Band::Day => self.day,
            Band::Morning | Band::Evening => self.dawn,
            Band::Working => self.work,
            Band::Off => self.off,
        }
    }

    pub fn level(&self, level: Level) -> char {
        match level {
            Level::Quiet => self.quiet,
            Level::OffPeak => self.off_peak,
            Level::Peak => self.peak,
        }
    }

    /// Spec text as this set can show it: unchanged in full, and every non-ASCII glyph spelled
    /// in ASCII otherwise.
    pub fn text(&self, text: &str) -> String {
        if !self.ascii {
            return text.to_owned();
        }
        text.chars()
            .map(|c| match c {
                '←' => "<-".to_owned(),
                '→' => "->".to_owned(),
                '↑' => "up".to_owned(),
                '↓' => "down".to_owned(),
                '┣' | '[' => "[".to_owned(),
                '┫' | ']' => "]".to_owned(),
                '═' | '━' => "=".to_owned(),
                '◂' => "<".to_owned(),
                '·' => ".".to_owned(),
                '⋯' | '…' => "...".to_owned(),
                c if c.is_ascii() => c.to_string(),
                _ => "?".to_owned(),
            })
            .collect()
    }
}
