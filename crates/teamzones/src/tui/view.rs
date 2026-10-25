//! The pure view: draws the app into a frame, deciding nothing the core has not decided.

use chrono::{DateTime, Offset, TimeDelta, Timelike, Utc};
use chrono_tz::Tz;
use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Widget};
use teamzones_core::clock::{Range, Slot, When};
use teamzones_core::config::{PersonId, Rule};
use teamzones_core::fit::{day_start, slot_fits};
use teamzones_core::question::{Row, evaluate, person_band};
use teamzones_core::schedule::{Band, Level, band, is_day};

use super::app::{App, Cursor, Lanes, Mode, without_colour};
use super::fold::Entry;
use super::glyphs::Glyphs;
use super::layout::{Areas, MapView, too_small};
use super::say::{detail, duration_text, local_text, summary};
use super::theme::{Role, Theme};
use super::update::context;
use super::{footer, help, keys};
use teamzones_core::config::RangeStyle;

const DAY_SECONDS: i64 = 24 * 60 * 60;

/// How the screen looks: the resolved theme and the glyph set.
#[derive(Clone, Copy)]
pub struct Look<'a> {
    pub theme: &'a Theme,
    pub glyphs: &'static Glyphs,
}

/// The where block's title, which a click hit-tests as the sort key.
pub const WHERE_TITLE: &str = " where ";

/// The say panel's copy hint, key then label, which a click hit-tests as `y`.
pub const COPY_HINT: (&str, &str) = (" y", " copy ");

/// The display width of `text`, in terminal columns.
pub fn width_of(text: &str) -> usize {
    Span::raw(text).width()
}

/// `text` cut to `max` columns, ending in `mark` when anything was cut.
pub fn truncate_with_mark(text: &str, max: usize, mark: &str) -> String {
    if width_of(text) <= max {
        return text.to_owned();
    }
    let room = max.saturating_sub(width_of(mark));
    let mut kept = String::new();
    for c in text.chars() {
        if width_of(&kept) + width_of(&c.to_string()) > room {
            break;
        }
        kept.push(c);
    }
    kept + mark
}

/// Writes `text` at (`x`, `y`) without passing `area`'s right edge; returns the column after it.
pub fn put(buf: &mut Buffer, area: Rect, x: u16, y: u16, text: &str, style: Style) -> u16 {
    let area = area.intersection(buf.area);
    if y < area.top() || y >= area.bottom() || x < area.left() || x >= area.right() {
        return x;
    }
    buf.set_stringn(x, y, text, usize::from(area.right() - x), style)
        .0
}

fn patch(buf: &mut Buffer, area: Rect, x: u16, y: u16, style: Style) {
    if area.contains((x, y).into())
        && let Some(cell) = buf.cell_mut((x, y))
    {
        cell.set_style(style);
    }
}

fn set_char(buf: &mut Buffer, area: Rect, x: u16, y: u16, c: char, style: Style) {
    if area.contains((x, y).into())
        && let Some(cell) = buf.cell_mut((x, y))
    {
        cell.set_char(c).set_style(style);
    }
}

/// The track's geometry: where it starts, how many cells, and the instant each cell covers.
///
/// The cells split the viewed day evenly, so cell `index` is the same instant in every row.
pub struct Track {
    pub x: u16,
    pub cells: u16,
    /// The label's width: the columns before the track, or the whole line when stacked.
    pub label: u16,
    /// How far below a row's label line its lane sits: 0 side by side, 1 stacked.
    pub dy: u16,
    start: DateTime<Utc>,
    when: When,
    /// How the range is marked, already fallen back for a theme without colour.
    marking: RangeStyle,
}

/// The grid rows a range style can mark: the ruler, the track, a lane or an answer row.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Surface {
    Ruler,
    Track,
    Lane,
    Answer,
}

impl Track {
    /// Cell `index`'s style on `surface`: `base` plus however this range style marks it.
    fn marked(&self, theme: &Theme, surface: Surface, index: u16, base: Style) -> Style {
        let selected = self.selected(index);
        let shade = || base.patch(theme.selection);
        match self.marking {
            RangeStyle::Columns if selected => shade(),
            RangeStyle::Edges if selected => match surface {
                Surface::Ruler => shade(),
                Surface::Track | Surface::Lane | Surface::Answer => base,
            },
            RangeStyle::Underline if selected => match surface {
                Surface::Ruler | Surface::Lane | Surface::Answer => {
                    base.add_modifier(Modifier::UNDERLINED)
                }
                Surface::Track => base,
            },
            RangeStyle::Focus if !selected => match surface {
                Surface::Lane | Surface::Answer => base.add_modifier(Modifier::DIM),
                Surface::Ruler | Surface::Track => base,
            },
            RangeStyle::Columns
            | RangeStyle::Edges
            | RangeStyle::Lines
            | RangeStyle::Underline
            | RangeStyle::Focus => base,
        }
    }

    /// Draws the edge glyphs styles 2 and 3 put just outside the range on `surface`'s row `y`.
    fn edges(&self, buf: &mut Buffer, area: Rect, y: u16, look: Look, surface: Surface) {
        let glyphs = look.glyphs;
        let pair = match self.marking {
            RangeStyle::Edges => match surface {
                Surface::Lane => Some((glyphs.edge_before, glyphs.edge_after)),
                Surface::Ruler | Surface::Track | Surface::Answer => None,
            },
            RangeStyle::Lines => match surface {
                Surface::Lane | Surface::Answer => Some((glyphs.edge_line, glyphs.edge_line)),
                Surface::Ruler | Surface::Track => None,
            },
            RangeStyle::Columns | RangeStyle::Underline | RangeStyle::Focus => None,
        };
        let Some((before, after)) = pair else { return };
        let mut selected = (0..self.cells).filter(|index| self.selected(*index));
        let Some(first) = selected.next() else { return };
        let last = selected.next_back().unwrap_or(first);
        let style = look.theme.fg(Role::Accent);
        if first > 0 {
            set_char(buf, area, self.x + first - 1, y, before, style);
        }
        if last + 1 < self.cells {
            set_char(buf, area, self.x + last + 1, y, after, style);
        }
    }

    fn cell_start(&self, index: u16) -> DateTime<Utc> {
        let seconds = i64::from(index) * DAY_SECONDS / i64::from(self.cells.max(1));
        self.start + TimeDelta::seconds(seconds)
    }

    /// The grid slot cell `index` starts in, which decides its state.
    pub fn first_slot(&self, index: u16) -> Slot {
        Slot::containing(self.cell_start(index))
    }

    /// The grid slots cell `index` overlaps, first to last.
    pub fn slots(&self, index: u16) -> impl Iterator<Item = Slot> {
        let first = Slot::containing(self.cell_start(index));
        let last = Slot::containing(self.cell_start(index + 1) - TimeDelta::seconds(1));
        (0..=last.since(first).max(0)).map(move |step| first.offset(step))
    }

    /// Whether `key` of the local time in `tz` changes inside cell `index`, so the cell holds
    /// that boundary (a local midnight, say).
    fn crosses<K: PartialEq>(&self, index: u16, tz: Tz, key: impl Fn(DateTime<Tz>) -> K) -> bool {
        key(self.last_second(index, tz)) != key(self.last_second(index + 1, tz))
    }

