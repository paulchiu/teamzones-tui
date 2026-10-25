//! The TUI's state, and the pure queries the update and the view both read.

use chrono::{DateTime, NaiveDate, Offset, Utc};
use teamzones_core::clock::{Range, Slot, When};
use teamzones_core::config::{Config, MapShow, PersonId, RangeStyle, Roster};
use teamzones_core::copy::{Format, Who, format_line};
use teamzones_core::question::{Marks, Row};

use super::footer::MapKey;
use super::glyphs::Glyphs;
use super::keys::{Context, Sort};
use super::layout::{MapChoice, MapView, map_view, room_for_map};
use super::matrix::Matrix;
use super::mouse::Drag;
use super::notice::Notice;
use super::picker::Picker;
use super::undo::Undo;

/// Where the cursor sits: on exact now, or on a grid slot once anything moves it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cursor {
    Now,
    At(Slot),
}

/// A visual range being drawn: its fixed and moving ends, and what to restore on `esc`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Visual {
    pub anchor: Slot,
    pub active: Slot,
    pub before: (Cursor, Option<Range>),
}

impl Visual {
    /// The range from the earlier end through the later one.
    pub fn range(self) -> Range {
        let (start, last) = if self.anchor <= self.active {
            (self.anchor, self.active)
        } else {
            (self.active, self.anchor)
        };
        Range::new(start, last.next()).unwrap_or(Range::single(start))
    }
}

/// The active lens: a configured group by index, or `0 all`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lens {
    Group(usize),
    All,
}

/// Which modal, if any, owns the keys.
#[derive(Clone, Debug, PartialEq)]
pub enum Mode {
    Normal,
    Prompt(String),
    /// The `+` prompt in the strip: the new group's name as typed so far.
    NewGroup(String),
    CopyMenu,
    SortMenu,
    /// Help for the keys of a context, over the mode it opened from, which closing it returns to.
    Help(Context, Box<Mode>),
    /// The membership matrix overlay.
    Matrix(Box<Matrix>),
    /// `a`'s choice of a person or a place, with the map point a double click gave.
    AddMenu(Option<(f64, f64)>),
    /// The picker for a new or edited person or place.
    Picker(Box<Picker>),
    /// The drift tag's menu: save the drift, or revert it.
    DriftMenu,
}

/// How people lanes are drawn: by band, or plain day and night.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lanes {
    Hours,
    Plain,
}

/// The style after `style` in the `R` cycle.
pub fn next_style(style: RangeStyle) -> RangeStyle {
    match style {
        RangeStyle::Columns => RangeStyle::Edges,
        RangeStyle::Edges => RangeStyle::Lines,
        RangeStyle::Lines => RangeStyle::Underline,
        RangeStyle::Underline => RangeStyle::Focus,
        RangeStyle::Focus => RangeStyle::Columns,
    }
}

/// The style's name, as `[theme] range` spells it and the footer shows it.
pub fn style_name(style: RangeStyle) -> &'static str {
    match style {
        RangeStyle::Columns => "columns",
        RangeStyle::Edges => "edges",
        RangeStyle::Lines => "lines",
        RangeStyle::Underline => "underline",
        RangeStyle::Focus => "focus",
    }
}

/// The style to draw when the theme has no colour: those that need a background or a dim
/// foreground fall back to edge lines.
pub fn without_colour(style: RangeStyle, mono: bool) -> RangeStyle {
    match style {
        RangeStyle::Columns | RangeStyle::Edges | RangeStyle::Focus if mono => RangeStyle::Lines,
        RangeStyle::Columns
        | RangeStyle::Edges
        | RangeStyle::Lines
        | RangeStyle::Underline
        | RangeStyle::Focus => style,
    }
}

/// The rows of one block in display order: above the answer row, then below it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockRows {
    pub above: Vec<Row>,
    pub below: Vec<Row>,
}

