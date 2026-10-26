//! The clock list below 60 columns: a two-line row per city and per place, as a phone's world
//! clock lists cities, in place of lanes the width cannot hold.

use chrono::{DateTime, Offset, Utc};
use chrono_tz::Tz;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use teamzones_core::config::{PersonId, Roster};
use teamzones_core::question::{Row, evaluate, person_band, place_state};
use teamzones_core::schedule::{Band, Level, band};

use super::app::{App, Cursor, Lanes, Lens};
use super::fold::Entry;
use super::keys::Sort;
use super::layout::{Areas, Columns};
use super::say::{duration_text, local_text};
use super::theme::Role;
use super::view::{Look, more, person_mark, put, truncate_with_mark, width_of};

/// The roster's cities, home's first, each holding the people who live there in config order.
/// People share a city when they share its name and zone; home's reference row joins the city
/// it names, right after home's own.
pub fn cities(roster: &Roster) -> Vec<Vec<Entry>> {
    let key_of = |city: &str, tz: Tz| (city.to_owned(), tz);
    let mut people = roster.people().map(|(id, person)| {
        (
            key_of(&person.city, person.location.tz),
            Entry::Row(Row::Person(id)),
        )
    });
    let home = people.next();
    let reference = roster.reference().map(|reference| {
        (
            key_of(&reference.city, reference.location.tz),
            Entry::Reference,
        )
    });
    let mut cities: Vec<((String, Tz), Vec<Entry>)> = Vec::new();
    for (key, entry) in home.into_iter().chain(reference).chain(people) {
        match cities.iter_mut().find(|(city, _)| *city == key) {
            Some((_, members)) => members.push(entry),
            None => cities.push((key, vec![entry])),
        }
    }
    cities.into_iter().map(|(_, members)| members).collect()
}

/// The who block's lines: home's city, home's reference city while travelling, the other cities
/// holding a counted person, the answer row, then the `not in` rule over the rest.
pub fn who_lines(app: &App) -> Vec<Entry> {
    let mut cities = cities(app.roster()).into_iter();
    let first = cities.next();
    let (pinned, rest): (Vec<_>, Vec<_>) = cities.partition(|members| {
        members
            .iter()
            .any(|member| matches!(member, Entry::Reference))
    });
    let (counted, other): (Vec<_>, Vec<_>) = rest.into_iter().partition(|members| {
        members
            .iter()
            .flat_map(Entry::rows)
            .any(|row| app.is_counted(row))
    });
    let city = Entry::City;
    let outside = (!other.is_empty()).then_some(Entry::Outside);
    first
        .into_iter()
        .chain(pinned)
        .chain(sorted(app, counted))
        .map(city)
        .chain(std::iter::once(Entry::Answer))
        .chain(outside)
        .chain(sorted(app, other).into_iter().map(city))
        .collect()
}

/// `cities` east to west by offset from the axis zone (or by city name under `s`), keeping config
/// order between ties.
fn sorted(app: &App, mut cities: Vec<Vec<Entry>>) -> Vec<Vec<Entry>> {
    let start = app.when().start();
    match app.sort {
        Sort::Offset => {
            cities.sort_by_key(|members| -offset_seconds(location_of(app, members).1, start));
        }
        Sort::Name => {
            cities.sort_by_key(|members| location_of(app, members).0.to_lowercase());
        }
    }
    cities
}

/// A city row's name and zone, from its first member.
fn location_of(app: &App, members: &[Entry]) -> (String, Tz) {
    let roster = app.roster();
    members
        .iter()
        .find_map(|member| match member {
            Entry::Row(Row::Person(id)) => {
                let person = roster.person(*id);
                Some((person.city.clone(), person.location.tz))
            }
            Entry::Reference => roster
                .reference()
                .map(|reference| (reference.city.clone(), reference.location.tz)),
            Entry::Row(Row::Place(_))
            | Entry::Answer
            | Entry::More(_)
            | Entry::City(_)
            | Entry::Outside => None,
        })
        .unwrap_or_else(|| (String::new(), roster.axis()))
}

/// The screen lines an entry of the clock list takes: two for a city or place row, else one.
pub fn height(entry: &Entry) -> u16 {
    match entry {
        Entry::City(_) | Entry::Row(_) | Entry::Reference => 2,
        Entry::Answer | Entry::More(_) | Entry::Outside => 1,
    }
}

/// One drawn line or row of a clock block.
enum Placed<'a> {
    Rule,
    Entry(&'a Entry),
}

