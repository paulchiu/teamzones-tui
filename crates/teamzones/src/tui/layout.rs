//! Layout maths: where each part of the screen sits for a terminal size and a roster.

use ratatui::layout::Rect;

/// Whether the map takes the spare rows: by the hiding rule, whenever there are any, or never.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MapChoice {
    Auto,
    Always,
    Hidden,
}

/// Where the map is drawn: in the spare rows, nowhere, or over the grid in a box.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MapView {
    InPlace,
    Hidden,
    Overlay,
}

/// The map's place for a screen whose configured choice is `base`, after `m` has `flipped` it:
/// a shown map hides, and a hidden one shows where the hiding rule finds room, else as an overlay.
pub fn map_view(
    cols: u16,
    rows: u16,
    (people, places): (u16, u16),
    base: MapChoice,
    flipped: bool,
) -> MapView {
    let shown = layout(cols, rows, people, places, base).map.is_some();
    match (shown, flipped) {
        (true, false) => MapView::InPlace,
        (true, true) | (false, false) => MapView::Hidden,
        (false, true) if room_for_map(cols, rows, (people, places)) => MapView::InPlace,
        (false, true) => MapView::Overlay,
    }
}

/// Whether the hiding rule finds room for the map on a `cols` by `rows` screen.
pub fn room_for_map(cols: u16, rows: u16, (people, places): (u16, u16)) -> bool {
    layout(cols, rows, people, places, MapChoice::Auto)
        .map
        .is_some()
}

/// How each grid row splits into its label and its track; the track's columns split the viewed
/// day evenly.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Columns {
    /// The label, then the track on the same line.
    Beside { label: u16, track: u16 },
    /// The label on a line of its own, over a full-width track.
    Stacked { width: u16 },
    /// The label alone, with no track.
    List { width: u16 },
}

impl Columns {
    /// The label's width.
    pub fn label(self) -> u16 {
        match self {
            Self::Beside { label, .. } => label,
            Self::Stacked { width } | Self::List { width } => width,
        }
    }

    /// The track's width: its number of columns.
    pub fn track(self) -> u16 {
        match self {
            Self::Beside { track, .. } => track,
            Self::Stacked { width } => width,
            Self::List { .. } => 0,
        }
    }

    /// The column the track starts at.
    pub fn track_x(self) -> u16 {
        match self {
            Self::Beside { label, .. } => label,
            Self::Stacked { .. } | Self::List { .. } => 0,
        }
    }