impl BlockRows {
    pub fn iter(&self) -> impl Iterator<Item = Row> + '_ {
        self.above.iter().chain(&self.below).copied()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct App {
    pub config: Config,
    /// The instant the last tick sampled; every "now" the update and view need comes from here.
    pub now: DateTime<Utc>,
    pub cursor: Cursor,
    pub range: Option<Range>,
    pub visual: Option<Visual>,
    pub lens: Lens,
    pub marks: Marks,
    pub focus: Row,
    pub notice: Option<Notice>,
    pub undo: Option<Undo>,
    pub mode: Mode,
    pub lanes: Lanes,
    pub sort: Sort,
    pub running: bool,
    /// How the range is marked: `[theme] range` at start, then whatever `R` cycled to.
    pub range_style: RangeStyle,
    /// The footer's note naming the range style, raised when `R` changes it.
    pub style_flash: Option<Notice>,
    /// The terminal's columns and rows, as the last resize reported them.
    pub size: (u16, u16),
    /// Whether `m` has flipped the map's automatic choice since the last resize.
    pub map_flipped: bool,
    /// Whether the app holds mouse capture; `M` releases it for text selection.
    pub mouse: bool,
    /// The left-button drag in progress, if any.
    pub drag: Option<Drag>,
    /// The config file's text the config was parsed from, or `None` when there is no file.
    pub text: Option<String>,
    /// What the file on disk holds, as far as the app knows: `text` once every edit is written.
    pub disk: Option<String>,
    /// When `text` is due on disk, while a close waits out its undo window unwritten.
    pub flush_at: Option<DateTime<Utc>>,
    /// The cell and instant of the last left-button press, for spotting a double click.
    pub last_click: Option<(u16, u16, DateTime<Utc>)>,
}

impl App {
    /// The app at `now` on the first group (or `0 all` without one), focused on home.
    pub fn new(config: Config, now: DateTime<Utc>) -> Self {
        let lens = if config.roster.groups.is_empty() {
            Lens::All
        } else {
            Lens::Group(0)
        };
        let marks = base_marks(&config.roster, lens);
        let range_style = config.theme.range;
        Self {
            config,
            now,
            cursor: Cursor::Now,
            range: None,
            visual: None,
            lens,
            marks,
            focus: Row::Person(PersonId::HOME),
            notice: None,
            undo: None,
            mode: Mode::Normal,
            lanes: Lanes::Hours,
            sort: Sort::Offset,
            running: true,
            range_style,
            style_flash: None,
            size: (0, 0),
            map_flipped: false,
            mouse: true,
            drag: None,
            text: None,
            disk: None,
            flush_at: None,
            last_click: None,
        }
    }

    /// The app reading its config from a file holding `text`, or from no file, which raises the
    /// first-run notice.
    pub fn with_source(self, text: Option<String>) -> Self {
        let notice = text.is_none().then(|| Notice {
            text: "no config: E creates one".to_owned(),
            raised_at: self.now,
        });
        Self {
            disk: text.clone(),
            text,
            notice,
            ..self
        }
    }

    /// Where the map goes on a `cols` by `rows` screen; the ascii glyph set never draws it.
    pub fn map_view_at(&self, cols: u16, rows: u16) -> MapView {
        let ascii = self.ascii();
        let base = match (ascii, self.config.map) {
            (true, _) | (false, MapShow::Never) => MapChoice::Hidden,
            (false, MapShow::Auto) => MapChoice::Auto,
            (false, MapShow::Always) => MapChoice::Always,
        };
        map_view(
            cols,
            rows,
            self.roster_size(),
            base,
            self.map_flipped && !ascii,
        )
    }

    /// What `m` does on a `cols` by `rows` screen, which decides the footer's `m` hint.
    pub fn map_key_at(&self, cols: u16, rows: u16) -> MapKey {
        let room = room_for_map(cols, rows, self.roster_size());
        match (self.ascii(), self.map_view_at(cols, rows), room) {
            (true, _, _) => MapKey::Inert,
            (false, MapView::InPlace, _) | (false, MapView::Hidden | MapView::Overlay, true) => {
                MapKey::Flips
            }
            (false, MapView::Hidden | MapView::Overlay, false) => MapKey::Opens,
        }
    }

