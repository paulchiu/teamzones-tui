//! Travel: home moved to the travel city, with its own city as a reference row.

use super::*;

const TRAVEL: &str = "\n[travel]\nhere = \"London\"\nuntil = 2026-11-01\n";

/// The fixture travelling in London on Mon 26 Oct, 6am GMT.
fn in_london() -> App {
    App::new(config(TRAVEL), "2026-10-26T06:00:00Z".parse().unwrap())
}

#[test]
fn while_travelling_the_header_names_the_travel_city_and_the_axis_is_its_zone() {
    let buf = render(&in_london(), 120, 30);
    assert!(
        row(&buf, 0).starts_with("teamzones   now Mon 26 Oct 6am GMT"),
        "{}",
        row(&buf, 0)
    );
    assert!(row(&buf, 0).ends_with("✈ London GMT"), "{}", row(&buf, 0));
    assert!(
        row(&buf, 1).starts_with("Mon 26 Oct GMT"),
        "{}",
        row(&buf, 1)
    );
}

#[test]
fn home_s_row_is_marked_here_and_its_own_city_follows_as_a_dim_uncounted_reference() {
    let app = in_london();
    let buf = render(&app, 120, 30);
    let who = y_of(&buf, " who ");
    assert!(row(&buf, who + 1).starts_with("✈ Paul"), "{}", text(&buf));
    assert!(
        row(&buf, who + 1).contains("6am GMT"),
        "{}",
        row(&buf, who + 1)
    );
    let reference = row(&buf, who + 2);
    assert!(reference.starts_with("⌂ Melbourne"), "{}", text(&buf));
    assert!(reference.contains("5pm AEDT"), "{reference}");
    let mark = &buf[(0, who + 2)];
    assert_eq!(
        mark.fg,
        buf[(0, y_of(&buf, "○ Ellie"))].fg,
        "dim like an uncounted row"
    );
    // Home and the four team members are counted; the reference row is not.
    assert!(
        row(&buf, y_of(&buf, "≡ in hours")).contains("/5"),
        "{}",
        text(&buf)
    );
}

#[test]
fn typed_times_are_read_in_the_travel_city_and_the_say_line_copies_home_s_zone() {
    let app = go_to(in_london(), "3pm");
    let buf = render(&app, 120, 30);
    assert_eq!(
        say_line(&buf),
        "3pm GMT / Tue 2am AEDT / Tue 1am AEST / 5pm SAST"
    );
}

#[test]
fn from_the_day_after_until_home_is_back_with_no_reference_row() {
    // Mon 2 Nov 6am GMT.
    let app = App::new(config(TRAVEL), "2026-11-02T06:00:00Z".parse().unwrap());
    let buf = render(&app, 120, 30);
    assert!(
        row(&buf, 0).ends_with("⌂ Melbourne AEDT"),
        "{}",
        row(&buf, 0)
    );
    assert!(!text(&buf).contains("✈"), "{}", text(&buf));
}

#[test]
fn a_running_screen_comes_home_when_the_clock_passes_until_in_the_travel_city() {
    let (app, _) = update(
        in_london(),
        Msg::Tick("2026-11-02T00:00:00Z".parse().unwrap()),
    );
    let buf = render(&app, 120, 30);
    assert!(
        row(&buf, 0).ends_with("⌂ Melbourne AEDT"),
        "{}",
        row(&buf, 0)
    );
    assert!(!text(&buf).contains("⌂ Melbourne    "), "{}", text(&buf));
}