    /// The screen lines one grid row takes: its label line, plus a lane line when stacked.
    pub fn lines(self) -> u16 {
        match self {
            Self::Stacked { .. } => 2,
            Self::Beside { .. } | Self::List { .. } => 1,
        }
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
    /// The focused row's detail line.
    pub detail: Rect,
    /// The times box: the copy line in its border.
    pub say: Rect,
    pub notice: Rect,
    pub footer: Rect,
    pub columns: Columns,
}

/// The header, the block borders, the detail line, the times box, the notice and the footer: the
/// rows that never stack.
const FLAT_ROWS: u16 = 1 + 1 + 2 + 4 + 1 + 1;
/// Below this width rows stack when the height allows.
const STACK_BELOW: u16 = 80;
/// Below this width, rows too many to stack become the plain list.
const LIST_BELOW: u16 = 60;
/// The map needs this many columns, and this many rows left after the fixed rows.
const MAP_MIN_COLS: u16 = 100;
const MAP_MIN_ROWS: u16 = 12;
/// Below this many columns or rows the screen only says it is too small.
const MIN_COLS: u16 = 40;
const MIN_ROWS: u16 = 10;
/// The labels keep at least this many columns beside the track.
const LABEL_MIN: u16 = 20;

/// The label beside a track of the fewest whole columns per hour that make it at least twice
/// the label, cut back while the label would drop under its minimum; `None` when even one an
/// hour leaves the label too narrow.
fn beside(cols: u16) -> Option<Columns> {
    (1..=cols.div_ceil(36))
        .rev()
        .find(|per_hour| cols.saturating_sub(24 * per_hour) >= LABEL_MIN)
        .map(|per_hour| Columns::Beside {
            label: cols - 24 * per_hour,
            track: 24 * per_hour,
        })
}

/// The rows a screen needs with `lines` lines per grid row: the slider's two, each block's
/// answer row, and every person and place.
fn rows_needed(lines: u16, people: u16, places: u16) -> u16 {
    FLAT_ROWS + lines * (2 + 2 + people + places)
}

/// The columns for a `cols` by `rows` screen: side by side from 80 columns, stacked below that
/// when the height allows, side by side again from 60 columns, and otherwise the plain list.
fn shape(cols: u16, rows: u16, people: u16, places: u16) -> Columns {
    let list = Columns::List { width: cols };
    if cols >= STACK_BELOW {
        beside(cols).unwrap_or(list)
    } else if rows >= rows_needed(2, people, places) {
        Columns::Stacked { width: cols }
    } else if cols >= LIST_BELOW {
        beside(cols).unwrap_or(list)
    } else {
        list
    }
}

/// Whether a `cols` by `rows` terminal is too small to draw anything but the notice saying so.
pub fn too_small(cols: u16, rows: u16) -> bool {
    cols < MIN_COLS || rows < MIN_ROWS
}

/// The people and places rows each block shows on `rows` lines with `lines` lines per grid row:
/// every one when they fit, else the where block folds first, down to three places, then the
/// who block, down to home and one more.
fn shown(rows: u16, lines: u16, people: u16, places: u16) -> (u16, u16) {
    let mut short = rows_needed(lines, people, places)
        .saturating_sub(rows)
        .div_ceil(lines.max(1));
    let places_shown = places - short.min(places - places.min(3));
    short -= places - places_shown;
    let people_shown = people - short.min(people.saturating_sub(2));
    (people_shown, places_shown)
}

/// Lays out a `cols` by `rows` screen for `roster_len` people (home included) and `places_len` places.
///
/// Block heights come from the whole roster and the size, so switching groups never moves a row.
/// The times box, notice and footer keep to the bottom, and the spare rows sit over the times box:
/// all of them, or one when a shown map under the header takes the rest. Short of rows, the
/// blocks fold.
pub fn layout(cols: u16, rows: u16, roster_len: u16, places_len: u16, map: MapChoice) -> Areas {
    let columns = shape(cols, rows, roster_len, places_len);
    let lines = columns.lines();
    let spare = rows.saturating_sub(rows_needed(lines, roster_len, places_len));
    let (people_shown, places_shown) = shown(rows, lines, roster_len, places_len);
    let shows_map = match map {
        MapChoice::Auto => cols >= MAP_MIN_COLS && spare >= MAP_MIN_ROWS,
        MapChoice::Always => spare > 1,
        MapChoice::Hidden => false,
    };
    let gap = if shows_map { 1 } else { spare };
    let mut y = 0;
    let mut take = |height: u16| {
        let area = Rect::new(0, y, cols, height);
        y = y.saturating_add(height);
        area
    };
    let header = take(1);
    let map = shows_map.then(|| take(spare - gap));
    let slider = take(2 * lines);
    let who = take(1 + lines * (people_shown + 1));
    let place = take(2 + lines * (places_shown + 1));
    let detail = take(1);
    take(gap);
    let say = take(3);
    Areas {
        header,
        map,
        slider,
        who,
        place,
        detail,
        say,
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
            areas.detail,
            areas.say,
            areas.notice,
            areas.footer,
        ]
        .iter()
        .map(|area| (area.y, area.height))
        .collect()
    }

    #[test]
    fn at_120_by_36_the_map_takes_eleven_of_the_twelve_spare_rows_and_one_sits_over_the_times_box()
    {
        let areas = layout(120, 36, 7, 3, MapChoice::Auto);
        assert_eq!(areas.map, Some(Rect::new(0, 1, 120, 11)));
        assert_eq!(
            rows(&areas),
            [
                (0, 1),
                (12, 2),
                (14, 9),
                (23, 6),
                (29, 1),
                (31, 3),
                (34, 1),
                (35, 1)
            ]
        );
        assert_eq!(
            areas.columns,
            Columns::Beside {
                label: 24,
                track: 96
            }
        );
    }

    #[test]
    fn at_120_by_24_the_map_is_hidden_and_the_fixed_rows_fill_the_screen() {
        let areas = layout(120, 24, 7, 3, MapChoice::Auto);
        assert_eq!(areas.map, None);
        assert_eq!(
            rows(&areas),
            [
                (0, 1),
                (1, 2),
                (3, 9),
                (12, 6),
                (18, 1),
                (19, 3),
                (22, 1),
                (23, 1)
            ]
        );
    }

    #[test]
    fn at_80_by_24_the_track_is_48_half_hour_cells() {
        let areas = layout(80, 24, 7, 3, MapChoice::Auto);
        assert_eq!(areas.map, None);
        assert_eq!(
            areas.columns,
            Columns::Beside {
                label: 32,
                track: 48
            }
        );
        assert_eq!(areas.footer, Rect::new(0, 23, 80, 1));
    }

