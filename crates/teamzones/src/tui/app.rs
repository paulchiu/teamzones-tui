//! The TUI's state, and the pure queries the update and the view both read.

use chrono::{DateTime, NaiveDate, Offset, Utc};
use teamzones_core::clock::{Range, Slot, When};
use teamzones_core::config::{Config, PersonId, Roster};
use teamzones_core::copy::{Clause, Variant, focus_line, say_line};
use teamzones_core::question::{Marks, Row};

use super::keys::{Context, CopyAs, Sort};
use super::notice::Notice;

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
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Prompt(String),
    CopyMenu,
    SortMenu,
    Help(Context),
}

/// How people lanes are drawn: by band, or plain day and night.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lanes {
    Hours,
    Plain,
}

/// How the range is marked across the grid, cycled with `F2` under `TEAMZONES_DEBUG=1` until
/// one is picked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RangeStyle {
    /// The selection background on every in-range cell.
    Columns,
    /// The selection background on the ruler (the time axis row) only, and edge bars on every lane.
    Edges,
    /// Edge lines through every lane and answer row, with no fill.
    Lines,
    /// In-range cells underlined.
    Underline,
    /// Cells outside the range dimmed.
    Dim,
}

impl RangeStyle {
    const COUNT: usize = 5;

    /// The style's 1-based place in the `F2` cycle and its footer name.
    fn position(self) -> (usize, &'static str) {
        match self {
            Self::Columns => (1, "columns"),
            Self::Edges => (2, "edges"),
            Self::Lines => (3, "lines"),
            Self::Underline => (4, "underline"),
            Self::Dim => (5, "dim"),
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Columns => Self::Edges,
            Self::Edges => Self::Lines,
            Self::Lines => Self::Underline,
            Self::Underline => Self::Dim,
            Self::Dim => Self::Columns,
        }
    }

    /// The footer's name for it, such as `range style 2/5: edges`.
    pub fn label(self) -> String {
        let (index, name) = self.position();
        format!("range style {index}/{}: {name}", Self::COUNT)
    }

    /// The style to draw when the theme has no colour: those that need a background or a dim
    /// foreground fall back to edge lines.
    pub fn without_colour(self, mono: bool) -> Self {
        match self {
            Self::Columns | Self::Edges | Self::Dim if mono => Self::Lines,
            Self::Columns | Self::Edges | Self::Lines | Self::Underline | Self::Dim => self,
        }
    }
}

/// The one level of undo: the lens and marks a group switch dropped, until it expires.
#[derive(Clone, Debug, PartialEq)]
pub struct Undo {
    pub lens: Lens,
    pub marks: Marks,
    pub expires: DateTime<Utc>,
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
    /// Whether `TEAMZONES_DEBUG=1` turned on the debug keys.
    pub debug: bool,
    pub range_style: RangeStyle,
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
            debug: false,
            range_style: RangeStyle::Columns,
        }
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

    /// The marks differ from the lens's saved members as `+@UK` or `-Sam`, people first.
    pub fn drift(&self) -> Vec<String> {
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
            .chain(self.drift())
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// The say line: exactly what `y` copies.
    pub fn say_line(&self) -> String {
        say_line(self.roster(), &self.marks, self.when(), Variant::default())
    }

    /// The line one `c` menu variant copies.
    pub fn copy_as(&self, variant: CopyAs) -> String {
        let (roster, when) = (self.roster(), self.when());
        let shaped = |date, names, clause| Variant {
            date,
            names,
            clause,
        };
        match variant {
            CopyAs::Plain => self.say_line(),
            CopyAs::Date => say_line(
                roster,
                &self.marks,
                when,
                shaped(true, false, Clause::States),
            ),
            CopyAs::Names => say_line(
                roster,
                &self.marks,
                when,
                shaped(false, true, Clause::States),
            ),
            CopyAs::Zones => say_line(
                roster,
                &self.marks,
                when,
                shaped(false, false, Clause::Zones),
            ),
            CopyAs::Short => say_line(
                roster,
                &self.marks,
                when,
                shaped(false, false, Clause::Drop),
            ),
            CopyAs::ThisRow => focus_line(roster, self.focus, when),
            CopyAs::All => say_line(roster, &Marks::all(roster), when, Variant::default()),
        }
    }

    /// The copy-as dialog's rows, in key order: each format's key, the exact line it copies now,
    /// and a short description.
    pub fn copy_menu(&self) -> Vec<(char, String, String)> {
        let focused = match self.focus {
            Row::Person(id) => self.roster().person(id).name.clone(),
            Row::Place(id) => self.roster().place(id).name.clone(),
        };
        [
            ('y', CopyAs::Plain, "marked, ttz format".to_owned()),
            ('d', CopyAs::Date, "with the date".to_owned()),
            ('w', CopyAs::Names, "with names".to_owned()),
            (
                't',
                CopyAs::ThisRow,
                format!("for {focused}, their time first"),
            ),
            ('z', CopyAs::Zones, "with place zones".to_owned()),
            ('s', CopyAs::Short, "without the clause".to_owned()),
            ('a', CopyAs::All, "everyone".to_owned()),
        ]
        .into_iter()
        .map(|(key, variant, about)| (key, self.copy_as(variant), about))
        .collect()
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