    /// The local time in `tz` one second before cell `index` starts.
    fn last_second(&self, index: u16, tz: Tz) -> DateTime<Tz> {
        (self.cell_start(index) - TimeDelta::seconds(1)).with_timezone(&tz)
    }

    /// Whether cell `index` overlaps the range, or holds the cursor's instant.
    pub fn selected(&self, index: u16) -> bool {
        let (from, to) = (self.cell_start(index), self.cell_start(index + 1));
        match self.when {
            When::At(instant) => from <= instant && instant < to,
            When::Over(range) => from < range.end().start() && range.start().start() < to,
        }
    }

    /// Hours between ruler labels: the fewest that keep labels at least ten columns apart.
    fn label_every(&self) -> u32 {
        [1, 2, 3, 4, 6, 12]
            .into_iter()
            .find(|hours| u32::from(self.cells) * hours >= 24 * 10)
            .unwrap_or(12)
    }

    fn columns(&self) -> impl Iterator<Item = (u16, u16)> + '_ {
        (0..self.cells).map(|index| (index, self.x + index))
    }
}

/// The screen's areas for this frame size and roster.
pub fn areas(app: &App, size: Rect) -> Areas {
    app.areas_at(size.width, size.height)
}

/// Where the map is drawn on a screen of `size`: in place, inside the overlay's border, or
/// nowhere.
pub fn map_area(app: &App, size: Rect) -> Option<Rect> {
    match app.map_view_at(size.width, size.height) {
        MapView::InPlace => areas(app, size).map.map(|area| size.intersection(area)),
        MapView::Overlay => Some(map_overlay_area(size).inner(ratatui::layout::Margin::new(1, 1))),
        MapView::Hidden => None,
    }
}

/// The map overlay's box: centred, up to twelve map rows inside its border.
pub fn map_overlay_area(size: Rect) -> Rect {
    let width = size.width.saturating_sub(4).min(120);
    let height = size.height.saturating_sub(4).min(14);
    Rect::new(
        size.x + (size.width - width) / 2,
        size.y + (size.height - height) / 2,
        width,
        height,
    )
}

fn map_overlay(buf: &mut Buffer, size: Rect, app: &App, look: Look) {
    let theme = look.theme;
    let area = map_overlay_area(size);
    Clear.render(area, buf);
    let esc = if app.mode == Mode::Map {
        " done "
    } else {
        " close "
    };
    let block = Block::bordered()
        .border_set(look.glyphs.rounded)
        .border_style(theme.fg(Role::Accent))
        .title(Span::styled(
            " map ",
            theme.fg(Role::Text).add_modifier(Modifier::BOLD),
        ))
        .title_bottom(
            Line::from(vec![
                Span::styled(" esc", theme.fg(Role::Accent)),
                Span::styled(esc, theme.fg(Role::Dim)),
            ])
            .right_aligned(),
        );
    let inner = block.inner(area);
    block.render(area, buf);
    super::map::draw(buf, inner, app, look);
}

/// The track for `areas`: where its cells sit and the instant each covers.
pub fn track_for(app: &App, areas: &Areas, look: Look) -> Track {
    Track {
        x: areas.columns.track_x(),
        cells: areas.columns.track(),
        label: areas.columns.label(),
        dy: areas.columns.lines() - 1,
        start: day_start(app.day(), app.roster().axis()),
        when: app.when(),
        marking: without_colour(app.range_style, look.theme.is_mono()),
    }
}

/// Draws the whole screen.
pub fn draw(frame: &mut Frame, app: &App, look: Look) {
    let size = frame.area();
    if too_small(size.width, size.height) {
        let text = "terminal too small";
        let y = size.y + size.height / 2;
        let x = size.x
            + size
                .width
                .saturating_sub(u16::try_from(width_of(text)).unwrap_or(0))
                / 2;
        put(
            frame.buffer_mut(),
            size,
            x,
            y,
            text,
            look.theme.fg(Role::Dim),
        );
        return;
    }
    let areas = areas(app, size);
    let clip = |area: Rect| size.intersection(area);
    let track = track_for(app, &areas, look);
    let buf = frame.buffer_mut();
    header(buf, clip(areas.header), app, look);
    if let Some(area) = areas.map {
        super::map::draw(buf, clip(area), app, look);
    }
    slider(buf, clip(areas.slider), app, look, &track);
    let (who_entries, where_entries) = app.entries_at(size.width, size.height);
    who(buf, clip(areas.who), app, look, &track, &who_entries);
    places(buf, clip(areas.place), app, look, &track, &where_entries);
    detail_line(buf, clip(areas.detail), app, look);
    say_panel(buf, clip(areas.say), app, look);
    notice(buf, clip(areas.notice), app, look);
    let map_key = app.map_key_at(size.width, size.height);
    footer_row(buf, clip(areas.footer), app, look, map_key);
    if app.map_view_at(size.width, size.height) == MapView::Overlay {
        map_overlay(buf, size, app, look);
    }
    let folded = who_entries
        .iter()
        .chain(&where_entries)
        .find_map(|entry| match entry {
            Entry::More(folded) if app.fold_open && entry.rows().first() == Some(&app.focus) => {
                Some(folded)
            }
            Entry::More(_) | Entry::Row(_) | Entry::Reference | Entry::Answer => None,
        });
    if let Some(folded) = folded {
        fold_overlay(buf, size, app, look, folded);
    }
    match &app.mode {
        Mode::Help(from, _) => help_overlay(buf, size, *from, look),
        Mode::Matrix(matrix) => super::matrix::draw(buf, size, app, matrix, look),
        Mode::Picker(picker) => super::picker::draw(buf, size, app, picker, look),
        Mode::CopyMenu => copy_dialog(buf, size, app, look),
        Mode::DriftMenu => drift_menu(buf, size, app, look),
        Mode::Normal
        | Mode::Prompt(_)
        | Mode::NewGroup(_)
        | Mode::SortMenu
        | Mode::AddMenu(_)
        | Mode::Map => {}
    }
}

/// The header's now, as in `Sat 24 Oct 9:04am AEDT`.
fn now_text(app: &App) -> String {
    let now = app.now.with_timezone(&app.roster().axis());
    format!(
        "{} {} {}",
        now.format("%a %-d %b"),
        teamzones_core::copy::clock_text(&now),
        now.format("%Z")
    )
}

/// The header columns the now piece's text covers, after the app name and its gap.
pub fn header_now_columns(app: &App) -> std::ops::Range<u16> {
    let start = u16::try_from(width_of("teamzones   now ")).unwrap_or(0);
    let width = u16::try_from(width_of(&now_text(app))).unwrap_or(0);
    start..start + width
}

