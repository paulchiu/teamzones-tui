//! The clock list below 60 columns: a two-line row per city and per place.

use super::*;
use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use teamzones_core::question::Row;

use crate::tui::mouse::Pointer;

/// `app` on a `cols` by `rows` terminal, as the loop tells it after a resize.
fn sized(app: App, cols: u16, rows: u16) -> App {
    update(app, Msg::Resize(cols, rows)).0
}

/// The team frame of the issue's mockup: 44 by 30, Mon 26 Oct 5-6pm.
fn team_44() -> App {
    sized(team_monday(), 44, 30)
}

fn click(app: App, (x, y): (u16, u16)) -> App {
    let event = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: x,
        row: y,
        modifiers: KeyModifiers::NONE,
    };
    update(
        app,
        Msg::Mouse(Pointer::from_crossterm(event, chrono::TimeDelta::zero())),
    )
    .0
}

fn person(app: &App, name: &str) -> Row {
    Row::Person(app.roster().person_named(name).unwrap())
}

#[test]
fn a_click_on_either_line_of_a_city_row_focuses_its_first_person() {
    let app = team_44();
    let buf = render(&app, 44, 30);
    let small = y_of(&buf, "● Priya, ● Sam");
    for y in [small, small + 1] {
        let clicked = click(app.clone(), (20, y));
        assert_eq!(clicked.focus, person(&clicked, "Priya"), "line {y}");
    }
    let uk = y_of(&buf, "◇ London");
    let clicked = click(app, (2, uk + 1));
    let id = clicked.roster().place_named("UK").unwrap();
    assert_eq!(clicked.focus, Row::Place(id));
}

/// The columns of row `y` drawn with the selection, as a string of the symbols there.
fn selected_text(buf: &Buffer, y: u16) -> String {
    shaded(buf, y, 0)
        .into_iter()
        .map(|x| buf[(x, y)].symbol())
        .collect()
}

#[test]
fn j_and_k_move_through_people_in_row_order_then_places_and_the_focused_name_is_marked() {
    let mut app = team_44();
    let mut order = vec![focus_name(&app)];
    for _ in 0..10 {
        app = press(app, &chars("j"));
        order.push(focus_name(&app));
    }
    assert_eq!(
        order,
        [
            "Paul", "Priya", "Sam", "Thandi", "Johan", "Ellie", "Marcus", "AU", "UK", "US", "US"
        ]
    );
    let sam = focus_on(team_44(), "Sam");
    let buf = render(&sam, 44, 30);
    let y = y_of(&buf, "● Priya, ● Sam");
    assert_eq!(selected_text(&buf, y).trim(), "● Sam", "{}", text(&buf));
    let back = press(sam, &chars("k"));
    assert_eq!(focus_name(&back), "Priya");
}

#[test]
fn space_and_e_act_on_the_focused_person_of_a_shared_city_row() {
    let from_file = App::new(config(""), now()).with_source(Some(FIXTURE.to_owned()));
    let sam = focus_on(sized(go_to(from_file, "26 oct 5-6pm"), 44, 30), "Sam");
    let dropped = press(sam.clone(), &chars(" "));
    assert!(
        !dropped
            .marks
            .people
            .contains(&dropped.roster().person_named("Sam").unwrap())
    );
    let buf = render(&dropped, 44, 30);
    assert!(row(&buf, 2).starts_with("team -Sam"), "{}", text(&buf));
    assert!(
        row(&buf, y_of(&buf, "Brisbane") - 1).contains("● Priya, ○ Sam"),
        "{}",
        text(&buf)
    );
    let edited = press(sam, &chars("e"));
    assert!(
        matches!(&edited.mode, super::super::app::Mode::Picker(_)),
        "{:?}",
        edited.mode
    );
}