    fn ascii(&self) -> bool {
        Glyphs::of(self.config.theme.glyphs).ascii
    }

    /// The whole roster's people and places, which size the blocks.
    fn roster_size(&self) -> (u16, u16) {
        let count = |n: usize| u16::try_from(n).unwrap_or(u16::MAX);
        (
            count(self.roster().people().count()),
            count(self.roster().places().count()),
        )
    }

    /// Where the map goes at the size the terminal last reported.
    pub fn map_view(&self) -> MapView {
        self.map_view_at(self.size.0, self.size.1)
    }

    pub fn roster(&self) -> &Roster {
        &self.config.roster
    }

    /// The range on screen: the visual range while drawing one, else the committed range.
    pub fn shown_range(&self) -> Option<Range> {
        self.visual.map(Visual::range).or(self.range)
    }

    /// Whether the view sits on exact now, with no range drawn or committed.
    pub fn at_now(&self) -> bool {
        self.cursor == Cursor::Now && self.shown_range().is_none()
    }

    /// What the say line and labels are about: the range, or the cursor's instant.
    pub fn when(&self) -> When {
        match (self.shown_range(), self.cursor) {
            (Some(range), _) => When::Over(range),
            (None, Cursor::Now) => When::At(self.now),
            (None, Cursor::At(slot)) => When::At(slot.start()),
        }
    }

    /// The slot the view starts from: the range's start, or the cursor's slot.
    pub fn start_slot(&self) -> Slot {
        Slot::containing(self.when().start())
    }

    /// The axis-zone date the track shows.
    pub fn day(&self) -> NaiveDate {
        self.when()
            .start()
            .with_timezone(&self.roster().axis())
            .date_naive()
    }

    pub fn today(&self) -> NaiveDate {
        self.now.with_timezone(&self.roster().axis()).date_naive()
    }

    /// The lens's name: its group's name, or `all`.
    pub fn lens_name(&self) -> String {
        match self.lens {
            Lens::Group(index) => self
                .roster()
                .groups
                .get(index)
                .map_or_else(|| "all".to_owned(), |group| group.name.clone()),
            Lens::All => "all".to_owned(),
        }
    }

    /// The marks differ from the lens's saved members as `+@UK` or `-Sam`, people first, then a
    /// changed `where` as `=closed`.
    pub fn drift(&self) -> Vec<String> {
        let base = base_marks(self.roster(), self.lens);
        let rule = (base.rule != self.marks.rule).then(|| format!("={}", self.marks.rule.word()));
        self.member_drift().into_iter().chain(rule).collect()
    }

    /// The marks' members that differ from the lens's saved ones, as `+@UK` or `-Sam`.
    fn member_drift(&self) -> Vec<String> {
        let roster = self.roster();
        let base = base_marks(roster, self.lens);
        let people = roster
            .people()
            .filter(|(id, _)| *id != PersonId::HOME)
            .filter_map(|(id, person)| {
                match (base.people.contains(&id), self.marks.people.contains(&id)) {
                    (false, true) => Some(format!("+{}", person.name)),
                    (true, false) => Some(format!("-{}", person.name)),
                    (true, true) | (false, false) => None,
                }
            });
        let places = roster.places().filter_map(|(id, place)| {
            match (base.places.contains(&id), self.marks.places.contains(&id)) {
                (false, true) => Some(format!("+@{}", place.name)),
                (true, false) => Some(format!("-@{}", place.name)),
                (true, true) | (false, false) => None,
            }
        });
        people.chain(places).collect()
    }