fn header(buf: &mut Buffer, area: Rect, app: &App, look: Look) {
    let (theme, glyphs) = (look.theme, look.glyphs);
    let axis = app.roster().axis();
    let days = (app.day() - app.today()).num_days();
    let mut pieces: Vec<Vec<Span>> = vec![
        vec![Span::styled(
            "teamzones",
            theme.fg(Role::Accent).add_modifier(Modifier::BOLD),
        )],
        vec![
            Span::styled("   now ", theme.fg(Role::Dim)),
            Span::styled(now_text(app), theme.fg(Role::Dim)),
        ],
    ];
    if days != 0 {
        let viewed = app.when().start().with_timezone(&axis);
        pieces.push(vec![
            Span::styled("   viewing ", theme.fg(Role::Dim)),
            Span::styled(viewed.format("%a %-d %b").to_string(), theme.fg(Role::Text)),
            Span::styled(format!(" {days:+}d"), theme.fg(Role::Dim)),
        ]);
    }
    let roster = app.roster();
    let home = roster.person(PersonId::HOME);
    let right = vec![
        Span::styled(
            format!("{} ", app.home_mark(glyphs)),
            theme.fg(Role::Marker),
        ),
        Span::styled(home.city.clone(), theme.fg(Role::Text)),
        Span::styled(
            format!(" {}", app.when().start().with_timezone(&axis).format("%Z")),
            theme.fg(Role::Dim),
        ),
    ];
    let span_width = |spans: &[Span]| spans.iter().map(Span::width).sum::<usize>();
    let cols = usize::from(area.width);
    let mut right = Some(right);
    while span_width(&pieces.concat()) + right.as_deref().map_or(0, |r| span_width(r) + 1) > cols {
        if right.take().is_none() && pieces.len() > 1 {
            pieces.pop();
        } else if pieces.len() <= 1 {
            break;
        }
    }
    Line::from(pieces.concat()).render(area, buf);
    if let Some(right) = right {
        let width = u16::try_from(span_width(&right)).unwrap_or(area.width);
        let x = area.right().saturating_sub(width);
        Line::from(right).render(Rect::new(x, area.y, width, 1), buf);
    }
}

fn hour_label(hour: u32) -> String {
    match hour {
        0 => "12a".to_owned(),
        12 => "12p".to_owned(),
        h if h < 12 => format!("{h}a"),
        h => format!("{}p", h - 12),
    }
}

fn slider(buf: &mut Buffer, area: Rect, app: &App, look: Look, track: &Track) {
    let (theme, glyphs) = (look.theme, look.glyphs);
    let axis = app.roster().axis();
    let (ruler_label_y, ruler_y) = (area.y, area.y + track.dy);
    let (track_label_y, track_y) = (ruler_y + 1, ruler_y + 1 + track.dy);
    let viewed = app.when().start().with_timezone(&axis);
    let x = put(
        buf,
        area,
        area.x,
        ruler_label_y,
        &viewed.format("%a %-d %b").to_string(),
        theme.fg(Role::Text).add_modifier(Modifier::BOLD),
    );
    put(
        buf,
        area,
        x + 1,
        ruler_label_y,
        &viewed.format("%Z").to_string(),
        theme.fg(Role::Dim),
    );
    let every = track.label_every();
    for (index, column) in track.columns() {
        set_char(buf, area, column, ruler_y, ' ', Style::new());
        let fitting = track
            .slots(index)
            .filter(|slot| slot_fits(app.roster(), &app.marks, *slot))
            .count();
        let total = track.slots(index).count();
        let (mark, role) = match fitting {
            0 => (glyphs.track, Role::Dim),
            n if n == total => (glyphs.track_fit, Role::Ok),
            _ => (glyphs.track_partial, Role::Ok),
        };
        set_char(buf, area, column, track_y, mark, theme.fg(role));
    }
    let hours: Vec<(u32, u16)> = track
        .columns()
        .filter(|(index, _)| {
            track.crosses(*index, axis, |local| {
                (local.date_naive(), local.hour(), local.offset().fix())
            })
        })
        .map(|(index, column)| (track.last_second(index + 1, axis).hour(), column))
        .collect();
    for (_, column) in hours.iter().filter(|(hour, _)| hour % every != 0) {
        set_char(
            buf,
            area,
            *column,
            ruler_y,
            glyphs.tick,
            theme.fg(Role::Dim),
        );
    }
    for &(hour, column) in hours.iter().filter(|(hour, _)| hour % every == 0) {
        for (offset, c) in (0..).zip(hour_label(hour).chars()) {
            set_char(buf, area, column + offset, ruler_y, c, theme.fg(Role::Dim));
        }
    }
    for (index, column) in track.columns() {
        patch(
            buf,
            area,
            column,
            ruler_y,
            track.marked(theme, Surface::Ruler, index, Style::new()),
        );
    }
    let selected: Vec<u16> = track
        .columns()
        .filter(|(index, _)| track.selected(*index))
        .map(|(_, column)| column)
        .collect();
    let mark = theme.fg(Role::Accent).add_modifier(Modifier::BOLD);
    let mark = selected.first().map_or(mark, |first| {
        track.marked(theme, Surface::Track, first - track.x, mark)
    });
    match (app.shown_range(), selected.first(), selected.last()) {
        (Some(_), Some(first), Some(last)) if first != last => {
            for column in &selected {
                patch(buf, area, *column, track_y, mark);
            }
            set_char(buf, area, *first, track_y, glyphs.range_start, mark);
            set_char(buf, area, *last, track_y, glyphs.range_end, mark);
        }
        (_, Some(first), _) => set_char(buf, area, *first, track_y, glyphs.cursor, mark),
        (_, None, _) => {}
    }
    let label = match (app.shown_range(), app.cursor) {
        (Some(range), _) => (
            local_text(When::Over(range), axis),
            duration_text(range.end().since(range.start())),
        ),
        (None, Cursor::Now) => (local_text(app.when(), axis), "now".to_owned()),
        (None, Cursor::At(_)) => (local_text(app.when(), axis), String::new()),
    };
    let x = put(
        buf,
        area,
        area.x,
        track_label_y,
        &format!("{} ", glyphs.pointer),
        theme.fg(Role::Accent),
    );
    let x = put(
        buf,
        area,
        x,
        track_label_y,
        &format!("{} {}", label.0, viewed.format("%Z")),
        theme.fg(Role::Accent).add_modifier(Modifier::BOLD),
    );
    put(
        buf,
        area,
        x + 1,
        track_label_y,
        &label.1,
        theme.fg(Role::Dim),
    );
    if app.day() != app.today() && track.label > 0 {
        put(
            buf,
            area,
            area.x + track.label - 1,
            track_label_y,
            glyphs.back,
            theme.fg(Role::Dim),
        );
    }
}

/// Draws one lane: a glyph and role per cell, the selection, then the local weekday tags.
fn lane(
    buf: &mut Buffer,
    area: Rect,
    y: u16,
    look: Look,
    track: &Track,
    tz: Tz,
    cells: &[(char, Role)],
) {
    let theme = look.theme;
    for ((index, column), (glyph, role)) in track.columns().zip(cells) {
        let style = track.marked(theme, Surface::Lane, index, theme.fg(*role));
        set_char(buf, area, column, y, *glyph, style);
    }
    for (index, column) in track.columns() {
        if track.crosses(index, tz, |local| local.date_naive()) {
            let end = area.right().min(track.x + track.cells);
            let tag = track.last_second(index + 1, tz).format("%a").to_string();
            put(
                buf,
                Rect::new(column, y, end.saturating_sub(column), 1).intersection(area),
                column,
                y,
                &tag,
                theme.fg(Role::Text).add_modifier(Modifier::BOLD),
            );
        }
    }
    track.edges(buf, area, y, look, Surface::Lane);
}

fn band_role(band: Band) -> Role {
    match band {
        Band::Night => Role::Night,
        Band::Day => Role::Day,
        Band::Morning | Band::Evening => Role::Dawn,
        Band::Working => Role::Work,
        Band::Off => Role::Dim,
    }
}

fn level_role(level: Level) -> Role {
    match level {
        Level::Closed => Role::Quiet,
        Level::Open => Role::Dim,
        Level::Busy => Role::Warn,
    }
}