#[test]
fn the_title_line_names_the_group_its_drift_and_its_place_and_digits_still_switch() {
    let app = team_44();
    let buf = render(&app, 44, 30);
    let title = y_of(&buf, "1 of 5");
    assert_eq!(row(&buf, title), format!("team{}1 of 5", " ".repeat(34)));
    assert!(
        buf[(0, title)]
            .modifier
            .contains(ratatui::style::Modifier::BOLD)
    );
    let drifted = press(focus_on(app.clone(), "UK"), &chars(" "));
    let buf = render(&drifted, 44, 30);
    assert!(
        row(&buf, title).starts_with("team +@UK"),
        "{}",
        row(&buf, title)
    );
    let theme = Theme::resolve(&drifted.config.theme, TerminalEnv::default());
    assert_eq!(
        buf[(6, title)].fg,
        theme.fg(super::super::theme::Role::Warn).fg.unwrap()
    );
    for (keys, name, position) in [("3", "regions", "3 of 5"), ("0", "all", "5 of 5")] {
        let buf = render(&press(app.clone(), &chars(keys)), 44, 30);
        let line = row(&buf, title);
        assert!(
            line.starts_with(name) && line.ends_with(position),
            "{keys}: {line}"
        );
    }
}

#[test]
fn home_comes_first_then_counted_cities_east_to_west_then_the_rest_under_a_not_in_rule() {
    let deploy = press(team_44(), &chars("4"));
    let buf = render(&deploy, 44, 30);
    let ys: Vec<u16> = [
        "⌂ Paul",
        "● Priya, ○ Sam",
        "● Thandi, ○ Johan",
        "≡ in hours",
        " not in deploy ",
        "○ Ellie",
        "○ Marcus",
    ]
    .iter()
    .map(|needle| y_of(&buf, needle))
    .collect();
    assert!(
        ys.windows(2).all(|pair| pair[0] < pair[1]),
        "{}",
        text(&buf)
    );
    let theme = Theme::resolve(&deploy.config.theme, TerminalEnv::default());
    let dim = theme.fg(super::super::theme::Role::Dim).fg.unwrap();
    let text_role = theme.fg(super::super::theme::Role::Text).fg.unwrap();
    let cell = |needle: &str, offset: u16| {
        let y = y_of(&buf, needle);
        let x = (0..44)
            .find(|x| {
                (*x..44)
                    .map(|x| buf[(x, y)].symbol())
                    .collect::<String>()
                    .starts_with(needle)
            })
            .unwrap();
        buf[(x + offset, y)].fg
    };
    assert_eq!(cell("○ Sam", 2), dim, "Sam is uncounted in a counted city");
    assert_eq!(cell("● Priya", 2), text_role);
    assert_eq!(cell("London", 0), dim, "London is not in deploy");
    assert_eq!(cell("Brisbane", 0), text_role);
}

#[test]
fn each_city_row_reads_its_day_offset_people_and_state_over_its_city_and_time() {
    let buf = render(&team_44(), 44, 30);
    let brisbane = y_of(&buf, "● Priya, ● Sam");
    assert_eq!(
        [row(&buf, brisbane), row(&buf, brisbane + 1)],
        [
            format!("Mon, -1h  ● Priya, ● Sam{}working", " ".repeat(13)),
            format!("Brisbane{}4-5pm", " ".repeat(31)),
        ]
    );
    for x in [0, 39] {
        assert!(
            buf[(x, brisbane + 1)]
                .modifier
                .contains(ratatui::style::Modifier::BOLD),
            "column {x}"
        );
    }
    let ellie = row(&buf, y_of(&buf, "○ Ellie"));
    assert!(
        ellie.starts_with("Mon, -11h  ○ Ellie") && ellie.ends_with("night"),
        "{ellie}"
    );
}

#[test]
fn a_row_whose_people_differ_reads_mixed_and_a_day_off_reads_off() {
    let app = App::new(
        config("\n[[person]]\nname = \"Kai\"\ncity = \"Brisbane\"\n"),
        now(),
    );
    let app = sized(go_to(app, "26 oct 5-6pm"), 44, 40);
    let buf = render(&app, 44, 40);
    let brisbane = row(&buf, y_of(&buf, "Priya"));
    assert!(brisbane.ends_with("mixed"), "{}", text(&buf));
    let today = render(&sized(fixture(), 44, 40), 44, 40);
    let paul = row(&today, y_of(&today, "⌂ Paul"));
    assert!(
        paul.starts_with("Sat, +0h") && paul.ends_with("off"),
        "{paul}"
    );
}

