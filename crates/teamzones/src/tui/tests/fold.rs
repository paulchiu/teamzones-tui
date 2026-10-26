//! Folding: a terminal too short for every row folds the blocks, never the answer rows.

use super::*;
use teamzones_core::question::Row;

/// `app` on a `cols` by `rows` terminal, as the loop tells it after a resize.
fn sized(app: App, cols: u16, rows: u16) -> App {
    update(app, Msg::Resize(cols, rows)).0
}

#[test]
fn at_80_by_20_the_fold_row_counts_what_it_hides_and_both_answer_rows_and_the_times_box_show() {
    let app = sized(team_monday(), 80, 20);
    let buf = render(&app, 80, 20);
    let who = y_of(&buf, " who ");
    assert!(row(&buf, who + 1).starts_with("⌂ Paul"), "{}", text(&buf));
    assert!(row(&buf, who + 2).starts_with("● Priya"), "{}", text(&buf));
    assert_eq!(row(&buf, who + 3), "○ +5 more, 3 counted", "{}", text(&buf));
    assert!(
        row(&buf, who + 4).starts_with("≡ in hours"),
        "{}",
        text(&buf)
    );
    assert!(row(&buf, y_of(&buf, " where ") + 1).starts_with("≡ where"));
    assert_eq!(say_line(&buf), "5-6pm AEDT / 4-5pm AEST / 8-9am SAST");
    assert!(row(&buf, 19).starts_with("h/l"), "{}", text(&buf));
    insta::assert_snapshot!("team_folded_80x20", text(&buf));
}

#[test]
fn at_80_by_20_switching_groups_never_moves_a_block() {
    let team = sized(team_monday(), 80, 20);
    let regions = press(team.clone(), &chars("3"));
    let deploy = press(regions.clone(), &chars("4"));
    let [team, regions, deploy] = [&team, &regions, &deploy].map(|app| render(app, 80, 20));
    assert_eq!(anchors(&team), anchors(&regions));
    assert_eq!(anchors(&regions), anchors(&deploy));
}

#[test]
fn enter_on_the_fold_row_lists_the_rows_it_folds_and_enter_again_closes_the_list() {
    let app = press(sized(team_monday(), 80, 20), &chars("jj"));
    let open = press(app, &[Key::Enter]);
    let buf = render(&open, 80, 20);
    let listed: Vec<u16> = ["Sam", "Thandi", "Johan", "Ellie", "Marcus"]
        .iter()
        .map(|name| y_of(&buf, &format!(" {name} ")))
        .collect();
    assert!(
        listed.windows(2).all(|pair| pair[0] < pair[1]),
        "{}",
        text(&buf)
    );
    insta::assert_snapshot!("fold_list_80x20", text(&buf));
    let closed = render(&press(open, &[Key::Enter]), 80, 20);
    assert!(!text(&closed).contains("Thandi"), "{}", text(&closed));
}

#[test]
fn esc_closes_the_fold_list_before_anything_else() {
    let app = go_to(sized(fixture(), 80, 20), "26 oct 5-6pm");
    let open = press(app, &[Key::Char('j'), Key::Char('j'), Key::Enter]);
    let closed = press(open, &[Key::Esc]);
    assert!(!text(&render(&closed, 80, 20)).contains("Thandi"));
    assert!(closed.range.is_some(), "esc closed the list, not the range");
}

#[test]
fn a_resize_that_folds_the_focused_row_moves_focus_onto_the_fold_row_holding_it() {
    let app = focus_on(team_monday(), "Marcus");
    let folded = sized(app, 80, 20);
    let sam = Row::Person(folded.roster().person_named("Sam").unwrap());
    assert_eq!(
        folded.focus, sam,
        "the fold row holding Marcus stands for Sam"
    );
    assert!(folded.focused_more().is_some());
}

#[test]
fn at_30_by_8_the_screen_says_terminal_too_small_and_q_still_quits() {
    let app = sized(fixture(), 30, 8);
    let buf = render(&app, 30, 8);
    assert!(text(&buf).contains("terminal too small"), "{}", text(&buf));
    assert!(!press(app, &chars("q")).running);
}

/// The team frame at 80x20 read from a config file, with focus on its `+5 more` line.
fn on_the_fold_row() -> App {
    let app = App::new(config(""), now()).with_source(Some(FIXTURE.to_owned()));
    let app = press(sized(go_to(app, "26 oct 5-6pm"), 80, 20), &chars("jj"));
    assert!(app.focused_more().is_some(), "focus is on the fold row");
    app
}

#[test]
fn row_keys_on_the_fold_row_leave_the_roster_and_the_marks_alone() {
    let app = on_the_fold_row();
    let (after, effects) = press_all(app.clone(), &chars("d "));
    assert!(effects.is_empty(), "{effects:?}");
    assert_eq!(after.roster(), app.roster());
    assert_eq!(after.marks, app.marks);
    let edited = press(app, &chars("e"));
    assert!(!matches!(edited.mode, super::super::app::Mode::Picker(_)));
}

#[test]
fn the_detail_line_on_the_fold_row_describes_the_fold_not_its_first_row() {
    let buf = render(&on_the_fold_row(), 80, 20);
    let detail = row(&buf, y_of(&buf, "▶"));
    assert!(detail.starts_with("▶ +5 more, 3 counted"), "{detail}");
    assert!(!detail.contains("Brisbane"), "{detail}");
}