/// A row's label cells: the mark, the name, an optional middle column and the right-aligned text.
struct Label<'a> {
    mark: &'a str,
    mark_role: Role,
    name: &'a str,
    name_width: usize,
    middle: Option<String>,
    middle_width: usize,
    right: Vec<String>,
    right_style: Style,
    counted: bool,
    focused: bool,
}

fn label(buf: &mut Buffer, area: Rect, y: u16, width: u16, look: Look, label: &Label) {
    let theme = look.theme;
    let bounds = Rect::new(area.x, y, width.saturating_sub(1), 1).intersection(area);
    let base = if label.focused {
        theme.selection
    } else {
        Style::new()
    };
    for x in bounds.left()..bounds.right() {
        set_char(buf, bounds, x, y, ' ', base);
    }
    put(
        buf,
        bounds,
        area.x,
        y,
        label.mark,
        theme.fg(label.mark_role).patch(base),
    );
    let name_role = if label.counted { Role::Text } else { Role::Dim };
    let mut name_style = theme.fg(name_role).patch(base);
    if label.focused {
        name_style = name_style.add_modifier(Modifier::BOLD);
    }
    let name_x = area.x + 2;
    put(buf, bounds, name_x, y, label.name, name_style);
    let mut left_end = usize::from(name_x) + label.name_width + 1;
    if let Some(middle) = &label.middle {
        let x = u16::try_from(left_end).unwrap_or(u16::MAX);
        put(buf, bounds, x, y, middle, theme.fg(Role::Dim).patch(base));
        left_end += label.middle_width + 1;
    }
    let room = usize::from(bounds.right()).saturating_sub(left_end);
    let text = label
        .right
        .iter()
        .find(|candidate| width_of(candidate) <= room)
        .cloned()
        .unwrap_or_else(|| {
            truncate_with_mark(label.right.last().map_or("", String::as_str), room, "")
        });
    let x = bounds
        .right()
        .saturating_sub(u16::try_from(width_of(&text)).unwrap_or(0));
    put(buf, bounds, x, y, &text, label.right_style.patch(base));
}

/// The right-hand time text, widest first: with the zone, without it, then without minutes.
fn time_candidates(app: &App, tz: Tz) -> Vec<String> {
    let when = app.when();
    let local = when.start().with_timezone(&tz);
    let weekday = if local.date_naive() == app.day() {
        String::new()
    } else {
        format!("{} ", local.format("%a"))
    };
    let text = format!("{weekday}{}", local_text(when, tz));
    let hours_only: String = {
        let mut out = String::new();
        let mut chars = text.chars().peekable();
        while let Some(c) = chars.next() {
            if c == ':' {
                while chars.peek().is_some_and(char::is_ascii_digit) {
                    chars.next();
                }
            } else {
                out.push(c);
            }
        }
        out
    };
    vec![format!("{text} {}", local.format("%Z")), text, hours_only]
}

fn person_cells(app: &App, track: &Track, look: Look, id: PersonId) -> Vec<(char, Role)> {
    let roster = app.roster();
    let person = roster.person(id);
    track
        .columns()
        .map(|(index, _)| {
            let slot = track
                .slots(index)
                .next()
                .unwrap_or(Slot::containing(track.start));
            let band = match app.lanes {
                Lanes::Hours => person_band(roster, id, slot),
                Lanes::Plain if is_day(person.location, roster.daylight, slot) => Band::Day,
                Lanes::Plain => Band::Night,
            };
            (look.glyphs.band(band), band_role(band))
        })
        .collect()
}

/// The city column: as wide as the longest city, cut only so every row keeps its time without
/// the zone.
fn city_width(app: &App, label_width: usize, names: usize) -> usize {
    let roster = app.roster();
    let longest = name_width(roster.people().map(|(_, p)| p.city.as_str()));
    let time = roster
        .people()
        .map(|(_, person)| width_of(&time_candidates(app, person.location.tz)[1]))
        .max()
        .unwrap_or(0);
    // The label keeps its last column free, and the mark, name and city each end with a gap.
    let room = label_width.saturating_sub(1 + 2 + names + 1 + 1 + time);
    longest.min(room)
}

fn name_width<'a>(names: impl Iterator<Item = &'a str>) -> usize {
    names.map(width_of).max().unwrap_or(0)
}

/// Draws a block's answer row: the rule and count, then each cell's fraction of counted rows.
fn answer(
    buf: &mut Buffer,
    area: Rect,
    y: u16,
    look: Look,
    track: &Track,
    word: &str,
    result: Result<(usize, usize, Vec<usize>), String>,
) {
    let (theme, glyphs) = (look.theme, look.glyphs);
    let label_width = track.label;
    put(buf, area, area.x, y, glyphs.answer, theme.fg(Role::Accent));
    put(buf, area, area.x + 2, y, word, theme.fg(Role::Dim));
    match result {
        Err(why) => {
            put(
                buf,
                area,
                area.x + label_width.saturating_sub(3),
                y,
                "--",
                theme.fg(Role::Dim),
            );
            let room = usize::from(area.right().saturating_sub(track.x + 1));
            put(
                buf,
                area,
                track.x + 1,
                y + track.dy,
                &truncate_with_mark(&why, room, ""),
                theme.fg(Role::Dim),
            );
        }
        Ok((qualifying, counted, cells)) => {
            let count = format!("{qualifying}/{counted}");
            let all = qualifying == counted;
            let style = theme
                .fg(if all { Role::Ok } else { Role::Dim })
                .add_modifier(Modifier::BOLD);
            let x = (area.x + label_width)
                .saturating_sub(1)
                .saturating_sub(u16::try_from(count.len()).unwrap_or(0));
            put(buf, area, x, y, &count, style);
            let cell_glyphs: Vec<(char, Role)> = cells
                .iter()
                .map(|k| match *k {
                    0 => (glyphs.answer_none, Role::Dim),
                    k if k >= counted => (glyphs.answer_all, Role::Ok),
                    k => {
                        let seventh = (k * 7 * 2 + counted) / (counted * 2);
                        (glyphs.answer_fill[seventh.clamp(1, 7) - 1], Role::Dim)
                    }
                })
                .collect();
            for ((index, column), (glyph, role)) in track.columns().zip(cell_glyphs) {
                let style = track.marked(theme, Surface::Answer, index, theme.fg(role));
                set_char(buf, area, column, y + track.dy, glyph, style);
            }
            track.edges(buf, area, y + track.dy, look, Surface::Answer);
        }
    }
}

/// How many counted rows of one block qualify throughout each cell.
fn cell_counts(
    app: &App,
    track: &Track,
    pick: impl Fn(&teamzones_core::question::Answer) -> usize,
) -> Vec<usize> {
    track
        .columns()
        .map(|(index, _)| {
            track
                .slots(index)
                .map(|slot| {
                    pick(&evaluate(
                        app.roster(),
                        &app.marks,
                        When::Over(Range::single(slot)),
                    ))
                })
                .min()
                .unwrap_or(0)
        })
        .collect()
}

fn legend(items: &[(char, Role, &str)], look: Look) -> Line<'static> {
    let spans: Vec<Span<'static>> = items
        .iter()
        .flat_map(|(glyph, role, word)| {
            [
                Span::styled(format!(" {glyph}"), look.theme.fg(*role)),
                Span::styled(format!(" {word} "), look.theme.fg(Role::Dim)),
            ]
        })
        .collect();
    Line::from(spans)
}