/// Each line and row of a clock block from line `top`, with a rule line between every two city
/// rows when `rules`.
fn placed(entries: &[Entry], top: u16, rules: bool) -> Vec<(u16, Placed<'_>)> {
    let mut y = top;
    let mut lines = Vec::new();
    let mut after_city = false;
    for entry in entries {
        let city = matches!(entry, Entry::City(_));
        if rules && city && after_city {
            lines.push((y, Placed::Rule));
            y += 1;
        }
        lines.push((y, Placed::Entry(entry)));
        y += height(entry);
        after_city = city;
    }
    lines
}

/// The clock list's entry drawn over screen line `y`, in either block.
pub fn entry_at(app: &App, areas: &Areas, y: u16) -> Option<Entry> {
    let rules = rules(areas);
    let (who, place) = app.entries();
    let find = |top: u16, entries: &[Entry]| {
        placed(entries, top + 1, rules)
            .into_iter()
            .find_map(|(line, placed)| match placed {
                Placed::Entry(entry) if (line..line + height(entry)).contains(&y) => {
                    Some(entry.clone())
                }
                Placed::Entry(_) | Placed::Rule => None,
            })
    };
    find(areas.who.y, &who).or_else(|| find(areas.place.y, &place))
}

/// Whether the layout left room for a rule between every two city rows.
fn rules(areas: &Areas) -> bool {
    match areas.columns {
        Columns::Clocks { rules, .. } => rules,
        Columns::Beside { .. } | Columns::Stacked { .. } => false,
    }
}

/// Seconds `tz` is ahead of UTC at `instant`.
fn offset_seconds(tz: Tz, instant: DateTime<Utc>) -> i32 {
    instant.with_timezone(&tz).offset().fix().local_minus_utc()
}

/// The small line's lead: the local weekday at `instant` and the offset from the axis zone, as in
/// `Mon, -9h` or `Mon, +5:30h`.
fn day_and_offset(tz: Tz, axis: Tz, instant: DateTime<Utc>) -> String {
    let ahead = offset_seconds(tz, instant) - offset_seconds(axis, instant);
    let sign = if ahead < 0 { '-' } else { '+' };
    let minutes = ahead.unsigned_abs() / 60;
    let offset = match minutes % 60 {
        0 => format!("{}h", minutes / 60),
        rest => format!("{}:{rest:02}h", minutes / 60),
    };
    let weekday = instant.with_timezone(&tz).format("%a");
    format!("{weekday}, {sign}{offset}")
}

/// A zone's city, from its IANA name: `Europe/London` reads `London`.
fn zone_city(tz: Tz) -> String {
    tz.name()
        .rsplit('/')
        .next()
        .unwrap_or_default()
        .replace('_', " ")
}

/// The word for a band in a city row.
fn band_word(band: Band) -> &'static str {
    match band {
        Band::Night => "night",
        Band::Day => "day",
        Band::Morning => "morning",
        Band::Working => "working",
        Band::Evening => "evening",
        Band::Off => "off",
    }
}

/// The word for the one band `bands` share, or `mixed`.
fn shared_word(mut bands: impl Iterator<Item = Band>) -> &'static str {
    let Some(first) = bands.next() else {
        return "mixed";
    };
    if bands.all(|band| band == first) {
        band_word(first)
    } else {
        "mixed"
    }
}

/// A city row's state word: the lane state every member holds at the cursor or the range's
/// start, or `mixed`.
fn city_word(app: &App, members: &[Entry]) -> &'static str {
    let roster = app.roster();
    let slot = app.start_slot();
    let bands = members.iter().filter_map(|member| match member {
        Entry::Row(Row::Person(id)) => {
            let person = roster.person(*id);
            Some(match app.lanes {
                Lanes::Hours => person_band(roster, *id, slot),
                Lanes::Plain => band(person.location, None, roster.daylight, slot),
            })
        }
        Entry::Reference => roster
            .reference()
            .map(|reference| band(reference.location, None, roster.daylight, slot)),
        Entry::Row(Row::Place(_))
        | Entry::Answer
        | Entry::More(_)
        | Entry::City(_)
        | Entry::Outside => None,
    });
    shared_word(bands)
}