#[test]
fn the_answer_rows_keep_their_place_on_one_line_and_never_fold() {
    for rows in [14, 20, 30, 40] {
        let app = sized(team_monday(), 44, rows);
        let buf = render(&app, 44, rows);
        let who = y_of(&buf, "≡ in hours");
        assert!(
            row(&buf, who).ends_with("5/5"),
            "{rows} rows: {}",
            text(&buf)
        );
        let place = y_of(&buf, "≡ where");
        assert!(
            row(&buf, place).ends_with("--"),
            "{rows} rows: {}",
            text(&buf)
        );
        assert!(
            row(&buf, place + 1).starts_with("Mon, +0h  ◇ Melbourne"),
            "{rows} rows: {}",
            text(&buf)
        );
    }
}

#[test]
fn each_place_is_a_two_line_row_and_space_counts_one_the_group_does_not_ask() {
    let app = team_44();
    let buf = render(&app, 44, 30);
    assert!(row(&buf, y_of(&buf, " where ")).starts_with(" where  not asked ──"));
    let au = y_of(&buf, "◇ Melbourne");
    assert_eq!(
        [row(&buf, au), row(&buf, au + 1)],
        [
            format!("Mon, +0h  ◇ Melbourne{}busy", " ".repeat(19)),
            format!("AU{}5-6pm", " ".repeat(37)),
        ]
    );
    let theme = Theme::resolve(&app.config.theme, TerminalEnv::default());
    let busy = &buf[(40, au)];
    assert_eq!(
        busy.fg,
        theme.fg(super::super::theme::Role::Warn).fg.unwrap()
    );
    assert!(busy.modifier.contains(ratatui::style::Modifier::BOLD));
    let counted = press(focus_on(app, "UK"), &chars(" "));
    let buf = render(&counted, 44, 30);
    let uk = y_of(&buf, "◆ London");
    assert!(uk < y_of(&buf, "≡ open"), "{}", text(&buf));
    assert!(row(&buf, uk).ends_with("closed"), "{}", text(&buf));
}

#[test]
fn rules_sit_between_city_rows_with_room_and_go_first_when_short() {
    let rule = "─".repeat(44);
    let rules = |rows: u16| {
        let buf = render(&sized(team_monday(), 44, rows), 44, rows);
        (0..rows).filter(|y| row(&buf, *y) == rule).count()
    };
    // Two between the counted cities and one between the two others.
    assert_eq!(rules(31), 3);
    assert_eq!(rules(30), 0);
    let buf = render(&sized(team_monday(), 44, 31), 44, 31);
    let paul = y_of(&buf, "⌂ Paul");
    assert_eq!(row(&buf, paul + 2), rule);
    assert!(row(&buf, paul + 3).contains("● Priya"));
}

#[test]
fn short_of_rows_the_city_rows_fold_and_enter_lists_them() {
    let app = sized(team_monday(), 44, 20);
    let buf = render(&app, 44, 20);
    let more = y_of(&buf, "+4 more");
    assert_eq!(row(&buf, more), "○ +4 more, 2 counted", "{}", text(&buf));
    let open = press(focus_on(app, "Priya"), &[Key::Enter]);
    assert!(open.fold_open);
    let buf = render(&open, 44, 20);
    assert!(text(&buf).contains("Thandi"), "{}", text(&buf));
}

#[test]
fn the_times_box_notice_and_footer_keep_to_the_bottom_and_y_copies_the_wide_line() {
    for rows in [24, 30, 45] {
        let app = sized(team_monday(), 44, rows);
        let buf = render(&app, 44, rows);
        assert!(
            row(&buf, rows - 5).starts_with("╭ times for team"),
            "{}",
            text(&buf)
        );
        assert!(row(&buf, rows - 1).starts_with(". now"), "{}", text(&buf));
        let (_, effects) = press_all(app, &chars("y"));
        let wide = press_all(sized(team_monday(), 120, 36), &chars("y")).1;
        assert_eq!(copied(&effects), copied(&wide));
    }
}

