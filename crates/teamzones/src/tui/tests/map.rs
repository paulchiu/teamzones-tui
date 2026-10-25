//! The map: its labels, the night side and the sun, and when it shows.

use super::*;

/// Regions on Mon 26 Oct with a 2-4pm AEDT range, as the screen spec's second mockup draws it.
fn regions_monday() -> App {
    go_to(press(fixture(), &chars("3")), "26 oct 2-4pm")
}

/// The map's rows: everything between the header and the ruler.
fn map_text(buf: &Buffer) -> String {
    let ruler = y_of(buf, "Mon 26 Oct AEDT");
    (1..ruler)
        .map(|y| row(buf, y))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn the_map_labels_home_and_each_counted_place_with_its_state_over_the_range() {
    let map = map_text(&render(&regions_monday(), 120, 36));
    for label in [
        "UK closed 3-5am",
        "US open 10pm-12am",
        "AU open 2-4pm",
        "Melbourne 2-4pm",
    ] {
        assert!(map.contains(label), "no {label:?} in\n{map}");
    }
    insta::assert_snapshot!("regions_map_120x36", map);
}

/// The cell a latitude and longitude falls in on a 120 by 12 map one row below the header,
/// by plain arithmetic: 360 degrees over 120 columns, and 80N to 58S over 12 rows.
fn cell_of(lat: f64, lon: f64) -> (i32, i32) {
    let x = ((lon + 180.0) / 3.0).floor() as i32;
    let y = 1 + ((80.0 - lat) / 11.5).floor() as i32;
    (x, y)
}

#[test]
fn the_sun_sits_within_a_cell_of_the_subsolar_point_at_the_range_s_midpoint() {
    let buf = render(&regions_monday(), 120, 36);
    let suns: Vec<(i32, i32)> = (1..13)
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
