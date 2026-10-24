//! Layout maths: where each part of the screen sits for a terminal size and a roster.

use ratatui::layout::Rect;

/// Whether the map takes the spare rows: by the hiding rule, or forced either way.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MapChoice {
    Auto,
    Hidden,
}

/// The label and track widths and the minutes each track cell covers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Columns {
    pub label: u16,
    pub track: u16,
    pub minutes: u16,
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

/// The header, slider, block borders, answer rows, say panel, notice and footer: everything but
/// the people and places themselves.
const FIXED_ROWS: u16 = 1 + 2 + 2 + 3 + 4 + 1 + 1;
/// The map needs this many columns, and this many rows left after the fixed rows.
const MAP_MIN_COLS: u16 = 100;
const MAP_MIN_ROWS: u16 = 12;
/// The track never takes the first 20 columns, which hold the labels.
const LABEL_MIN: u16 = 20;

/// The smallest of 15, 30 or 60 minutes a cell for which a whole day fits beside the labels.
fn columns(cols: u16) -> Columns {
    let minutes = [15, 30, 60]
        .into_iter()
        .find(|minutes| 1440 / minutes <= cols.saturating_sub(LABEL_MIN))
        .unwrap_or(60);
    let track = (1440 / minutes).min(cols);
    Columns {
        label: cols - track,
        track,
        minutes,
    }
}

/// Lays out a `cols` by `rows` screen for `roster_len` people (home included) and `places_len` places.
///
/// Block heights come from the whole roster, so switching groups never moves a row.
pub fn layout(cols: u16, rows: u16, roster_len: u16, places_len: u16, map: MapChoice) -> Areas {
    let fixed = FIXED_ROWS + roster_len + places_len;
    let spare = rows.saturating_sub(fixed);
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
        slider: take(2),
        who: take(roster_len + 2),
        place: take(places_len + 3),
        say: take(4),
        notice: take(1),
        footer: take(1),
        columns: columns(cols),
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
                minutes: 15
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
                minutes: 30
            }
        );
        assert_eq!(areas.footer, Rect::new(0, 23, 80, 1));
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