#[test]
fn below_60_columns_at_any_height_from_10_rows_the_blocks_are_the_clock_list() {
    for (cols, rows) in (10..=60).flat_map(|rows| [(40, rows), (59, rows)]) {
        let buf = render(&sized(team_monday(), cols, rows), cols, rows);
        let frame = text(&buf);
        assert!(row(&buf, 2).ends_with("1 of 5"), "{cols}x{rows}:\n{frame}");
        assert!(
            !frame.contains(['▓', '░', '┣', '┃']),
            "{cols}x{rows}:\n{frame}"
        );
    }
    for rows in [24, 40, 60] {
        let buf = render(&sized(team_monday(), 60, rows), 60, rows);
        assert!(!text(&buf).contains("1 of 5"), "60x{rows}");
        assert!(text(&buf).contains('▓'), "60x{rows}");
    }
    let tiny = render(&sized(team_monday(), 39, 30), 39, 30);
    assert!(text(&tiny).contains("terminal too small"));
}

#[test]
fn the_ascii_glyphs_and_no_color_both_draw_a_legible_list() {
    let ascii = App::new(config("glyphs = \"ascii\"\n"), now());
    let ascii = sized(go_to(ascii, "26 oct 5-6pm"), 44, 31);
    let buf = render(&ascii, 44, 31);
    let frame = text(&buf);
    for needle in [
        "^ Paul",
        "* Priya, * Sam",
        "- not in team -",
        "o Ellie",
        "- Melbourne",
    ] {
        assert!(frame.contains(needle), "{needle}:\n{frame}");
    }
    let header_and_list: String = frame.lines().skip(1).take(25).collect();
    assert!(header_and_list.is_ascii(), "{frame}");
    let mono = render_with(&focus_on(team_44(), "Sam"), 44, 30, NO_COLOR);
    let y = y_of(&mono, "● Priya, ● Sam");
    assert_eq!(selected_text(&mono, y).trim(), "● Sam");
}

/// The screen spec's clock list mockup: the fenced frame whose second line is the one-line slider.
fn spec_mockup() -> String {
    SCREEN_SPEC
        .split("```\n")
        .find(|block| {
            block
                .lines()
                .nth(1)
                .is_some_and(|line| line.starts_with("▸ Mon 26 Oct"))
        })
        .expect("the screen spec has the clock list mockup")
        .trim_end_matches('\n')
        .to_owned()
}

#[test]
fn the_44_by_30_team_frame_is_the_screen_spec_s_mockup() {
    assert_eq!(text(&render(&team_44(), 44, 30)), spec_mockup());
}

#[test]
fn switching_groups_never_moves_the_where_heading_or_the_times_box() {
    for rows in [20, 30, 31, 40] {
        let at = |keys: &str| {
            let buf = render(
                &press(sized(team_monday(), 44, rows), &chars(keys)),
                44,
                rows,
            );
            (y_of(&buf, " where "), y_of(&buf, " times for "))
        };
        let team = at("");
        for keys in ["2", "3", "4", "0"] {
            assert_eq!(at(keys), team, "{rows} rows, group {keys}");
        }
    }
}

#[test]
fn a_people_line_too_long_for_its_row_ends_in_a_dot_before_the_state() {
    let long = "\n[[person]]\nname = \"Bartholomew\"\ncity = \"Brisbane\"\nhours = \"9am-5pm\"\n";
    let app = sized(go_to(App::new(config(long), now()), "26 oct 5-6pm"), 40, 40);
    let buf = render(&app, 40, 40);
    let line = row(&buf, y_of(&buf, "● Priya"));
    assert_eq!(
        line,
        "Mon, -1h  ● Priya, ● Sam, ○ Bar. working",
        "{}",
        text(&buf)
    );
}

#[test]
fn while_travelling_home_shares_the_travel_city_s_row_and_its_own_city_follows() {
    let travel = "\n[travel]\nhere = \"London\"\nuntil = 2026-11-01\n";
    let app = App::new(config(travel), "2026-10-26T06:00:00Z".parse().unwrap());
    let buf = render(&sized(app, 44, 40), 44, 40);
    let london = y_of(&buf, "✈ Paul");
    assert!(
        row(&buf, london).starts_with("Mon, +0h  ✈ Paul, ○ Ellie"),
        "{}",
        text(&buf)
    );
    assert_eq!(
        row(&buf, london + 1),
        format!("London{}6am", " ".repeat(35))
    );
    let melbourne = row(&buf, london + 3);
    assert!(melbourne.starts_with("Mon, +11h  ⌂ home"), "{}", text(&buf));
}