/// What a piece of the who strip is, which a click on it stands for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Piece {
    Title,
    /// A group's number and name, by its digit (`0` for all).
    Group(char),
    Drift,
    NewGroup,
    Matrix,
    Gap,
    /// The people legend at the border's right, which a click hit-tests as `b`.
    Legend,
}

/// The who strip's pieces: `who`, each numbered group with the active one marked and its drift,
/// then `+` (or its name prompt) and the matrix's `⋯`.
pub fn strip_pieces(app: &App, look: Look) -> Vec<(Piece, Span<'static>)> {
    let theme = look.theme;
    let mut pieces = vec![(
        Piece::Title,
        Span::styled(" who ", theme.fg(Role::Text).add_modifier(Modifier::BOLD)),
    )];
    let active = |index: Option<usize>| match (app.lens, index) {
        (super::app::Lens::Group(lens), Some(index)) => lens == index,
        (super::app::Lens::All, None) => true,
        (super::app::Lens::Group(_), None) | (super::app::Lens::All, Some(_)) => false,
    };
    let names = app
        .roster()
        .groups
        .iter()
        .take(9)
        .enumerate()
        .map(|(index, group)| (index + 1, group.name.clone(), Some(index)))
        .chain(std::iter::once((0, "all".to_owned(), None)));
    for (number, name, index) in names {
        let digit = char::from_digit(u32::try_from(number).unwrap_or(0), 10).unwrap_or('0');
        let group = Piece::Group(digit);
        pieces.push((
            group,
            Span::styled(format!("  {number} "), theme.fg(Role::Dim)),
        ));
        if active(index) {
            let style = theme
                .fg(Role::Accent)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED);
            pieces.push((group, Span::styled(name, style)));
            let drift = app.drift();
            if !drift.is_empty() {
                let text = format!(" {}", drift.join(" "));
                pieces.push((Piece::Drift, Span::styled(text, theme.fg(Role::Warn))));
            }
        } else {
            pieces.push((group, Span::styled(name, theme.fg(Role::Dim))));
        }
    }
    let adding = match &app.mode {
        Mode::NewGroup(input) => Span::styled(format!("  + {input}_"), theme.fg(Role::Accent)),
        Mode::Normal
        | Mode::Prompt(_)
        | Mode::CopyMenu
        | Mode::SortMenu
        | Mode::Help(..)
        | Mode::Matrix(_)
        | Mode::AddMenu(_)
        | Mode::Picker(_)
        | Mode::DriftMenu
        | Mode::Map => Span::styled("  +", theme.fg(Role::Dim)),
    };
    pieces.push((Piece::NewGroup, adding));
    let matrix = format!("  {}", look.glyphs.text("⋯"));
    pieces.push((Piece::Matrix, Span::styled(matrix, theme.fg(Role::Dim))));
    pieces.push((Piece::Gap, Span::raw("  ")));
    pieces
}

/// The drift menu's two items, in order.
pub const DRIFT_MENU: [(&str, &str); 2] = [("w", "save"), ("r", "revert")];

/// The drift menu's box: just under the drift tag in the strip.
pub fn drift_menu_area(app: &App, look: Look, size: Rect) -> Rect {
    let who = areas(app, size).who;
    let before: usize = strip_pieces(app, look)
        .iter()
        .take_while(|(piece, _)| *piece != Piece::Drift)
        .map(|(_, span)| span.width())
        .sum();
    let x = who.x + u16::try_from(before).unwrap_or(0);
    Rect::new(x, who.y + 1, 12, 4).intersection(size)
}

fn drift_menu(buf: &mut Buffer, size: Rect, app: &App, look: Look) {
    let theme = look.theme;
    let area = drift_menu_area(app, look, size);
    Clear.render(area, buf);
    Block::bordered()
        .border_set(look.glyphs.rounded)
        .border_style(theme.fg(Role::Accent))
        .render(area, buf);
    for ((key, label), y) in DRIFT_MENU.iter().zip(area.y + 1..) {
        let x = put(buf, area, area.x + 2, y, key, theme.fg(Role::Accent));
        put(buf, area, x + 1, y, label, theme.fg(Role::Dim));
    }
}

/// The who strip as one line.
pub fn strip(app: &App, look: Look) -> Line<'static> {
    Line::from(
        strip_pieces(app, look)
            .into_iter()
            .map(|(_, span)| span)
            .collect::<Vec<_>>(),
    )
}

/// Whether a block's legend fits beside its title in `width` columns.
pub fn legend_fits(title: &Line, legend: &Line, width: u16) -> bool {
    title.width() + legend.width() + 2 <= usize::from(width)
}

/// A block's top border: its title, then the legend right-aligned when both fit.
fn block_with_legend(
    borders: Borders,
    title: Line<'static>,
    legend: Line<'static>,
    width: u16,
    look: Look,
) -> Block<'static> {
    let fits = legend_fits(&title, &legend, width);
    let block = Block::new()
        .borders(borders)
        .border_set(look.glyphs.line)
        .border_style(look.theme.fg(Role::Dim))
        .title(title);
    if fits {
        block.title_top(legend.right_aligned())
    } else {
        block
    }
}

/// The who border's legend: the bands drawn in the people lanes, in the spec's order.
pub fn who_legend(app: &App, look: Look, track: &Track) -> Line<'static> {
    who_legend_of(&who_lanes(app, look, track), look)
}

fn who_lanes(app: &App, look: Look, track: &Track) -> Vec<(PersonId, Vec<(char, Role)>)> {
    app.who_rows()
        .iter()
        .filter_map(|row| match row {
            Row::Person(id) => Some((id, person_cells(app, track, look, id))),
            Row::Place(_) => None,
        })
        .collect()
}

fn who_legend_of(lanes: &[(PersonId, Vec<(char, Role)>)], look: Look) -> Line<'static> {
    let glyphs = look.glyphs;
    let on_screen = |glyph: char| {
        lanes
            .iter()
            .any(|(_, cells)| cells.iter().any(|(g, _)| *g == glyph))
    };
    let items: Vec<(char, Role, &str)> = [
        (glyphs.day, Role::Day, "day"),
        (glyphs.dawn, Role::Dawn, "morning/evening"),
        (glyphs.work, Role::Work, "working"),
        (glyphs.night, Role::Night, "night"),
        (glyphs.off, Role::Dim, "off"),
    ]
    .into_iter()
    .filter(|(glyph, _, _)| on_screen(*glyph))
    .collect();
    legend(&items, look)
}