    #[test]
    fn from_80_columns_the_label_never_outgrows_the_track_and_from_120_stays_within_half() {
        for cols in 80..=400 {
            let columns = layout(cols, 60, 7, 3, MapChoice::Auto).columns;
            let (label, track) = (columns.label(), columns.track());
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
            assert_eq!(columns, Columns::Beside { label, track }, "{cols} columns");
        }
    }

    #[test]
    fn under_100_columns_the_map_stays_hidden_however_tall() {
        assert_eq!(layout(99, 60, 7, 3, MapChoice::Auto).map, None);
    }

    #[test]
    fn a_hidden_map_moves_the_blocks_up_and_leaves_its_rows_above_the_times_box() {
        let areas = layout(120, 36, 7, 3, MapChoice::Hidden);
        assert_eq!(areas.map, None);
        assert_eq!(
            rows(&areas),
            [
                (0, 1),
                (1, 2),
                (3, 9),
                (12, 6),
                (18, 1),
                (31, 3),
                (34, 1),
                (35, 1)
            ]
        );
    }

    #[test]
    fn wherever_the_blocks_fit_whole_the_times_box_keeps_to_the_bottom_under_a_blank_row() {
        for (cols, rows, choice) in (60..=240).step_by(5).flat_map(|cols| {
            (10..=60).flat_map(move |rows| {
                [MapChoice::Auto, MapChoice::Always, MapChoice::Hidden]
                    .map(|choice| (cols, rows, choice))
            })
        }) {
            let areas = layout(cols, rows, 7, 3, choice);
            let needed = rows_needed(areas.columns.lines(), 7, 3);
            if rows < needed {
                continue;
            }
            let spare = rows - needed;
            let at = format!("{cols}x{rows} {choice:?}");
            let bottom = [areas.say.y, areas.notice.y, areas.footer.y];
            assert_eq!(bottom, [rows - 5, rows - 2, rows - 1], "{at}");
            let gap = areas.say.y - areas.detail.bottom();
            assert_eq!(gap > 0, spare > 0, "{at}: {gap} blank rows, {spare} spare");
        }
    }

    #[test]
    fn m_flips_each_configured_choice_to_the_other_or_to_an_overlay_without_room() {
        let view = |cols, rows, base, flipped| map_view(cols, rows, (7, 3), base, flipped);
        assert_eq!(view(120, 36, MapChoice::Auto, false), MapView::InPlace);
        assert_eq!(view(120, 36, MapChoice::Auto, true), MapView::Hidden);
        assert_eq!(view(120, 24, MapChoice::Auto, true), MapView::Overlay);
        assert_eq!(view(120, 36, MapChoice::Hidden, true), MapView::InPlace);
        // `always` needs only two spare rows, so 30 rows leave it 6 even under the 12 auto needs.
        assert_eq!(view(120, 30, MapChoice::Auto, false), MapView::Hidden);
        assert_eq!(view(120, 30, MapChoice::Always, false), MapView::InPlace);
        assert_eq!(view(120, 30, MapChoice::Always, true), MapView::Hidden);
    }

    #[test]
    fn always_shows_no_map_with_one_spare_row_and_leaves_it_blank_over_the_times_box() {
        let areas = layout(120, 25, 7, 3, MapChoice::Always);
        assert_eq!(areas.map, None);
        assert_eq!(areas.say.y - areas.detail.bottom(), 1);
        assert!(layout(120, 26, 7, 3, MapChoice::Always).map.is_some());
    }

    #[test]
    fn at_80_by_20_the_who_block_folds_to_fit_and_everything_else_keeps_its_rows() {
        let areas = layout(80, 20, 7, 3, MapChoice::Auto);
        // The who block keeps its border, home, two more rows and the answer row.
        assert_eq!(
            rows(&areas),
            [
                (0, 1),
                (1, 2),
                (3, 5),
                (8, 6),
                (14, 1),
                (15, 3),
                (18, 1),
                (19, 1)
            ]
        );
    }

    #[test]
    fn short_of_rows_the_where_block_folds_first_down_to_three_places() {
        // 27 rows needed for 7 people and 6 places, so 22 leaves 5 to find.
        let areas = layout(80, 22, 7, 6, MapChoice::Auto);
        assert_eq!((areas.who.height, areas.place.height), (7, 6));
        assert_eq!(areas.footer.y, 21);
    }

    #[test]
    fn the_who_block_never_folds_below_home_one_more_row_and_its_answer() {
        let areas = layout(80, 12, 7, 3, MapChoice::Auto);
        assert_eq!((areas.who.height, areas.place.height), (4, 6));
    }

    #[test]
    fn block_heights_come_from_the_whole_roster() {
        let areas = layout(120, 40, 2, 0, MapChoice::Auto);
        assert_eq!((areas.who.height, areas.place.height), (4, 3));
    }
}
