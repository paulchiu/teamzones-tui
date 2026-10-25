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
use teamzones_core::config::{MapShow, PersonId, Rule};
use teamzones_core::fit::{day_start, slot_fits};
use teamzones_core::question::{Row, evaluate, person_band};
use teamzones_core::schedule::{Band, Level, band, is_day};

use super::app::{App, Cursor, Lanes, Mode, without_colour};
use super::glyphs::Glyphs;
use super::layout::{Areas, MapChoice, layout};
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
fn put(buf: &mut Buffer, area: Rect, x: u16, y: u16, text: &str, style: Style) -> u16 {
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
struct Track {
    x: u16,
    cells: u16,
    /// The label's width: the columns before the track, or the whole line when stacked.
    label: u16,
    /// How far below a row's label line its lane sits: 0 side by side, 1 stacked.
    dy: u16,
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

    /// The grid slots cell `index` overlaps, first to last.
    fn slots(&self, index: u16) -> impl Iterator<Item = Slot> {
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
    fn selected(&self, index: u16) -> bool {
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

fn map_choice(app: &App, glyphs: &Glyphs) -> MapChoice {
    match (glyphs.ascii, app.config.map) {
        (true, _) | (false, MapShow::Never) => MapChoice::Hidden,
        (false, MapShow::Auto | MapShow::Always) => MapChoice::Auto,
    }
}

/// The screen's areas for this frame size and roster.
pub fn areas(app: &App, glyphs: &Glyphs, size: Rect) -> Areas {
    let count = |n: usize| u16::try_from(n).unwrap_or(u16::MAX);
    layout(
        size.width,
        size.height,
        count(app.roster().people().count()),
        count(app.roster().places().count()),
        map_choice(app, glyphs),
    )
}

/// Draws the whole screen.
pub fn draw(frame: &mut Frame, app: &App, look: Look) {
    let size = frame.area();
    let areas = areas(app, look.glyphs, size);
    let clip = |area: Rect| size.intersection(area);
    let track = Track {
        x: areas.columns.track_x(),
        cells: areas.columns.track(),
        label: areas.columns.label(),
        dy: areas.columns.lines() - 1,
        start: day_start(app.day(), app.roster().axis()),
        when: app.when(),
        marking: without_colour(app.range_style, look.theme.is_mono()),
    };
    let buf = frame.buffer_mut();
    header(buf, clip(areas.header), app, look);
    if let Some(area) = areas.map {
        super::map::draw(buf, clip(area), app, look);
    }
    slider(buf, clip(areas.slider), app, look, &track);
    who(buf, clip(areas.who), app, look, &track);
    places(buf, clip(areas.place), app, look, &track);
    say_panel(buf, clip(areas.say), app, look);
    notice(buf, clip(areas.notice), app, look);
    footer_row(buf, clip(areas.footer), app, look);
    match app.mode {
        Mode::Help(from) => help_overlay(buf, size, from, look),
        Mode::CopyMenu => copy_dialog(buf, size, app, look),
        Mode::Normal | Mode::Prompt(_) | Mode::SortMenu => {}
    }
}

fn header(buf: &mut Buffer, area: Rect, app: &App, look: Look) {
    let (theme, glyphs) = (look.theme, look.glyphs);
    let axis = app.roster().axis();
    let now = app.now.with_timezone(&axis);
    let days = (app.day() - app.today()).num_days();
    let mut pieces: Vec<Vec<Span>> = vec![
        vec![Span::styled(
            "teamzones",
            theme.fg(Role::Accent).add_modifier(Modifier::BOLD),
        )],
        vec![
            Span::styled("   now ", theme.fg(Role::Dim)),
            Span::styled(
                format!(
                    "{} {}",
                    now.format("%a %-d %b"),
                    teamzones_core::copy::clock_text(&now)
                ),
                theme.fg(Role::Dim),
            ),
            Span::styled(format!(" {}", now.format("%Z")), theme.fg(Role::Dim)),
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
    let home = app.roster().person(PersonId::HOME);
    let right = vec![
        Span::styled(format!("{} ", glyphs.home), theme.fg(Role::Marker)),
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

/// The who strip: `who`, then each numbered group with the active one marked and its drift.
fn strip(app: &App, look: Look) -> Line<'static> {
    let theme = look.theme;
    let mut spans = vec![Span::styled(
        " who ",
        theme.fg(Role::Text).add_modifier(Modifier::BOLD),
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
        .map(|(index, group)| ((index + 1).to_string(), group.name.clone(), Some(index)))
        .chain(std::iter::once(("0".to_owned(), "all".to_owned(), None)));
    for (number, name, index) in names {
        spans.push(Span::styled(format!("  {number} "), theme.fg(Role::Dim)));
        if active(index) {
            spans.push(Span::styled(
                name,
                theme
                    .fg(Role::Accent)
                    .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            ));
            let drift = app.drift();
            if !drift.is_empty() {
                spans.push(Span::styled(
                    format!(" {}", drift.join(" ")),
                    theme.fg(Role::Warn),
                ));
            }
        } else {
            spans.push(Span::styled(name, theme.fg(Role::Dim)));
        }
    }
    spans.push(Span::raw("  "));
    Line::from(spans)
}

/// A block's top border: its title, then the legend right-aligned when both fit.
fn block_with_legend(
    borders: Borders,
    title: Line<'static>,
    legend: Line<'static>,
    width: u16,
    look: Look,
) -> Block<'static> {
    let fits = title.width() + legend.width() + 2 <= usize::from(width);
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

fn who(buf: &mut Buffer, area: Rect, app: &App, look: Look, track: &Track) {
    let (theme, glyphs) = (look.theme, look.glyphs);
    let roster = app.roster();
    let rows = app.who_rows();
    let lanes: Vec<(PersonId, Vec<(char, Role)>)> = rows
        .iter()
        .filter_map(|row| match row {
            Row::Person(id) => Some((id, person_cells(app, track, look, id))),
            Row::Place(_) => None,
        })
        .collect();
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
    block_with_legend(
        Borders::TOP,
        strip(app, look),
        legend(&items, look),
        area.width,
        look,
    )
    .render(area, buf);
    let label_width = track.label;
    let wide = label_width >= 32;
    let names = name_width(roster.people().map(|(_, p)| p.name.as_str()));
    let city_width = city_width(app, usize::from(label_width), names);
    let draw_person = |buf: &mut Buffer, y: u16, id: PersonId| {
        let person = roster.person(id);
        let counted = app.is_counted(Row::Person(id));
        let (mark, mark_role) = match (id == PersonId::HOME, counted) {
            (true, _) => (glyphs.home, Role::Marker),
            (false, true) => (glyphs.person, Role::Marker),
            (false, false) => (glyphs.person_other, Role::Dim),
        };
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
    let mut y = area.y + 1;
    for row in &rows.above {
        if let Row::Person(id) = row {
            draw_person(buf, y, *id);
        }
        y += 1 + track.dy;
    }
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
    answer(buf, area, y, look, track, word, result);
    y += 1 + track.dy;
    for row in &rows.below {
        if let Row::Person(id) = row {
            draw_person(buf, y, *id);
        }
        y += 1 + track.dy;
    }
}

fn places(buf: &mut Buffer, area: Rect, app: &App, look: Look, track: &Track) {
    let (theme, glyphs) = (look.theme, look.glyphs);
    let roster = app.roster();
    let marks = &app.marks;
    let mut title = vec![Span::styled(
        " where ",
        theme.fg(Role::Text).add_modifier(Modifier::BOLD),
    )];
    match (marks.asks_where(), marks.rule, marks.places.is_empty()) {
        (true, rule, _) => title.extend([
            Span::styled(" asks ", theme.fg(Role::Dim)),
            Span::styled(rule.word(), theme.fg(Role::Accent)),
        ]),
        (false, Rule::Context, false) => title.push(Span::styled(" context", theme.fg(Role::Dim))),
        (false, Rule::Context, true) | (false, Rule::Open | Rule::Closed, _) => {
            title.push(Span::styled(" not asked", theme.fg(Role::Dim)));
        }
    }
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
    let rows = app.where_rows();
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
    let mut y = area.y + 1;
    for row in &rows.above {
        draw_place(buf, y, *row);
        y += 1 + track.dy;
    }
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
    answer(buf, area, y, look, track, word, result);
    y += 1 + track.dy;
    for row in &rows.below {
        draw_place(buf, y, *row);
        y += 1 + track.dy;
    }
}

/// The copy box and the dim detail line above it: the box holds only the line `y` copies.
fn say_panel(buf: &mut Buffer, area: Rect, app: &App, look: Look) {
    let theme = look.theme;
    detail_line(buf, Rect { height: 1, ..area }, app, look);
    let area = Rect {
        y: area.y + 1,
        height: area.height.saturating_sub(1),
        ..area
    };
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
                Span::styled(" y", theme.fg(Role::Accent)),
                Span::styled(" copy ", theme.fg(Role::Dim)),
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
        Mode::Normal | Mode::Help(_) => app
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

fn footer_row(buf: &mut Buffer, area: Rect, app: &App, look: Look) {
    let theme = look.theme;
    let context = context(app);
    let main = context == keys::Context::Main;
    let situation = footer::Situation {
        range: main && app.range.is_some(),
        away: main && !app.at_now(),
    };
    let flash = app
        .style_flash
        .as_ref()
        .filter(|flash| flash.visible(app.now))
        .map(|flash| flash.text.as_str());
    let room = flash.map_or(area.width, |label| {
        area.width
            .saturating_sub(u16::try_from(label.len() + 2).unwrap_or(u16::MAX))
    });
    if let Some(label) = flash {
        let x = area
            .right()
            .saturating_sub(u16::try_from(label.len()).unwrap_or(0));
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

/// The copy-as dialog: each format's key and the line it copies, with a dim description under it,
/// or one row per format with no descriptions when two rows each would not fit.
fn copy_dialog(buf: &mut Buffer, size: Rect, app: &App, look: Look) {
    let theme = look.theme;
    let rows = app.copy_menu();
    let width = size.width.min(110);
    let tall = u16::try_from(rows.len() * 2 + 2).unwrap_or(u16::MAX);
    let step = if tall <= size.height { 2 } else { 1 };
    let height = size
        .height
        .min(u16::try_from(rows.len() * step + 2).unwrap_or(u16::MAX));
    let area = Rect::new(
        size.x + (size.width - width) / 2,
        size.y + (size.height - height) / 2,
        width,
        height,
    );
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
    for ((key, line, about), y) in rows.iter().zip((inner.top()..inner.bottom()).step_by(step)) {
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

fn help_overlay(buf: &mut Buffer, size: Rect, from: keys::Context, look: Look) {
    let theme = look.theme;
    let lines = help::lines(from, look.glyphs);
    let width = size.width.min(96);
    let height = size
        .height
        .min(u16::try_from(lines.len() + 2).unwrap_or(u16::MAX));
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