/// Writes `spans` from `x` on line `y`, cut with a `.` in the cut span's style when they would
/// pass column `end`.
fn spans_until(buf: &mut Buffer, area: Rect, x: u16, y: u16, end: u16, spans: &[(String, Style)]) {
    let room = usize::from(end.saturating_sub(x));
    let cut = spans.iter().map(|(text, _)| width_of(text)).sum::<usize>() > room;
    let budget = if cut { room.saturating_sub(1) } else { room };
    let (mut x, mut used) = (x, 0);
    for (text, style) in spans {
        let shown = truncate_with_mark(text, budget - used, "");
        used += width_of(&shown);
        x = put(buf, area, x, y, &shown, *style);
        if shown.len() < text.len() {
            put(buf, area, x, y, ".", *style);
            return;
        }
    }
}

/// Writes `text` right-aligned to the line's last column; returns the column it starts at.
fn right(buf: &mut Buffer, area: Rect, y: u16, text: &str, style: Style) -> u16 {
    let x = area
        .right()
        .saturating_sub(u16::try_from(width_of(text)).unwrap_or(area.width));
    put(buf, area, x, y, text, style);
    x
}

/// The clock list's slider: one line naming the viewed day and the cursor's time or the range,
/// with `◂` at the right while the view is away from today.
pub fn slider(buf: &mut Buffer, area: Rect, app: &App, look: Look) {
    let (theme, glyphs) = (look.theme, look.glyphs);
    let axis = app.roster().axis();
    let viewed = app.when().start().with_timezone(&axis);
    let duration = match (app.shown_range(), app.cursor) {
        (Some(range), _) => duration_text(range.end().since(range.start())),
        (None, Cursor::Now) => "now".to_owned(),
        (None, Cursor::At(_)) => String::new(),
    };
    let x = put(
        buf,
        area,
        area.x,
        area.y,
        &format!("{} ", glyphs.pointer),
        theme.fg(Role::Accent),
    );
    let label = format!(
        "{} {} {}",
        viewed.format("%a %-d %b"),
        local_text(app.when(), axis),
        viewed.format("%Z")
    );
    let x = put(
        buf,
        area,
        x,
        area.y,
        &label,
        theme.fg(Role::Accent).add_modifier(Modifier::BOLD),
    );
    put(buf, area, x + 1, area.y, &duration, theme.fg(Role::Dim));
    if app.day() != app.today() {
        right(buf, area, area.y, glyphs.back, theme.fg(Role::Dim));
    }
}

/// The lens's place among every lens, as `1 of 5`: the groups in file order, then `all`.
fn lens_position(app: &App) -> String {
    let count = app.roster().groups.len() + 1;
    let position = match app.lens {
        Lens::Group(index) => index + 1,
        Lens::All => count,
    };
    format!("{position} of {count}")
}

/// The title line over the who block: the lens's name, its drift, and its place among the lenses.
fn title(buf: &mut Buffer, area: Rect, app: &App, look: Look) {
    let theme = look.theme;
    let position = lens_position(app);
    let end = right(buf, area, area.y, &position, theme.fg(Role::Dim)).saturating_sub(1);
    let drift = app.drift();
    let mut spans = vec![(
        app.lens_name(),
        theme.fg(Role::Text).add_modifier(Modifier::BOLD),
    )];
    if !drift.is_empty() {
        spans.push((format!(" {}", drift.join(" ")), theme.fg(Role::Warn)));
    }
    spans_until(buf, area, area.x, area.y, end, &spans);
}

/// Draws the who block as the clock list: the title line, then each city's two lines.
pub fn who(buf: &mut Buffer, area: Rect, app: &App, look: Look, areas: &Areas, entries: &[Entry]) {
    title(buf, area, app, look);
    let who = evaluate(app.roster(), &app.marks, app.when()).who;
    let word = who.map_or("who", |block| block.rule.label());
    let score = who.map(|block| (block.score.qualifying, block.score.counted));
    for (y, line) in placed(entries, area.y + 1, rules(areas)) {
        match line {
            Placed::Rule => rule(buf, area, y, look, ""),
            Placed::Entry(Entry::City(members)) => city_row(buf, area, y, app, look, members),
            Placed::Entry(Entry::Answer) => answer(buf, area, y, look, word, score),
            Placed::Entry(Entry::Outside) => {
                rule(buf, area, y, look, &format!(" not in {} ", app.lens_name()));
            }
            Placed::Entry(Entry::More(folded)) => more(buf, area, y, area.width, app, look, folded),
            Placed::Entry(Entry::Row(_) | Entry::Reference) => {}
        }
    }
}