    /// The who-expression the marks stand for, such as `team +@UK`.
    pub fn expression(&self) -> String {
        std::iter::once(self.lens_name())
            .chain(self.member_drift())
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// The say line: the default format's line, exactly what `y` copies.
    pub fn say_line(&self) -> String {
        self.line_of(self.config.copy.default_format())
    }

    fn line_of(&self, format: &Format) -> String {
        format_line(self.roster(), &self.marks, self.focus, self.when(), format)
    }

    /// The line the format with `key` copies, or `None` when no format has that key.
    pub fn copy_as(&self, key: char) -> Option<String> {
        self.config.copy.get(key).map(|format| self.line_of(format))
    }

    /// The copy-as dialog's rows, in format order: each format's key, the exact line it copies
    /// now, and its label (naming the focused row for a focused-first format), leaving out a
    /// format whose line an earlier row already shows.
    pub fn copy_menu(&self) -> Vec<(char, String, String)> {
        let roster = self.roster();
        let focused = match self.focus {
            Row::Person(id) => &roster.person(id).name,
            Row::Place(id) => &roster.place(id).name,
        };
        let mut rows: Vec<(char, String, String)> = Vec::new();
        for format in self.config.copy.all() {
            let line = self.line_of(format);
            let about = match format.who {
                Who::FocusedFirst => format!("{} ({focused})", format.label),
                Who::Marked | Who::All => format.label.clone(),
            };
            if rows.iter().all(|(_, shown, _)| *shown != line) {
                rows.push((format.key, line, about));
            }
        }
        rows
    }

    /// Whether `row` sits above its block's line: home when the marks ask who, or a marked row.
    pub fn is_counted(&self, row: Row) -> bool {
        match row {
            Row::Person(id) if id == PersonId::HOME => self.marks.asks_who(),
            Row::Person(id) => self.marks.people.contains(&id),
            Row::Place(id) => self.marks.places.contains(&id),
        }
    }

    fn ordered(&self, mut rows: Vec<Row>) -> Vec<Row> {
        let roster = self.roster();
        let start = self.when().start();
        let offset = |row: &Row| {
            let tz = match *row {
                Row::Person(id) => roster.person(id).location.tz,
                Row::Place(id) => roster.place(id).location.tz,
            };
            -start.with_timezone(&tz).offset().fix().local_minus_utc()
        };
        let name = |row: &Row| match *row {
            Row::Person(id) => roster.person(id).name.to_lowercase(),
            Row::Place(id) => roster.place(id).name.to_lowercase(),
        };
        match self.sort {
            Sort::Offset => rows.sort_by_key(offset),
            Sort::Name => rows.sort_by_key(name),
        }
        rows
    }

    /// The who block's rows: home, the counted people, then the rest.
    pub fn who_rows(&self) -> BlockRows {
        let (counted, other): (Vec<Row>, Vec<Row>) = self
            .roster()
            .people()
            .filter(|(id, _)| *id != PersonId::HOME)
            .map(|(id, _)| Row::Person(id))
            .partition(|row| self.is_counted(*row));
        BlockRows {
            above: std::iter::once(Row::Person(PersonId::HOME))
                .chain(self.ordered(counted))
                .collect(),
            below: self.ordered(other),
        }
    }

    /// The where block's rows: the marked places, then the rest.
    pub fn where_rows(&self) -> BlockRows {
        let (counted, other): (Vec<Row>, Vec<Row>) = self
            .roster()
            .places()
            .map(|(id, _)| Row::Place(id))
            .partition(|row| self.is_counted(*row));
        BlockRows {
            above: self.ordered(counted),
            below: self.ordered(other),
        }
    }

    /// Every row focus can sit on, top to bottom through both blocks.
    pub fn focus_rows(&self) -> Vec<Row> {
        self.who_rows()
            .iter()
            .chain(self.where_rows().iter())
            .collect()
    }
}

/// The marks a lens sets: its group's members, or everyone for `0 all`.
pub fn base_marks(roster: &Roster, lens: Lens) -> Marks {
    match lens {
        Lens::Group(index) => roster
            .groups
            .get(index)
            .map_or_else(|| Marks::all(roster), Marks::of_group),
        Lens::All => Marks::all(roster),
    }
}
