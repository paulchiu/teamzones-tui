//! Layout maths: where each part of the screen sits for a terminal size and a roster.

use ratatui::layout::Rect;

/// Whether the map takes the spare rows: by the hiding rule, or forced either way.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MapChoice {
    Auto,
    Hidden,
}

/// The label and track widths; the track's columns split the viewed day evenly.
///
/// Side by side, the track starts after the label. Stacked, each row's label takes a line of its
/// own and the track fills the line under it. A track of no columns is the plain list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Columns {
    pub label: u16,
    pub track: u16,
    pub stacked: bool,
}

impl Columns {
    /// The column the track starts at.
    pub fn track_x(self) -> u16 {
        if self.stacked { 0 } else { self.label }
    }

    /// The screen lines one grid row takes: its label line, plus a lane line when stacked.
    pub fn lines(self) -> u16 {
        1 + u16::from(self.stacked)
    }
}

/// Every area of the screen, top to bottom; `map` is `None` while the map is hidden.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Areas {
    pub header: Rect,
    pub map: Option<Rect>,
    pub slider: Rect,
    pub who: Rect,
    pub place: Rect,
    pub say: Rect,
    pub notice: Rect,
    pub footer: Rect,
    pub columns: Columns,
}

/// The header, the block borders, the say panel, the notice and the footer: the rows that never
/// stack.
const FLAT_ROWS: u16 = 1 + 1 + 2 + 4 + 1 + 1;
/// Below this width rows stack, or become the plain list when stacking does not fit the height.
const STACK_BELOW: u16 = 80;
/// The map needs this many columns, and this many rows left after the fixed rows.
const MAP_MIN_COLS: u16 = 100;
const MAP_MIN_ROWS: u16 = 12;
/// The labels keep at least this many columns beside the track.
const LABEL_MIN: u16 = 20;

/// The fewest whole columns per hour that make the track at least twice the label, cut back
/// while the label would drop under its minimum.
///
/// So 120 columns gives four an hour (a 96-column track beside a 24-column label) and 80 gives
/// two (48 beside 32), and the label stays a third of the width or less as the terminal widens.
fn columns(cols: u16) -> Columns {
    let ideal = cols.div_ceil(36).max(1);
    let per_hour = (1..=ideal)
        .rev()
        .find(|per_hour| cols.saturating_sub(24 * per_hour) >= LABEL_MIN)
        .unwrap_or(1);
    let track = (24 * per_hour).min(cols);
    Columns {
        label: cols - track,
        track,
        stacked: false,
    }
}

/// The rows a screen needs with `lines` lines per grid row: the slider's two, each block's
/// answer row, and every person and place.
fn rows_needed(lines: u16, people: u16, places: u16) -> u16 {
    FLAT_ROWS + lines * (2 + 2 + people + places)
}

/// The columns for a `cols` by `rows` screen: side by side from 80 columns, stacked below that
/// when the height allows, and otherwise the plain list.
fn shape(cols: u16, rows: u16, people: u16, places: u16) -> Columns {
    let stacked = Columns {
        label: cols,
        track: cols,
        stacked: true,
    };
    let list = Columns {
        label: cols,
        track: 0,
        stacked: false,
    };
    match (cols >= STACK_BELOW, rows >= rows_needed(2, people, places)) {
        (true, _) => columns(cols),
        (false, true) => stacked,
        (false, false) => list,
    }
}