/// A rule across the block, with `label` near its right end.
fn rule(buf: &mut Buffer, area: Rect, y: u16, look: Look, label: &str) {
    let style = look.theme.fg(Role::Dim);
    let line = look
        .glyphs
        .line
        .horizontal_top
        .repeat(usize::from(area.width));
    put(buf, area, area.x, y, &line, style);
    if !label.is_empty() {
        let width = u16::try_from(width_of(label)).unwrap_or(area.width);
        let x = area.right().saturating_sub(width + 3).max(area.x);
        put(buf, area, x, y, label, style);
    }
}

/// An answer row on one line: its word on the left and the count, or `--`, on the right.
fn answer(
    buf: &mut Buffer,
    area: Rect,
    y: u16,
    look: Look,
    word: &str,
    score: Option<(usize, usize)>,
) {
    let (theme, glyphs) = (look.theme, look.glyphs);
    let x = put(buf, area, area.x, y, glyphs.answer, theme.fg(Role::Accent));
    put(buf, area, x + 1, y, word, theme.fg(Role::Dim));
    let (count, style) = match score {
        Some((qualifying, counted)) => (
            format!("{qualifying}/{counted}"),
            theme
                .fg(if qualifying == counted {
                    Role::Ok
                } else {
                    Role::Dim
                })
                .add_modifier(Modifier::BOLD),
        ),
        None => ("--".to_owned(), theme.fg(Role::Dim)),
    };
    right(buf, area, y, &count, style);
}

/// Draws a city row: the day, offset, people and state on the small line, then the city and its
/// time in bold on the big line. A city with nobody counted is dim throughout.
fn city_row(buf: &mut Buffer, area: Rect, y: u16, app: &App, look: Look, members: &[Entry]) {
    let (theme, glyphs) = (look.theme, look.glyphs);
    let roster = app.roster();
    let (city, tz) = location_of(app, members);
    let counts = members.iter().any(|member| match member {
        Entry::Row(row @ Row::Person(id)) => *id == PersonId::HOME || app.is_counted(*row),
        Entry::Reference => true,
        Entry::Row(Row::Place(_))
        | Entry::Answer
        | Entry::More(_)
        | Entry::City(_)
        | Entry::Outside => false,
    });
    let role = |counted: bool| {
        if counted && counts {
            Role::Text
        } else {
            Role::Dim
        }
    };
    let word = city_word(app, members);
    let end = right(buf, area, y, word, theme.fg(role(true))).saturating_sub(1);
    let mut spans = vec![(
        format!(
            "{}  ",
            day_and_offset(tz, roster.axis(), app.when().start())
        ),
        theme.fg(Role::Dim),
    )];
    for (index, member) in members.iter().enumerate() {
        if index > 0 {
            spans.push((", ".to_owned(), theme.fg(Role::Dim)));
        }
        let (mark, mark_role, name, counted, focused) = match member {
            Entry::Row(row @ Row::Person(id)) => {
                let (mark, mark_role) = person_mark(app, glyphs, *id);
                let counted = *id == PersonId::HOME || app.is_counted(*row);
                let name = roster.person(*id).name.clone();
                (mark, mark_role, name, counted, app.focus == *row)
            }
            Entry::Reference => (glyphs.home, Role::Dim, "home".to_owned(), false, false),
            Entry::Row(Row::Place(_))
            | Entry::Answer
            | Entry::More(_)
            | Entry::City(_)
            | Entry::Outside => continue,
        };
        let mark_role = if counts { mark_role } else { Role::Dim };
        let focus = |style: Style| {
            if focused {
                style.patch(theme.selection).add_modifier(Modifier::BOLD)
            } else {
                style
            }
        };
        spans.push((format!("{mark} "), focus(theme.fg(mark_role))));
        spans.push((name, focus(theme.fg(role(counted)))));
    }
    spans_until(buf, area, area.x, y, end, &spans);
    big_line(
        buf,
        area,
        y + 1,
        look,
        &city,
        &local_text(app.when(), tz),
        role(true),
    );
}

/// A row's big line: its name on the left and its local time on the right, both bold.
fn big_line(buf: &mut Buffer, area: Rect, y: u16, look: Look, name: &str, time: &str, role: Role) {
    let style = look.theme.fg(role).add_modifier(Modifier::BOLD);
    let end = right(buf, area, y, time, style).saturating_sub(1);
    spans_until(buf, area, area.x, y, end, &[(name.to_owned(), style)]);
}

