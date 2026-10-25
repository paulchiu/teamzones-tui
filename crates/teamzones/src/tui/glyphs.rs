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
    /// The edges and lines range styles' marks just outside the range: bars that hug it, or lines.
    pub edge_before: char,
    pub edge_after: char,
    pub edge_line: char,
    pub answer: &'static str,
    pub pointer: &'static str,
    pub detail: &'static str,
    pub back: &'static str,
    pub tick: char,
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
    edge_before: '▕',
    edge_after: '▏',
    edge_line: '│',
    answer: "≡",
    pointer: "▸",
    detail: "▶",
    back: "◂",
    tick: '·',
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
    edge_before: '|',
    edge_after: '|',
    edge_line: '|',
    answer: "=",
    pointer: ">",
    detail: ">",
    back: "<",
    tick: '.',
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
            Level::Closed => self.quiet,
            Level::Open => self.off_peak,
            Level::Busy => self.peak,
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

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

    const THEME_SPEC: &str = include_str!("../../../../docs/spec/theme.md");

    /// The spec's ascii glyph set, keyed by (section, full glyph or label): `` `a`→`b` `` items
    /// map `a` to `b` (pairwise when both hold several), and `` label `x` `` items map the label.
    fn ascii_spec() -> BTreeMap<(String, String), String> {
        let start = THEME_SPEC
            .find("**ascii glyph set**")
            .expect("theme.md has the ascii glyph set");
        let mut spec = BTreeMap::new();
        for line in THEME_SPEC[start..].lines().skip(1) {
            let Some(item_list) = line.trim_start().strip_prefix("+ ") else {
                break;
            };
            let (section, items) = match item_list.split_once(": ") {
                Some((section, items)) if !section.contains('`') => (section, items),
                _ => ("", item_list),
            };
            for item in items.trim_end_matches(';').split(", ") {
                let ticked: Vec<&str> = item.split('`').skip(1).step_by(2).collect();
                match (item.contains('→'), ticked.as_slice()) {
                    (true, [from, to]) => {
                        let (froms, tos): (Vec<&str>, Vec<&str>) =
                            (from.split(' ').collect(), to.split(' ').collect());
                        for (index, from) in froms.iter().enumerate() {
                            let to = if froms.len() == tos.len() {
                                tos[index]
                            } else {
                                to
                            };
                            spec.insert((section.to_owned(), (*from).to_owned()), to.to_owned());
                        }
                    }
                    (false, [value, ..]) => {
                        let label = item.split('`').next().unwrap_or_default().trim();
                        spec.insert((section.to_owned(), label.to_owned()), (*value).to_owned());
                    }
                    _ => {}
                }
            }
        }
        spec
    }

    #[test]
    fn the_ascii_set_is_the_one_the_theme_spec_lists() {
        let Glyphs {
            home,
            person,
            person_other,
            place,
            place_other,
            night,
            day,
            dawn,
            work,
            off,
            quiet,
            off_peak,
            peak,
            answer_none,
            answer_fill,
            answer_all,
            track,
            track_fit,
            track_partial,
            cursor,
            range_start,
            range_end,
            edge_before,
            edge_after,
            edge_line,
            answer,
            pointer,
            detail,
            back,
            tick,
            line,
            rounded,
            ascii,
        } = ASCII;
        let c = |glyph: char| glyph.to_string();
        let border = |set: border::Set<'static>| {
            format!(
                "{} {} {}",
                set.top_left, set.horizontal_top, set.vertical_left
            )
        };
        let mut expected = vec![
            ("marks", FULL.home.to_owned(), home.to_owned()),
            ("marks", FULL.person.to_owned(), person.to_owned()),
            (
                "marks",
                FULL.person_other.to_owned(),
                person_other.to_owned(),
            ),
            ("marks", FULL.place.to_owned(), place.to_owned()),
            ("marks", FULL.place_other.to_owned(), place_other.to_owned()),
            ("person lanes", c(FULL.night), c(night)),
            ("person lanes", c(FULL.day), c(day)),
            ("person lanes", c(FULL.dawn), c(dawn)),
            ("person lanes", c(FULL.work), c(work)),
            ("person lanes", c(FULL.off), c(off)),
            ("place lanes", c(FULL.quiet), c(quiet)),
            ("place lanes", c(FULL.off_peak), c(off_peak)),
            ("place lanes", c(FULL.peak), c(peak)),
            ("answer rows", "none".to_owned(), c(answer_none)),
            ("answer rows", "all".to_owned(), c(answer_all)),
            ("track", String::new(), c(track)),
            ("track", "fit".to_owned(), c(track_fit)),
            ("track", "partial".to_owned(), c(track_partial)),
            ("track", "cursor".to_owned(), c(cursor)),
            (
                "track",
                "range ends".to_owned(),
                format!("{range_start} {range_end}"),
            ),
            ("range marking", c(FULL.edge_before), c(edge_before)),
            ("range marking", c(FULL.edge_after), c(edge_after)),
            ("range marking", c(FULL.edge_line), c(edge_line)),
            ("", FULL.answer.to_owned(), answer.to_owned()),
            ("", FULL.pointer.to_owned(), pointer.to_owned()),
            ("", FULL.detail.to_owned(), detail.to_owned()),
            ("", FULL.back.to_owned(), back.to_owned()),
            ("", c(FULL.tick), c(tick)),
            ("", "borders and corners".to_owned(), border(line)),
            ("", "borders and corners".to_owned(), border(rounded)),
        ];
        expected.extend(answer_fill.map(|fill| ("answer rows", "partial".to_owned(), c(fill))));
        let spec = ascii_spec();
        let wrong: Vec<_> = expected
            .into_iter()
            .filter(|(section, key, glyph)| {
                spec.get(&((*section).to_owned(), key.clone())) != Some(glyph)
            })
            .collect();
        assert!(
            ascii && wrong.is_empty(),
            "differs from theme.md: {wrong:?}\n{spec:#?}"
        );
    }
}