/// Lays out a `cols` by `rows` screen for `roster_len` people (home included) and `places_len` places.
///
/// Block heights come from the whole roster, so switching groups never moves a row.
pub fn layout(cols: u16, rows: u16, roster_len: u16, places_len: u16, map: MapChoice) -> Areas {
    let columns = shape(cols, rows, roster_len, places_len);
    let lines = columns.lines();
    let spare = rows.saturating_sub(rows_needed(lines, roster_len, places_len));
    let shows_map = match map {
        MapChoice::Auto => cols >= MAP_MIN_COLS && spare >= MAP_MIN_ROWS,
        MapChoice::Hidden => false,
    };
    let mut y = 0;
    let mut take = |height: u16| {
        let area = Rect::new(0, y, cols, height);
        y = y.saturating_add(height);
        area
    };
    let header = take(1);
    let spare_area = take(spare);
    Areas {
        header,
        map: shows_map.then_some(spare_area),
        slider: take(2 * lines),
        who: take(1 + lines * (roster_len + 1)),
        place: take(2 + lines * (places_len + 1)),
        say: take(4),
        notice: take(1),
        footer: take(1),
        columns,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each area's first row and height, for comparing with the row budget.
    fn rows(areas: &Areas) -> Vec<(u16, u16)> {
        [
            areas.header,
            areas.slider,
            areas.who,
            areas.place,
            areas.say,
            areas.notice,
            areas.footer,
        ]
        .iter()
        .map(|area| (area.y, area.height))
        .collect()
    }

    #[test]
    fn at_120_by_36_the_map_takes_the_twelve_spare_rows() {
        let areas = layout(120, 36, 7, 3, MapChoice::Auto);
        assert_eq!(areas.map, Some(Rect::new(0, 1, 120, 12)));
        assert_eq!(
            rows(&areas),
            [(0, 1), (13, 2), (15, 9), (24, 6), (30, 4), (34, 1), (35, 1)]
        );
        assert_eq!(
            areas.columns,
            Columns {
                label: 24,
                track: 96,
                stacked: false
            }
        );
    }

    #[test]
    fn at_120_by_24_the_map_is_hidden_and_the_fixed_rows_fill_the_screen() {
        let areas = layout(120, 24, 7, 3, MapChoice::Auto);
        assert_eq!(areas.map, None);
        assert_eq!(
            rows(&areas),
            [(0, 1), (1, 2), (3, 9), (12, 6), (18, 4), (22, 1), (23, 1)]
        );
    }

    #[test]
    fn at_80_by_24_the_track_is_48_half_hour_cells() {
        let areas = layout(80, 24, 7, 3, MapChoice::Auto);
        assert_eq!(areas.map, None);
        assert_eq!(
            areas.columns,
            Columns {
                label: 32,
                track: 48,
                stacked: false
            }
        );
        assert_eq!(areas.footer, Rect::new(0, 23, 80, 1));
    }

    #[test]
    fn from_80_columns_the_label_never_outgrows_the_track_and_from_120_stays_within_half() {
        for cols in 80..=400 {
            let Columns { label, track, .. } = layout(cols, 60, 7, 3, MapChoice::Auto).columns;
            assert_eq!(label + track, cols, "{cols} columns");
            assert!(
                label <= track,
                "{cols} columns: label {label}, track {track}"
            );
            if cols >= 120 {
                assert!(
                    2 * label <= track,
                    "{cols} columns: label {label}, track {track}"
                );
            }
        }
    }

    #[test]
    fn wide_terminals_give_the_track_whole_columns_per_hour() {
        let split = |cols| {
            let columns = layout(cols, 60, 7, 3, MapChoice::Auto).columns;
            (columns.label, columns.track)
        };
        assert_eq!(split(160), (40, 120));
        assert_eq!(split(200), (56, 144));
        assert_eq!(split(250), (82, 168));
        assert_eq!(split(320), (104, 216));
    }

    const SCREEN_SPEC: &str = include_str!("../../../../docs/spec/screen.md");

    /// The `N columns: L + T` examples in the screen spec's Columns bullet.
    fn spec_splits() -> Vec<(u16, u16, u16)> {
        let bullet = SCREEN_SPEC
            .lines()
            .find(|line| line.starts_with("* **Columns**"))
            .expect("the screen spec has a Columns bullet");
        bullet
            .split('`')
            .filter_map(|code| {
                let (cols, split) = code.split_once(" columns: ")?;
                let (label, track) = split.split_once(" + ")?;
                Some((cols.parse().ok()?, label.parse().ok()?, track.parse().ok()?))
            })
            .collect()
    }

    #[test]
    fn the_split_matches_every_example_in_the_screen_spec() {
        let splits = spec_splits();
        assert!(splits.len() >= 6, "{splits:?}");
        for (cols, label, track) in splits {
            let columns = layout(cols, 60, 7, 3, MapChoice::Auto).columns;
            assert_eq!(
                (columns.label, columns.track),
                (label, track),
                "{cols} columns"
            );
        }
    }

    #[test]
    fn under_100_columns_the_map_stays_hidden_however_tall() {
        assert_eq!(layout(99, 60, 7, 3, MapChoice::Auto).map, None);
    }

    #[test]
    fn a_hidden_map_leaves_its_rows_blank_above_the_slider() {
        let areas = layout(120, 36, 7, 3, MapChoice::Hidden);
        assert_eq!((areas.map, areas.slider.y, areas.footer.y), (None, 13, 35));
    }

    #[test]
    fn block_heights_come_from_the_whole_roster() {
        let areas = layout(120, 40, 2, 0, MapChoice::Auto);
        assert_eq!((areas.who.height, areas.place.height), (4, 3));
    }
}