fn who(buf: &mut Buffer, area: Rect, app: &App, look: Look, track: &Track, entries: &[Entry]) {
    let (theme, glyphs) = (look.theme, look.glyphs);
    let roster = app.roster();
    let lanes = who_lanes(app, look, track);
    block_with_legend(
        Borders::TOP,
        strip(app, look),
        who_legend_of(&lanes, look),
        area.width,
        look,
    )
    .render(area, buf);
    let label_width = track.label;
    let wide = label_width >= 32;
    let reference = roster.reference();
    let names = name_width(
        roster
            .people()
            .map(|(_, p)| p.name.as_str())
            .chain(reference.map(|r| r.city.as_str())),
    );
    let city_width = city_width(app, usize::from(label_width), names);
    let draw_person = |buf: &mut Buffer, y: u16, id: PersonId| {
        let person = roster.person(id);
        let counted = app.is_counted(Row::Person(id));
        let (mark, mark_role) = person_mark(app, glyphs, id);
        label(
            buf,
            area,
            y,
            label_width,
            look,
            &Label {
                mark,
                mark_role,
                name: &person.name,
                name_width: names,
                middle: wide.then(|| truncate_with_mark(&person.city, city_width, ".")),
                middle_width: city_width,
                right: time_candidates(app, person.location.tz),
                right_style: theme.fg(if counted || id == PersonId::HOME {
                    Role::Text
                } else {
                    Role::Dim
                }),
                counted: counted || id == PersonId::HOME,
                focused: app.focus == Row::Person(id),
            },
        );
        if let Some((_, cells)) = lanes.iter().find(|(lane_id, _)| *lane_id == id) {
            lane(
                buf,
                area,
                y + track.dy,
                look,
                track,
                person.location.tz,
                cells,
            );
        }
    };
    let draw_reference = |buf: &mut Buffer, y: u16| {
        let Some(reference) = reference else { return };
        label(
            buf,
            area,
            y,
            label_width,
            look,
            &Label {
                mark: glyphs.home,
                mark_role: Role::Dim,
                name: &reference.city,
                name_width: names,
                middle: wide.then(String::new),
                middle_width: city_width,
                right: time_candidates(app, reference.location.tz),
                right_style: theme.fg(Role::Dim),
                counted: false,
                focused: false,
            },
        );
        let cells = clock_cells(track, look, reference.location, roster.daylight);
        lane(
            buf,
            area,
            y + track.dy,
            look,
            track,
            reference.location.tz,
            &cells,
        );
    };
    let lens = app.lens_name();
    let who = evaluate(roster, &app.marks, app.when()).who;
    let result = match who {
        Some(block) => Ok((
            block.score.qualifying,
            block.score.counted,
            cell_counts(app, track, |a| a.who.map_or(0, |b| b.score.qualifying)),
        )),
        None => {
            let long = format!(
                "not asked: {lens} has no people, so home is the axis only (space on a person adds one)"
            );
            let room = usize::from(area.right().saturating_sub(track.x + 1));
            Err(if width_of(&long) <= room {
                long
            } else {
                format!("not asked: {lens} has no people")
            })
        }
    };
    let word = who.map_or("who", |block| block.rule.label());
    let mut y = area.y + 1;
    for entry in entries {
        match entry {
            Entry::Row(Row::Person(id)) => draw_person(buf, y, *id),
            Entry::Reference => draw_reference(buf, y),
            Entry::Answer => answer(buf, area, y, look, track, word, result.clone()),
            Entry::More(folded) => more(buf, area, y, label_width, app, look, folded),
            Entry::Row(Row::Place(_)) => {}
        }
        y += 1 + track.dy;
    }
}

/// A lane of plain day and night at `location`, as a clock row without hours draws it.
fn clock_cells(
    track: &Track,
    look: Look,
    location: teamzones_core::schedule::Location,
    daylight: teamzones_core::schedule::Daylight,
) -> Vec<(char, Role)> {
    track
        .columns()
        .map(|(index, _)| {
            let slot = track
                .slots(index)
                .next()
                .unwrap_or(Slot::containing(track.start));
            let band = band(location, None, daylight, slot);
            (look.glyphs.band(band), band_role(band))
        })
        .collect()
}

/// A person's mark and its role: home's own, counted, or dim.
fn person_mark(app: &App, glyphs: &Glyphs, id: PersonId) -> (&'static str, Role) {
    match (id == PersonId::HOME, app.is_counted(Row::Person(id))) {
        (true, _) => (app.home_mark(glyphs), Role::Marker),
        (false, true) => (glyphs.person, Role::Marker),
        (false, false) => (glyphs.person_other, Role::Dim),
    }
}

/// One line of the fold list: the mark, the name, then the local time or the place's state.
fn entry_text(app: &App, look: Look, entry: &Entry) -> String {
    let glyphs = look.glyphs;
    let roster = app.roster();
    match entry {
        Entry::Row(Row::Person(id)) => {
            let person = roster.person(*id);
            let (mark, _) = person_mark(app, glyphs, *id);
            let time = &time_candidates(app, person.location.tz)[0];
            format!("{mark} {}  {time}", person.name)
        }
        Entry::Row(row @ Row::Place(id)) => {
            let place = roster.place(*id);
            let mark = if app.is_counted(*row) {
                glyphs.place
            } else {
                glyphs.place_other
            };
            let state = teamzones_core::question::place_state(roster, *id, app.when()).map_or_else(
                || time_candidates(app, place.location.tz)[0].clone(),
                |level| place.labels().word(level).to_owned(),
            );
            format!("{mark} {}  {state}", place.name)
        }
        Entry::Reference => roster.reference().map_or_else(String::new, |reference| {
            let time = &time_candidates(app, reference.location.tz)[0];
            format!("{} {}  {time}", glyphs.home, reference.city)
        }),
        Entry::More(folded) => more_text(app, folded),
        Entry::Answer => String::new(),
    }
}

/// The list `enter` opens on a `+N more` line: a centred box with one line per folded row.
fn fold_overlay(buf: &mut Buffer, size: Rect, app: &App, look: Look, folded: &[Entry]) {
    let theme = look.theme;
    let lines: Vec<String> = folded
        .iter()
        .map(|entry| entry_text(app, look, entry))
        .collect();
    let widest = lines.iter().map(|line| width_of(line)).max().unwrap_or(0);
    let width = u16::try_from(widest + 4)
        .unwrap_or(u16::MAX)
        .min(size.width);
    let height = u16::try_from(lines.len() + 2)
        .unwrap_or(u16::MAX)
        .min(size.height);
    let area = Rect::new(
        size.x + (size.width - width) / 2,
        size.y + (size.height - height) / 2,
        width,
        height,
    );
    Clear.render(area, buf);
    let block = Block::bordered()
        .border_set(look.glyphs.rounded)
        .border_style(theme.fg(Role::Accent))
        .title(Span::styled(
            format!(" {} ", more_text(app, folded)),
            theme.fg(Role::Text).add_modifier(Modifier::BOLD),
        ));
    let inner = block.inner(area);
    block.render(area, buf);
    for (y, line) in (inner.y..inner.bottom()).zip(&lines) {
        put(buf, inner, inner.x + 1, y, line, theme.fg(Role::Text));
    }
}

/// The `+N more` text for `folded`, naming how many of its rows are counted.
pub fn more_text(app: &App, folded: &[Entry]) -> String {
    let counted = folded
        .iter()
        .flat_map(Entry::rows)
        .filter(|row| app.is_counted(*row))
        .count();
    match counted {
        0 => format!("+{} more", folded.len()),
        counted => format!("+{} more, {counted} counted", folded.len()),
    }
}

/// Draws a `+N more` line, its mark the block's uncounted one.
fn more(buf: &mut Buffer, area: Rect, y: u16, width: u16, app: &App, look: Look, folded: &[Entry]) {
    let glyphs = look.glyphs;
    let first = folded.iter().flat_map(Entry::rows).next();
    let mark = match first {
        Some(Row::Place(_)) => glyphs.place_other,
        Some(Row::Person(_)) | None => glyphs.person_other,
    };
    let text = more_text(app, folded);
    label(
        buf,
        area,
        y,
        width,
        look,
        &Label {
            mark,
            mark_role: Role::Dim,
            name: &text,
            name_width: width_of(&text),
            middle: None,
            middle_width: 0,
            right: Vec::new(),
            right_style: look.theme.fg(Role::Dim),
            counted: false,
            focused: first == Some(app.focus),
        },
    );
}

