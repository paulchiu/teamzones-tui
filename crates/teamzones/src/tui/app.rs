//! The TUI's state, and the pure queries the update and the view both read.

use chrono::{DateTime, NaiveDate, Offset, Utc};
use teamzones_core::clock::{Range, Slot, When};
use teamzones_core::config::{Config, MapShow, PersonId, RangeStyle, Roster};
use teamzones_core::copy::{Format, Who, format_line};
use teamzones_core::question::{Marks, Row};

use super::fold::{Entry, fold};
use super::footer::MapKey;
use super::glyphs::Glyphs;
use super::keys::{Context, Sort};
use super::layout::{Areas, MapChoice, MapView, layout, map_view, room_for_map};
use super::map::{Camera, framing};
use super::matrix::Matrix;
use super::mouse::Drag;
use super::notice::Notice;
use super::picker::Picker;
use super::undo::Undo;
use crate::state::LensMarks;

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
    /// Map mode: the map's keys pan and zoom it.
    Map,
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
    /// What the map is framed on, kept until map mode resets it or the terminal is resized.
    pub camera: Camera,
    /// Whether the app holds mouse capture; `M` releases it for text selection.
    pub mouse: bool,
    /// Whether the list of the rows the focused `+N more` line folds is open.
    pub fold_open: bool,
    /// The left-button drag in progress, if any.
    pub drag: Option<Drag>,
    /// The config file's text the config was parsed from, or `None` when there is no file.
    pub text: Option<String>,
    /// What the file on disk holds, as far as the app knows: `text` once every edit is written.
    pub disk: Option<String>,
    /// When `text` is due on disk, while a close waits out its undo window unwritten.
    pub flush_at: Option<DateTime<Utc>>,
    /// The cell of the last left-button press and when it came on the loop's monotonic clock,
    /// for spotting a double click.
    pub last_click: Option<(u16, u16, chrono::TimeDelta)>,
    /// Whether the TUI was started with no config file and the last read still found none, so
    /// the footer offers `E` to create one.
    pub first_run: bool,
    /// The marks the state file kept for lenses other than the active one, by name, which a
    /// switch to that lens puts back.
    pub kept: Vec<LensMarks>,
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
        let app = Self {
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
            camera: Camera {
                lat: 0.0,
                lon: 0.0,
                zoom: 0,
            },
            mouse: true,
            fold_open: false,
            drag: None,
            text: None,
            disk: None,
            flush_at: None,
            last_click: None,
            first_run: false,
            kept: Vec::new(),
        };
        Self {
            camera: framing(&app),
            ..app
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
            first_run: text.is_none(),
            disk: text.clone(),
            text,
            notice,
            ..self
        }
    }

    /// The app opened on the group named `last` (or `all`), as the state file saved it, with the
    /// marks `stored` kept for each lens still in the config; a name no longer in the config
    /// keeps the first group.
    pub fn on_group(self, last: Option<&str>, stored: &[LensMarks]) -> Self {
        let kept = stored
            .iter()
            .filter(|entry| self.lens_named(&entry.lens).is_some())
            .cloned()
            .collect();
        let lens = last
            .and_then(|name| self.lens_named(name))
            .unwrap_or(self.lens);
        let app = Self { kept, lens, ..self }.entered();
        Self {
            camera: framing(&app),
            ..app
        }
    }

    /// The lens named `name`: `all`, or a configured group, matched case-insensitively.
    fn lens_named(&self, name: &str) -> Option<Lens> {
        if name.eq_ignore_ascii_case("all") {
            return Some(Lens::All);
        }
        self.roster()
            .groups
            .iter()
            .position(|group| group.name.eq_ignore_ascii_case(name))
            .map(Lens::Group)
    }

    /// The app with the marks kept for its lens taken back out of `kept`, or its lens's config
    /// marks when none are kept.
    pub fn entered(self) -> Self {
        let name = self.lens_name();
        let (taken, kept): (Vec<LensMarks>, Vec<LensMarks>) = self
            .kept
            .iter()
            .cloned()
            .partition(|entry| entry.lens.eq_ignore_ascii_case(&name));
        let base = base_marks(self.roster(), self.lens);
        let marks = match taken.first() {
            Some(entry) => resolve(self.roster(), entry, base),
            None => base,
        };
        Self {
            marks,
            kept,
            ..self
        }
    }

    /// The marks to keep for each lens on quit: those still kept, plus the active lens's when
    /// they differ from its config.
    pub fn remembered(&self) -> Vec<LensMarks> {
        let base = base_marks(self.roster(), self.lens);
        let active = (base.people != self.marks.people || base.places != self.marks.places)
            .then(|| names_of(self.roster(), &self.lens_name(), &self.marks));
        self.kept
            .iter()
            .filter(|entry| self.lens_named(&entry.lens).is_some())
            .cloned()
            .chain(active)
            .collect()
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

    /// The whole roster's people (with home's reference row while travelling) and places,
    /// which size the blocks.
    fn roster_size(&self) -> (u16, u16) {
        let count = |n: usize| u16::try_from(n).unwrap_or(u16::MAX);
        let roster = self.roster();
        (
            count(roster.people().count() + usize::from(roster.reference().is_some())),
            count(roster.places().count()),
        )
    }

    /// The screen's areas on a `cols` by `rows` terminal.
    pub fn areas_at(&self, cols: u16, rows: u16) -> Areas {
        let choice = match self.map_view_at(cols, rows) {
            MapView::InPlace => MapChoice::Always,
            MapView::Hidden | MapView::Overlay => MapChoice::Hidden,
        };
        let (people, places) = self.roster_size();
        layout(cols, rows, people, places, choice)
    }

    /// Where the map goes at the size the terminal last reported.
    pub fn map_view(&self) -> MapView {
        self.map_view_at(self.size.0, self.size.1)
    }

    /// Home's mark: `✈` while travelling, else `⌂`.
    pub fn home_mark(&self, glyphs: &Glyphs) -> &'static str {
        if self.roster().reference().is_some() {
            glyphs.here
        } else {
            glyphs.home
        }
    }

    /// The roster as it stands now, with home moved while travel holds.
    pub fn roster(&self) -> &Roster {
        self.config.roster_at(self.now)
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
        let focus = self.focused_row().unwrap_or(Row::Person(PersonId::HOME));
        format_line(self.roster(), &self.marks, focus, self.when(), format)
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
        let focused = match self.focused_row().unwrap_or(Row::Person(PersonId::HOME)) {
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

    /// The who block's lines unfolded: home, its reference while travelling, the counted
    /// people, the answer row, then the rest.
    fn who_lines(&self) -> Vec<Entry> {
        let BlockRows { above, below } = self.who_rows();
        let reference = self.roster().reference().map(|_| Entry::Reference);
        let mut above = above.into_iter().map(Entry::Row);
        above
            .next()
            .into_iter()
            .chain(reference)
            .chain(above)
            .chain(std::iter::once(Entry::Answer))
            .chain(below.into_iter().map(Entry::Row))
            .collect()
    }

    /// The where block's lines unfolded: the counted places, the answer row, then the rest.
    fn where_lines(&self) -> Vec<Entry> {
        let BlockRows { above, below } = self.where_rows();
        above
            .into_iter()
            .map(Entry::Row)
            .chain(std::iter::once(Entry::Answer))
            .chain(below.into_iter().map(Entry::Row))
            .collect()
    }

    /// Each block's lines as drawn on a `cols` by `rows` terminal, folded to fit.
    pub fn entries_at(&self, cols: u16, rows: u16) -> (Vec<Entry>, Vec<Entry>) {
        let areas = self.areas_at(cols, rows);
        let lines = areas.columns.lines().max(1);
        let room = |height: u16, borders: u16| usize::from(height.saturating_sub(borders) / lines);
        (
            fold(self.who_lines(), room(areas.who.height, 1)),
            fold(self.where_lines(), room(areas.place.height, 2)),
        )
    }

    /// Each block's lines at the terminal's last size, unfolded before the first resize.
    pub fn entries(&self) -> (Vec<Entry>, Vec<Entry>) {
        match self.size {
            (0, 0) => (self.who_lines(), self.where_lines()),
            (cols, rows) => self.entries_at(cols, rows),
        }
    }

    /// Every row focus can sit on, top to bottom through both blocks: each row drawn, and for a
    /// `+N more` line the first row it folds, which stands for it.
    pub fn focus_rows(&self) -> Vec<Row> {
        let (who, place) = self.entries();
        who.iter()
            .chain(&place)
            .filter_map(|entry| entry.rows().first().copied())
            .collect()
    }

    /// The row focus is on, or none on a `+N more` line, which stands for no single row.
    pub fn focused_row(&self) -> Option<Row> {
        self.focused_more().is_none().then_some(self.focus)
    }

    /// The row standing for the `+N more` line that folds `row`, if a line folds it.
    pub fn fold_holding(&self, row: Row) -> Option<Row> {
        let (who, place) = self.entries();
        who.iter().chain(&place).find_map(|entry| match entry {
            Entry::More(_) if entry.rows().contains(&row) => entry.rows().first().copied(),
            Entry::More(_) | Entry::Row(_) | Entry::Reference | Entry::Answer => None,
        })
    }

    /// The `+N more` line focus is on, if any.
    pub fn focused_more(&self) -> Option<Entry> {
        let (who, place) = self.entries();
        who.into_iter().chain(place).find(|entry| {
            matches!(entry, Entry::More(_)) && entry.rows().first() == Some(&self.focus)
        })
    }
}

/// The marks `entry` names, by name in `roster`, over `base`'s rule; names the roster no
/// longer has are dropped.
fn resolve(roster: &Roster, entry: &LensMarks, base: Marks) -> Marks {
    let named = |names: &[String], name: &str| names.iter().any(|n| n.eq_ignore_ascii_case(name));
    Marks {
        people: roster
            .people()
            .filter(|(_, person)| named(&entry.people, &person.name))
            .map(|(id, _)| id)
            .collect(),
        places: roster
            .places()
            .filter(|(_, place)| named(&entry.places, &place.name))
            .map(|(id, _)| id)
            .collect(),
        ..base
    }
}

/// `marks` by name in `roster`, in config order, as the state file keeps them for `lens`.
fn names_of(roster: &Roster, lens: &str, marks: &Marks) -> LensMarks {
    LensMarks {
        lens: lens.to_owned(),
        people: roster
            .people()
            .filter(|(id, _)| marks.people.contains(id))
            .map(|(_, person)| person.name.clone())
            .collect(),
        places: roster
            .places()
            .filter(|(id, _)| marks.places.contains(id))
            .map(|(_, place)| place.name.clone())
            .collect(),
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