/// Draws the where block as the clock list: its heading, then each place's two lines.
pub fn places(buf: &mut Buffer, area: Rect, app: &App, look: Look, entries: &[Entry]) {
    let theme = look.theme;
    let rule_line = look
        .glyphs
        .line
        .horizontal_top
        .repeat(usize::from(area.width));
    put(buf, area, area.x, area.y, &rule_line, theme.fg(Role::Dim));
    let x = put(
        buf,
        area,
        area.x,
        area.y,
        super::view::WHERE_TITLE,
        theme.fg(Role::Text).add_modifier(Modifier::BOLD),
    );
    let mut x = x;
    for span in super::view::where_question(app, look) {
        x = put(buf, area, x, area.y, &span.content, span.style);
    }
    put(buf, area, x, area.y, " ", Style::new());
    let place = evaluate(app.roster(), &app.marks, app.when()).place;
    let word = place.map_or("where", |block| block.rule.word());
    let score = place.map(|block| (block.score.qualifying, block.score.counted));
    for (y, line) in placed(entries, area.y + 1, false) {
        match line {
            Placed::Entry(Entry::Row(row @ Row::Place(_))) => {
                place_row(buf, area, y, app, look, *row);
            }
            Placed::Entry(Entry::Answer) => answer(buf, area, y, look, word, score),
            Placed::Entry(Entry::More(folded)) => more(buf, area, y, area.width, app, look, folded),
            Placed::Rule
            | Placed::Entry(
                Entry::Row(Row::Person(_)) | Entry::Reference | Entry::City(_) | Entry::Outside,
            ) => {}
        }
    }
}

/// Draws a place row: the day, offset, mark, zone city and state on the small line, then the
/// place and its time on the big line. A place the group does not ask is dim.
fn place_row(buf: &mut Buffer, area: Rect, y: u16, app: &App, look: Look, row: Row) {
    let (theme, glyphs) = (look.theme, look.glyphs);
    let Row::Place(id) = row else { return };
    let roster = app.roster();
    let place = roster.place(id);
    let tz = place.location.tz;
    let counted = app.is_counted(row);
    let role = if counted { Role::Text } else { Role::Dim };
    let when = app.when();
    let (word, style) = match place_state(roster, id, when) {
        Some(level @ Level::Busy) => (
            place.labels().word(level).to_owned(),
            theme.fg(Role::Warn).add_modifier(Modifier::BOLD),
        ),
        Some(level @ (Level::Open | Level::Closed)) => {
            (place.labels().word(level).to_owned(), theme.fg(role))
        }
        None => (
            band_word(band(
                place.location,
                None,
                roster.daylight,
                app.start_slot(),
            ))
            .to_owned(),
            theme.fg(role),
        ),
    };
    let end = right(buf, area, y, &word, style).saturating_sub(1);
    let (mark, mark_role) = if counted {
        (glyphs.place, Role::Marker)
    } else {
        (glyphs.place_other, Role::Dim)
    };
    let focus = |style: Style| {
        if app.focus == row {
            style.patch(theme.selection).add_modifier(Modifier::BOLD)
        } else {
            style
        }
    };
    let spans = [
        (
            format!("{}  ", day_and_offset(tz, roster.axis(), when.start())),
            theme.fg(Role::Dim),
        ),
        (format!("{mark} "), focus(theme.fg(mark_role))),
        (zone_city(tz), focus(theme.fg(role))),
    ];
    spans_until(buf, area, area.x, y, end, &spans);
    big_line(
        buf,
        area,
        y + 1,
        look,
        &place.name,
        &local_text(when, tz),
        role,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono_tz::{America, Asia, Australia, Europe};

    fn instant() -> DateTime<Utc> {
        "2026-10-26T06:00:00Z".parse().unwrap()
    }

    #[test]
    fn the_small_line_names_the_local_weekday_and_the_offset_from_the_axis() {
        let axis = Australia::Melbourne;
        let cases = [
            (Australia::Melbourne, "Mon, +0h"),
            (Australia::Brisbane, "Mon, -1h"),
            (America::Chicago, "Mon, -16h"),
            (Asia::Kolkata, "Mon, -5:30h"),
            (chrono_tz::Pacific::Auckland, "Mon, +2h"),
        ];
        for (tz, text) in cases {
            assert_eq!(day_and_offset(tz, axis, instant()), text, "{tz}");
        }
    }

    #[test]
    fn a_zone_reads_as_its_city() {
        assert_eq!(zone_city(Europe::London), "London");
        assert_eq!(zone_city(America::Argentina::Buenos_Aires), "Buenos Aires");
        assert_eq!(zone_city(chrono_tz::UTC), "UTC");
    }
}