/// The where border's question after its title: `asks open`, `context` or `not asked`, which a
/// click hit-tests as `=`.
pub fn where_question(app: &App, look: Look) -> Vec<Span<'static>> {
    let theme = look.theme;
    let marks = &app.marks;
    match (marks.asks_where(), marks.rule, marks.places.is_empty()) {
        (true, rule, _) => vec![
            Span::styled(" asks ", theme.fg(Role::Dim)),
            Span::styled(rule.word(), theme.fg(Role::Accent)),
        ],
        (false, Rule::Context, false) => vec![Span::styled(" context", theme.fg(Role::Dim))],
        (false, Rule::Context, true) | (false, Rule::Open | Rule::Closed, _) => {
            vec![Span::styled(" not asked", theme.fg(Role::Dim))]
        }
    }
}

fn places(buf: &mut Buffer, area: Rect, app: &App, look: Look, track: &Track, entries: &[Entry]) {
    let (theme, glyphs) = (look.theme, look.glyphs);
    let roster = app.roster();
    let marks = &app.marks;
    let mut title = vec![Span::styled(
        WHERE_TITLE,
        theme.fg(Role::Text).add_modifier(Modifier::BOLD),
    )];
    title.extend(where_question(app, look));
    title.push(Span::raw(" "));
    let words = roster.shared_labels();
    let items = [
        (glyphs.quiet, Role::Quiet, words.closed.as_str()),
        (glyphs.off_peak, Role::Dim, words.open.as_str()),
        (glyphs.peak, Role::Warn, words.busy.as_str()),
    ];
    block_with_legend(
        Borders::TOP | Borders::BOTTOM,
        Line::from(title),
        legend(&items, look),
        area.width,
        look,
    )
    .render(area, buf);
    let label_width = track.label;
    let wide = label_width >= 32;
    let names = name_width(roster.places().map(|(_, p)| p.name.as_str()));
    let when = app.when();
    let draw_place = |buf: &mut Buffer, y: u16, row: Row| {
        let Row::Place(id) = row else { return };
        let place = roster.place(id);
        let tz = place.location.tz;
        let counted = app.is_counted(row);
        let state = teamzones_core::question::place_state(roster, id, when);
        let right_style = match (state, counted) {
            (Some(Level::Busy), true) => theme.fg(Role::Warn).add_modifier(Modifier::BOLD),
            (Some(Level::Busy), false) => theme.fg(Role::Warn),
            (Some(Level::Open | Level::Closed) | None, true) => theme.fg(Role::Text),
            (Some(Level::Open | Level::Closed) | None, false) => theme.fg(Role::Dim),
        };
        let clock = time_candidates(app, tz);
        label(
            buf,
            area,
            y,
            label_width,
            look,
            &Label {
                mark: if counted {
                    glyphs.place
                } else {
                    glyphs.place_other
                },
                mark_role: if counted { Role::Marker } else { Role::Dim },
                name: &place.name,
                name_width: names + 1,
                middle: wide.then(|| clock[0].clone()),
                middle_width: width_of(&clock[0]),
                right: vec![state.map_or_else(
                    || clock[1].clone(),
                    |level| place.labels().word(level).to_owned(),
                )],
                right_style,
                counted,
                focused: app.focus == row,
            },
        );
        let cells: Vec<(char, Role)> = track
            .columns()
            .map(|(index, _)| {
                let slot = track
                    .slots(index)
                    .next()
                    .unwrap_or(Slot::containing(track.start));
                match &place.trading {
                    Some(trading) => {
                        let level = teamzones_core::schedule::level(trading, tz, slot);
                        (glyphs.level(level), level_role(level))
                    }
                    None => {
                        let band = band(place.location, None, roster.daylight, slot);
                        (glyphs.band(band), band_role(band))
                    }
                }
            })
            .collect();
        lane(buf, area, y + track.dy, look, track, tz, &cells);
    };
    let lens = app.lens_name();
    let answer_now = evaluate(roster, marks, when).place;
    let result = match (answer_now, marks.rule) {
        (Some(block), _) => Ok((
            block.score.qualifying,
            block.score.counted,
            cell_counts(app, track, |a| a.place.map_or(0, |b| b.score.qualifying)),
        )),
        (None, Rule::Context) if !marks.places.is_empty() => {
            Err("context: these places are shown, not counted".to_owned())
        }
        (None, _) => {
            let long = format!("not asked: {lens} has no places (space on a place adds one)");
            let room = usize::from(area.right().saturating_sub(track.x + 1));
            Err(if width_of(&long) <= room {
                long
            } else {
                format!("not asked: {lens} has no places")
            })
        }
    };
    let word = answer_now.map_or("where", |block| block.rule.word());
    let mut y = area.y + 1;
    for entry in entries {
        match entry {
            Entry::Row(row) => draw_place(buf, y, *row),
            Entry::Answer => answer(buf, area, y, look, track, word, result.clone()),
            Entry::More(folded) => more(buf, area, y, label_width, app, look, folded),
            Entry::Reference => {}
        }
        y += 1 + track.dy;
    }
}

/// The times box: it holds only the line `y` copies.
fn say_panel(buf: &mut Buffer, area: Rect, app: &App, look: Look) {
    let theme = look.theme;
    let drift = app.drift();
    let mut title = vec![
        Span::styled(
            " times for ",
            theme.fg(Role::Text).add_modifier(Modifier::BOLD),
        ),
        Span::styled(app.lens_name(), theme.fg(Role::Accent)),
    ];
    if !drift.is_empty() {
        title.push(Span::styled(
            format!(" {}", drift.join(" ")),
            theme.fg(Role::Warn),
        ));
    }
    title.push(Span::raw(" "));
    let line = app.say_line();
    let block = Block::bordered()
        .border_set(look.glyphs.rounded)
        .border_style(theme.fg(Role::Dim))
        .title(Line::from(title))
        .title_top(
            Line::from(vec![
                Span::styled(COPY_HINT.0, theme.fg(Role::Accent)),
                Span::styled(COPY_HINT.1, theme.fg(Role::Dim)),
            ])
            .right_aligned(),
        )
        .title_bottom(
            Line::from(Span::styled(
                format!(" {} ", summary(app)),
                theme.fg(Role::Dim),
            ))
            .right_aligned(),
        );
    let inner = block.inner(area);
    block.render(area, buf);
    let inner = Rect::new(
        inner.x + 1,
        inner.y,
        inner.width.saturating_sub(2),
        inner.height,
    );
    put(
        buf,
        inner,
        inner.x,
        inner.y,
        &look.glyphs.text(&line),
        theme.fg(Role::Text).add_modifier(Modifier::BOLD),
    );
}

