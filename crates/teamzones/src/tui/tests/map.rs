//! The map: its labels, the night side and the sun, and when it shows.

use super::*;

/// Regions on Mon 26 Oct with a 2-4pm AEDT range, as the screen spec's second mockup draws it.
fn regions_monday() -> App {
    go_to(press(fixture(), &chars("3")), "26 oct 2-4pm")
}

/// The map's rows: everything between the header and the ruler, which reads `ruler`.
fn rows_above(buf: &Buffer, ruler: &str) -> String {
    let ruler = y_of(buf, ruler);
    (1..ruler)
        .map(|y| row(buf, y))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The map's rows on Mon 26 Oct in Melbourne.
fn map_text(buf: &Buffer) -> String {
    rows_above(buf, "Mon 26 Oct AEDT")
}

#[test]
fn the_map_labels_home_and_each_counted_place_with_its_state_over_the_range() {
    // 24 spare rows hold the whole map at 120 columns, so nothing is cropped.
    let map = map_text(&frame(&sized(regions_monday(), 120, 48)));
    for label in [
        "UK closed 3-5am",
        "US open 10pm-12am",
        "AU open 2-4pm",
        "Melbourne 2-4pm",
    ] {
        assert!(map.contains(label), "no {label:?} in\n{map}");
    }
}

#[test]
fn at_120_by_36_regions_fills_the_width_and_crops_to_the_band_around_its_markers() {
    // Melbourne (38S) to the UK (54.5N) centres the band on 8N, and 12 rows at 120 columns
    // show 72 degrees, so the band runs from about 44N to 28S: AU and the US are in it.
    let map = map_text(&frame(&sized(regions_monday(), 120, 36)));
    for label in ["US open 10pm-12am", "AU open 2-4pm"] {
        assert!(map.contains(label), "no {label:?} in\n{map}");
    }
    for cropped in ["UK closed", "Melbourne"] {
        assert!(!map.contains(cropped), "{cropped:?} in\n{map}");
    }
    insta::assert_snapshot!("regions_map_120x36", map);
}

/// The cell a latitude and longitude falls in on a whole 120-column map one row below the
/// header, by plain arithmetic: 3 degrees a column, and twice that a row, from 80N.
fn cell_of(lat: f64, lon: f64) -> (i32, i32) {
    let x = ((lon + 180.0) / 3.0).floor() as i32;
    let y = 1 + ((80.0 - lat) / 6.0).floor() as i32;
    (x, y)
}

#[test]
fn the_sun_sits_within_a_cell_of_the_subsolar_point_at_the_range_s_midpoint() {
    let buf = frame(&sized(regions_monday(), 120, 48));
    let suns: Vec<(i32, i32)> = (1..25)
        .flat_map(|y| (0..120).map(move |x| (x, y)))
        .filter(|(x, y)| buf[(*x, *y)].symbol() == "☼")
        .map(|(x, y)| (i32::from(x), i32::from(y)))
        .collect();
    // 3pm AEDT on Mon 26 Oct is 04:00 UTC: the sun is over 120E less the equation of time
    // (about 16 minutes, so 4 degrees), at a declination of about 12.6S.
    let (x, y) = cell_of(-12.6, 116.0);
    assert_eq!(suns.len(), 1, "{suns:?}");
    let (sx, sy) = suns[0];
    assert!(
        (sx - x).abs() <= 1 && (sy - y).abs() <= 1,
        "sun at {:?}, expected near {:?}",
        suns[0],
        (x, y)
    );
}

/// The app as a terminal of `cols` by `rows` reports itself.
fn sized(app: App, cols: u16, rows: u16) -> App {
    update(app, Msg::Resize(cols, rows)).0
}

/// Whether any row but an answer row (whose fills are Braille too) draws Braille land.
fn shows_map(buf: &Buffer) -> bool {
    text(buf)
        .lines()
        .filter(|line| !line.contains('≡'))
        .any(|line| line.chars().any(|c| ('⠁'..='⣿').contains(&c)))
}

/// Renders `app` at the size it last reported.
fn frame(app: &App) -> Buffer {
    render(app, app.size.0, app.size.1)
}

#[test]
fn at_120_by_36_the_map_shows_and_m_hides_it_until_m_again() {
    let app = sized(team_monday(), 120, 36);
    assert!(shows_map(&frame(&app)));
    let hidden = press(app, &chars("m"));
    assert!(!shows_map(&frame(&hidden)), "{}", text(&frame(&hidden)));
    assert!(shows_map(&frame(&press(hidden, &chars("m")))));
}

#[test]
fn where_there_is_no_room_m_opens_the_map_as_an_overlay_until_esc() {
    for (cols, rows) in [(120, 24), (80, 24)] {
        let app = sized(team_monday(), cols, rows);
        assert!(!shows_map(&frame(&app)), "{cols}x{rows}");
        let open = press(app, &chars("m"));
        let buf = frame(&open);
        assert!(shows_map(&buf), "{cols}x{rows}\n{}", text(&buf));
        assert!(
            text(&buf).contains(" map "),
            "{cols}x{rows}\n{}",
            text(&buf)
        );
        let closed = press(open, &[Key::Esc]);
        assert!(!shows_map(&frame(&closed)), "{cols}x{rows}");
    }
}

#[test]
fn a_resize_recomputes_the_choice_and_a_group_switch_never_does() {
    let hidden = press(sized(team_monday(), 120, 36), &chars("m"));
    for switched in [
        press(hidden.clone(), &chars("3")),
        press(hidden.clone(), &chars("40")),
    ] {
        assert!(!shows_map(&frame(&switched)));
    }
    let shown = sized(team_monday(), 120, 36);
    let shrunk = sized(shown.clone(), 120, 24);
    assert!(!shows_map(&frame(&shrunk)), "{}", text(&frame(&shrunk)));
    assert!(shows_map(&frame(&press(shown, &chars("34")))));
    assert!(
        shows_map(&frame(&sized(hidden, 120, 36))),
        "a resize drops the flip"
    );
}

/// The footer row of `app`'s frame at the size it last reported.
fn footer_of(app: &App) -> String {
    row(&frame(app), app.size.1 - 1)
}

#[test]
fn under_ascii_glyphs_the_footer_never_offers_m_which_does_nothing() {
    let ascii = go_to(
        App::new(config("glyphs = \"ascii\"\n"), now()),
        "26 oct 5-6pm",
    );
    for (cols, rows) in [(120, 36), (120, 24)] {
        let app = sized(ascii.clone(), cols, rows);
        assert!(!footer_of(&app).contains("m map"), "{}", footer_of(&app));
        assert_eq!(frame(&press(app.clone(), &chars("m"))), frame(&app));
    }
}

#[test]
fn m_ranks_below_h_l_only_while_the_hiding_rule_finds_no_room() {
    let shown = sized(team_monday(), 100, 36);
    let hidden = press(shown.clone(), &chars("m"));
    assert_eq!(footer_of(&hidden), footer_of(&shown));
    let cramped = sized(team_monday(), 100, 24);
    assert!(
        footer_of(&cramped).contains("m map"),
        "{}",
        footer_of(&cramped)
    );
    assert!(
        !footer_of(&shown).contains("m map"),
        "{}",
        footer_of(&shown)
    );
}

#[test]
fn main_keys_still_work_under_the_map_overlay() {
    let open = press(sized(team_monday(), 120, 24), &chars("m"));
    let scrubbed = press(open, &chars("l"));
    assert_eq!(
        scrubbed.say_line(),
        press(team_monday(), &chars("l")).say_line()
    );
    assert!(shows_map(&frame(&scrubbed)), "{}", text(&frame(&scrubbed)));
}

/// Paul in Oslo, with Ari in Reykjavik counted in the only group: every marker sits above 59N.
fn northern() -> App {
    let config = Config::parse(
        "[home]\nname = \"Paul\"\ncity = \"Oslo\"\n\n\
         [[person]]\nname = \"Ari\"\ncity = \"Reykjavik\"\n\n\
         [[group]]\nname = \"north\"\npeople = [\"Ari\"]\n",
    )
    .unwrap();
    App::new(config, now())
}

#[test]
fn the_band_is_centred_on_home_and_the_counted_markers_within_the_map_s_bounds() {
    // 14 map rows at 120 columns show 84 degrees, so a band centred on the equator would end
    // at 42N; centred on 62N it is held at the map's northern edge instead.
    let app = sized(northern(), 120, 30);
    let map = rows_above(&frame(&app), "Sat 24 Oct CEST");
    for label in ["Oslo ", "Reykjavík "] {
        assert!(map.contains(label), "no {label:?} in\n{map}");
    }
}

/// Each map cell that shows land or sea rather than a marker, the sun or a (bold) label, with
/// its symbol.
fn land(buf: &Buffer, rows: std::ops::Range<u16>) -> Vec<((u16, u16), String)> {
    rows.flat_map(|y| (0..buf.area.width).map(move |x| (x, y)))
        .filter(|cell| !buf[*cell].modifier.contains(ratatui::style::Modifier::BOLD))
        .map(|cell| (cell, buf[cell].symbol().to_owned()))
        .filter(|(_, symbol)| {
            symbol
                .chars()
                .all(|c| c == ' ' || ('\u{2800}'..='\u{28ff}').contains(&c))
        })
        .collect()
}

#[test]
fn a_group_switch_never_moves_the_map() {
    let team = sized(team_monday(), 120, 36);
    let regions = press(team.clone(), &chars("3"));
    let (before, after) = (frame(&team), frame(&regions));
    let sun = |buf: &Buffer| {
        (1..13)
            .flat_map(|y| (0..120).map(move |x| (x, y)))
            .find(|cell| buf[*cell].symbol() == "☼")
    };
    assert_eq!(sun(&before), sun(&after));
    let unmarked: Vec<_> = land(&after, 1..13);
    let shared: Vec<_> = land(&before, 1..13)
        .into_iter()
        .filter(|(cell, _)| unmarked.iter().any(|(other, _)| other == cell))
        .collect();
    let moved: Vec<_> = shared
        .iter()
        .filter(|cell| !unmarked.contains(cell))
        .collect();
    assert!(moved.is_empty(), "{moved:?}");
}

/// The map rows of `app`'s frame at the size it last reported, from the header to the ruler.
fn map_of(app: &App) -> String {
    let buf = frame(app);
    (1..y_of(&buf, " AEDT    ").min(y_of(&buf, "Mon 26 Oct AEDT")))
        .map(|y| row(&buf, y))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn z_opens_map_mode_while_the_map_is_drawn_and_its_footer_names_its_keys_until_esc() {
    let shown = sized(team_monday(), 120, 36);
    let mapping = press(shown.clone(), &chars("z"));
    assert_eq!(
        footer_of(&mapping),
        "map  h/j/k/l pan  +/- zoom  0 reset  esc done  ? help"
    );
    let left = press(mapping, &[Key::Esc]);
    assert_eq!(footer_of(&left), footer_of(&shown));
    let hidden = sized(team_monday(), 120, 24);
    assert_eq!(frame(&press(hidden.clone(), &chars("z"))), frame(&hidden));
}

#[test]
fn map_mode_works_on_the_overlay_and_esc_leaves_it_before_closing_the_overlay() {
    let overlay = press(sized(team_monday(), 120, 24), &chars("mz"));
    assert!(
        footer_of(&overlay).starts_with("map  "),
        "{}",
        footer_of(&overlay)
    );
    let left = press(overlay, &[Key::Esc]);
    assert!(shows_map(&frame(&left)), "the overlay is still open");
    assert!(!footer_of(&left).starts_with("map  "));
    assert!(!shows_map(&frame(&press(left, &[Key::Esc]))));
}

#[test]
fn h_and_l_pan_across_and_eight_steps_go_round_the_world() {
    let mapping = press(sized(team_monday(), 120, 36), &chars("z"));
    let start = map_of(&mapping);
    let east = press(mapping.clone(), &chars("l"));
    assert_ne!(map_of(&east), start);
    assert_eq!(map_of(&press(east, &chars("h"))), start);
    // A step is an eighth of the 120 columns, so eight of them wrap the world round once.
    assert_eq!(map_of(&press(mapping, &chars("llllllll"))), start);
}

#[test]
fn k_pans_north_until_the_map_s_edge_and_holds_there() {
    let regions = press(sized(regions_monday(), 120, 36), &chars("z"));
    assert!(!map_of(&regions).contains("UK closed"));
    let north = press(regions.clone(), &chars("k"));
    assert!(map_of(&north).contains("UK closed"), "{}", map_of(&north));
    let edge = press(regions, &chars("kkkkkkkkkk"));
    assert_eq!(map_of(&press(edge.clone(), &chars("kkk"))), map_of(&edge));
    assert_ne!(
        map_of(&press(edge.clone(), &chars("j"))),
        map_of(&edge),
        "held, not wound up"
    );
}

#[test]
fn minus_zooms_out_until_the_whole_map_shows_and_plus_zooms_back_in() {
    let mapping = press(sized(regions_monday(), 120, 36), &chars("z"));
    let start = map_of(&mapping);
    let out = press(mapping.clone(), &chars("----------"));
    for label in ["UK closed", "Melbourne", "US open", "AU open"] {
        assert!(
            map_of(&out).contains(label),
            "no {label:?} in\n{}",
            map_of(&out)
        );
    }
    let closer = press(mapping, &chars("+"));
    assert_ne!(map_of(&closer), start);
    assert_eq!(map_of(&press(closer, &chars("-"))), start);
}

#[test]
fn zero_resets_the_view_which_otherwise_stays_after_esc_until_a_resize() {
    let mapping = press(sized(team_monday(), 120, 36), &chars("z"));
    let start = map_of(&mapping);
    let moved = press(mapping, &chars("l+k"));
    let left = press(moved.clone(), &[Key::Esc]);
    assert_eq!(map_of(&left), map_of(&moved), "the view stays after esc");
    assert_eq!(map_of(&press(moved, &chars("0"))), start);
    assert_eq!(
        map_of(&sized(left, 120, 36)),
        start,
        "a resize resets the view"
    );
}

#[test]
fn a_resize_that_hides_the_map_leaves_map_mode() {
    let mapping = press(sized(team_monday(), 120, 36), &chars("z"));
    let shrunk = sized(mapping, 120, 24);
    assert!(
        !footer_of(&shrunk).starts_with("map  "),
        "{}",
        footer_of(&shrunk)
    );
    assert!(
        footer_of(&shrunk).contains("h/l time"),
        "{}",
        footer_of(&shrunk)
    );
}

#[test]
fn help_from_map_mode_lists_its_keys_and_closing_it_returns_to_map_mode() {
    let mapping = press(sized(team_monday(), 120, 36), &chars("z"));
    let help = press(mapping.clone(), &chars("?"));
    let shown = text(&frame(&help));
    for line in [
        "Pan the map",
        "Zoom in",
        "Back to the default view",
        "Leave map mode",
    ] {
        assert!(shown.contains(line), "no {line:?} in\n{shown}");
    }
    assert_eq!(press(help, &[Key::Esc]).mode, mapping.mode);
}

/// The cell showing `glyph` on the map rows of `app`'s frame.
fn cell_with(app: &App, glyph: &str, rows: std::ops::Range<u16>) -> Option<(u16, u16)> {
    let buf = frame(app);
    rows.flat_map(|y| (0..buf.area.width).map(move |x| (x, y)))
        .find(|cell| buf[*cell].symbol() == glyph)
}

#[test]
fn the_sun_and_the_markers_move_with_the_view_and_leave_it_when_it_moves_away() {
    let mapping = press(sized(team_monday(), 120, 36), &chars("z"));
    let east = press(mapping.clone(), &chars("l"));
    for glyph in ["☼", "⌂"] {
        let (x, y) = cell_with(&mapping, glyph, 1..13).unwrap();
        // A step east is 15 of the 120 columns, so everything on the map shifts 15 west.
        assert_eq!(cell_with(&east, glyph, 1..13), Some((x - 15, y)), "{glyph}");
    }
    // Zoomed four steps in on 0E, the view spans 90 degrees and Melbourne (145E) is outside it.
    let close = press(mapping, &chars("++++"));
    assert_eq!(cell_with(&close, "⌂", 1..13), None);
    assert!(!map_of(&close).contains("Melbourne"), "{}", map_of(&close));
}

#[test]
fn a_label_never_draws_past_the_overlay_s_border_wherever_the_view_pans() {
    let mut app = press(sized(team_monday(), 120, 24), &chars("mz"));
    for _ in 0..16 {
        app = press(app, &chars("l"));
        let buf = frame(&app);
        let border = buf
            .content
            .iter()
            .position(|cell| cell.symbol() == "╮")
            .map(|i| i as u16 % buf.area.width)
            .unwrap();
        let top = (0..buf.area.height)
            .find(|y| buf[(border, *y)].symbol() == "╮")
            .unwrap();
        let bottom = (top..buf.area.height)
            .find(|y| buf[(border, *y)].symbol() == "╯")
            .unwrap();
        for y in top + 1..bottom {
            assert_eq!(buf[(border, y)].symbol(), "│", "row {y}\n{}", text(&buf));
        }
    }
}