/// The focused row's detail, dim, with pieces dropped whole from the right; never copied.
fn detail_line(buf: &mut Buffer, area: Rect, app: &App, look: Look) {
    let style = look.theme.fg(Role::Dim);
    let (name, pieces) = detail(app);
    let (x, y) = (area.x, area.y);
    let x = put(buf, area, x, y, &format!("{} ", look.glyphs.detail), style);
    let mut x = put(buf, area, x, y, &name, style);
    for piece in pieces {
        let text = format!("  {piece}");
        if usize::from(x) + width_of(&text) > usize::from(area.right()) {
            break;
        }
        x = put(buf, area, x, y, &text, style);
    }
}

fn notice(buf: &mut Buffer, area: Rect, app: &App, look: Look) {
    let theme = look.theme;
    let text = match &app.mode {
        Mode::Prompt(input) => Some(format!("t  {input}_")),
        Mode::CopyMenu => Some("c  copy as".to_owned()),
        Mode::SortMenu => Some("s  sort by".to_owned()),
        Mode::AddMenu(_) => Some("a  add".to_owned()),
        Mode::Normal
        | Mode::NewGroup(_)
        | Mode::Help(..)
        | Mode::Matrix(_)
        | Mode::Picker(_)
        | Mode::DriftMenu
        | Mode::Map => app
            .notice
            .as_ref()
            .filter(|notice| notice.visible(app.now))
            .map(|notice| notice.text.clone()),
    };
    if let Some(text) = text {
        let text = truncate_with_mark(&look.glyphs.text(&text), usize::from(area.width), "");
        put(buf, area, area.x, area.y, &text, theme.fg(Role::Dim));
    }
}

fn footer_row(buf: &mut Buffer, area: Rect, app: &App, look: Look, map: footer::MapKey) {
    let theme = look.theme;
    let context = context(app);
    let main = context == keys::Context::Main;
    let situation = footer::Situation {
        range: main && app.range.is_some(),
        away: main && !app.at_now(),
        map,
        config: !app.first_run,
    };
    let flash = app
        .style_flash
        .as_ref()
        .filter(|flash| flash.visible(app.now))
        .map(|flash| flash.text.as_str());
    let notes: Vec<&str> = flash
        .into_iter()
        .chain((!app.mouse).then_some("mouse off"))
        .collect();
    let right = (!notes.is_empty()).then(|| notes.join("  "));
    let room = right.as_ref().map_or(area.width, |label| {
        area.width
            .saturating_sub(u16::try_from(width_of(label) + 2).unwrap_or(u16::MAX))
    });
    if let Some(label) = &right {
        let x = area
            .right()
            .saturating_sub(u16::try_from(width_of(label)).unwrap_or(0));
        put(buf, area, x, area.y, label, theme.fg(Role::Warn));
    }
    let footer::Footer { badge, hints } = footer::line(context, situation, room);
    let badge = badge.into_iter().flat_map(|badge| {
        [
            Span::styled(
                badge,
                theme
                    .fg(Role::Accent)
                    .add_modifier(Modifier::BOLD | Modifier::REVERSED),
            ),
            Span::raw("  "),
        ]
    });
    let spans: Vec<Span> = badge
        .chain(hints.iter().enumerate().flat_map(|(index, hint)| {
            let gap = if index + 1 < hints.len() { "  " } else { "" };
            [
                Span::styled(hint.keys, theme.fg(Role::Accent)),
                Span::styled(format!(" {}{gap}", hint.label), theme.fg(Role::Dim)),
            ]
        }))
        .collect();
    Line::from(spans).render(
        Rect {
            width: room,
            ..area
        },
        buf,
    );
}

/// The copy-as dialog's box for `rows` formats, and the screen lines each format takes.
pub fn copy_dialog_area(size: Rect, rows: usize) -> (Rect, u16) {
    let width = size.width.min(110);
    let tall = u16::try_from(rows * 2 + 2).unwrap_or(u16::MAX);
    let step = if tall <= size.height { 2 } else { 1 };
    let height = size
        .height
        .min(u16::try_from(rows * usize::from(step) + 2).unwrap_or(u16::MAX));
    let area = Rect::new(
        size.x + (size.width - width) / 2,
        size.y + (size.height - height) / 2,
        width,
        height,
    );
    (area, step)
}

/// The copy-as dialog: each format's key and the line it copies, with a dim description under it,
/// or one row per format with no descriptions when two rows each would not fit.
fn copy_dialog(buf: &mut Buffer, size: Rect, app: &App, look: Look) {
    let theme = look.theme;
    let rows = app.copy_menu();
    let (area, step) = copy_dialog_area(size, rows.len());
    Clear.render(area, buf);
    let block = Block::bordered()
        .border_set(look.glyphs.line)
        .border_style(theme.fg(Role::Accent))
        .title(Span::styled(
            " copy as ",
            theme.fg(Role::Text).add_modifier(Modifier::BOLD),
        ))
        .title_bottom(
            Line::from(vec![
                Span::styled(" esc", theme.fg(Role::Accent)),
                Span::styled(" close ", theme.fg(Role::Dim)),
            ])
            .right_aligned(),
        );
    let inner = block.inner(area);
    block.render(area, buf);
    let x = inner.x + 1;
    let room = usize::from(inner.width.saturating_sub(5));
    let mark = look.glyphs.text("…");
    for ((key, line, about), y) in rows
        .iter()
        .zip((inner.top()..inner.bottom()).step_by(usize::from(step)))
    {
        let key = look.glyphs.text(&key.to_string());
        put(buf, inner, x, y, &key, theme.fg(Role::Accent));
        let preview = truncate_with_mark(&look.glyphs.text(line), room, &mark);
        put(buf, inner, x + 3, y, &preview, theme.fg(Role::Text));
        if step == 2 {
            let about = truncate_with_mark(&look.glyphs.text(about), room, &mark);
            put(buf, inner, x + 3, y + 1, &about, theme.fg(Role::Dim));
        }
    }
}

/// The help overlay's box for help opened from `from`.
pub fn help_area(size: Rect, from: keys::Context, look: Look) -> Rect {
    let lines = help::lines(from, look.glyphs).len();
    let width = size.width.min(96);
    let height = size
        .height
        .min(u16::try_from(lines + 2).unwrap_or(u16::MAX));
    Rect::new(
        size.x + (size.width - width) / 2,
        size.y + (size.height - height) / 2,
        width,
        height,
    )
}

fn help_overlay(buf: &mut Buffer, size: Rect, from: keys::Context, look: Look) {
    let theme = look.theme;
    let lines = help::lines(from, look.glyphs);
    let area = help_area(size, from, look);
    Clear.render(area, buf);
    let block = Block::bordered()
        .border_set(look.glyphs.rounded)
        .border_style(theme.fg(Role::Accent))
        .title(Span::styled(
            " help ",
            theme.fg(Role::Text).add_modifier(Modifier::BOLD),
        ));
    let inner = block.inner(area);
    block.render(area, buf);
    for (line, y) in lines.iter().zip(inner.top()..inner.bottom()) {
        if line.keys.is_empty() && !line.text.contains("  ") {
            put(
                buf,
                inner,
                inner.x + 1,
                y,
                &line.text,
                theme.fg(Role::Text).add_modifier(Modifier::BOLD),
            );
        } else if line.keys.is_empty() {
            put(buf, inner, inner.x + 3, y, &line.text, theme.fg(Role::Dim));
        } else {
            put(
                buf,
                inner,
                inner.x + 3,
                y,
                &line.keys,
                theme.fg(Role::Accent),
            );
            put(buf, inner, inner.x + 20, y, &line.text, theme.fg(Role::Dim));
        }
    }
}
