//! Mouse capture and every mouse equivalent in the keymap.

use super::*;

/// Presses `keys` and runs every effect through `modes`, as the shell does.
fn press_running(app: App, keys: &[Key], modes: &mut Recorder) -> App {
    let (app, effects) = press_all(app, keys);
    run_against(app, effects, std::path::Path::new("/nonexistent"), modes)
}

fn footer(app: &App) -> String {
    row(&render(app, 120, 36), 35)
}

#[test]
fn capital_m_releases_the_mouse_and_says_so_until_m_again_recaptures_it() {
    let mut terminal = Recorder::default();
    let released = press_running(fixture(), &chars("M"), &mut terminal);
    assert_eq!(terminal.capture, [false]);
    assert!(
        footer(&released).ends_with("mouse off"),
        "{}",
        footer(&released)
    );
    let recaptured = press_running(released, &chars("M"), &mut terminal);
    assert_eq!(terminal.capture, [false, true]);
    assert!(
        !footer(&recaptured).contains("mouse off"),
        "{}",
        footer(&recaptured)
    );
}

use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use teamzones_core::question::Row;

use crate::tui::keys::BUILT;
use crate::tui::mouse::{self, Pointer};

/// The app as a 120 by 36 terminal reports itself.
fn at_120(app: App) -> App {
    update(app, Msg::Resize(120, 36)).0
}

fn event(kind: MouseEventKind, (x, y): (u16, u16), modifiers: KeyModifiers) -> MouseEvent {
    MouseEvent {
        kind,
        column: x,
        row: y,
        modifiers,
    }
}

fn left(kind: fn(MouseButton) -> MouseEventKind, at: (u16, u16)) -> MouseEvent {
    event(kind(MouseButton::Left), at, KeyModifiers::NONE)
}

/// What a crossterm mouse event stands for on `app`'s screen, leaving out the note of where a
/// press was, which only spots double clicks.
fn msgs(app: &App, event: MouseEvent) -> Vec<Msg> {
    mouse::msgs(
        app,
        Pointer::from_crossterm(event, chrono::TimeDelta::zero()),
    )
    .into_iter()
    .filter(|msg| !matches!(msg, Msg::Clicked(..)))
    .collect()
}

/// Sends each mouse event through the update, as the shell does.
fn send(app: App, events: &[MouseEvent]) -> App {
    events.iter().fold(app, |app, event| {
        update(
            app,
            Msg::Mouse(Pointer::from_crossterm(*event, chrono::TimeDelta::zero())),
        )
        .0
    })
}

/// The state a press left, without the note of where it was.
fn settled(app: App) -> App {
    App {
        last_click: None,
        ..app
    }
}

fn key(c: char) -> Msg {
    Msg::Key(Key::Char(c))
}

/// The key messages among `msgs`, leaving out the drag bookkeeping.
fn keys_of(msgs: Vec<Msg>) -> Vec<Msg> {
    msgs.into_iter()
        .filter(|msg| matches!(msg, Msg::Key(_)))
        .collect()
}

/// The cell where `needle` starts on screen.
fn find(app: &App, needle: &str) -> (u16, u16) {
    let buf = render(app, 120, 36);
    let y = y_of(&buf, needle);
    let cells: Vec<&str> = (0..120).map(|x| buf[(x, y)].symbol()).collect();
    let x = (0..cells.len())
        .find(|x| cells[*x..].concat().starts_with(needle))
        .unwrap();
    (u16::try_from(x).unwrap(), y)
}

/// The team frame on Monday at 120 columns; 5-6pm is cells 68 to 71, columns 92 to 95.
fn team() -> App {
    at_120(team_monday())
}

const TRACK: u16 = 13;
const WHEEL: [MouseEventKind; 4] = [
    MouseEventKind::ScrollDown,
    MouseEventKind::ScrollUp,
    MouseEventKind::ScrollRight,
    MouseEventKind::ScrollLeft,
];

fn h_l() {
    let lane = (60, y_of(&render(&team(), 120, 36), "⌂ Paul"));
    let wheel = |kind| msgs(&team(), event(kind, lane, KeyModifiers::NONE));
    assert_eq!(wheel(WHEEL[0]), [key('l')]);
    assert_eq!(wheel(WHEEL[1]), [key('h')]);
    let held = send(team(), &[left(MouseEventKind::Down, (93, TRACK))]);
    let moved = msgs(&held, left(MouseEventKind::Drag, (94, TRACK)));
    assert_eq!(keys_of(moved), [key('l')]);
    let dragged = send(
        held,
        &[
            left(MouseEventKind::Drag, (94, TRACK)),
            left(MouseEventKind::Up, (94, TRACK)),
        ],
    );
    assert_eq!(dragged.say_line(), press(team(), &chars("l")).say_line());
}

fn shift_h_l() {
    let shifted = |kind| msgs(&team(), event(kind, (60, 20), KeyModifiers::SHIFT));
    assert_eq!(shifted(WHEEL[0]), [key('L')]);
    assert_eq!(shifted(WHEEL[1]), [key('H')]);
}

fn brackets() {
    let scroll = |kind| msgs(&team(), event(kind, (60, 20), KeyModifiers::NONE));
    assert_eq!(scroll(WHEEL[2]), [key(']')]);
    assert_eq!(scroll(WHEEL[3]), [key('[')]);
}

fn dot() {
    let back = find(&team(), "◂");
    assert_eq!(msgs(&team(), left(MouseEventKind::Down, back)), [key('.')]);
    let now = find(&team(), "Sat 24 Oct 9:04am");
    assert_eq!(msgs(&team(), left(MouseEventKind::Down, now)), [key('.')]);
}

fn v_o() {
    // 2pm is cell 56, column 80; dragging along a lane or the ruler to column 83 draws 2-3pm.
    let lane = y_of(&render(&team(), 120, 36), "⌂ Paul");
    let ruler = y_of(&render(&team(), 120, 36), "Mon 26 Oct AEDT");
    for y in [lane, ruler] {
        let held = send(team(), &[left(MouseEventKind::Down, (80, y))]);
        let started = msgs(&held, left(MouseEventKind::Drag, (83, y)));
        assert_eq!(keys_of(started)[0], key('v'));
        let drawn = send(
            held,
            &[
                left(MouseEventKind::Drag, (83, y)),
                left(MouseEventKind::Up, (83, y)),
            ],
        );
        assert_eq!(drawn.say_line(), "2-3pm AEDT / 1-2pm AEST / 5-6am SAST");
    }
}

fn edges() {
    let start = send(team(), &[left(MouseEventKind::Down, (92, TRACK))]);
    assert_eq!(
        keys_of(msgs(&start, left(MouseEventKind::Drag, (91, TRACK)))),
        [key('{')]
    );
    let end = send(team(), &[left(MouseEventKind::Down, (95, TRACK))]);
    assert_eq!(
        keys_of(msgs(&end, left(MouseEventKind::Drag, (96, TRACK)))),
        [key('>')]
    );
}

fn n_n() {
    // On regions the 8-9:30am run starts at cell 32, column 56; the 2h range shortens to it.
    let regions = at_120(go_to(press(fixture(), &chars("3")), "26 oct 2-4pm"));
    let at = (57, TRACK);
    let clicked = send(
        regions,
        &[left(MouseEventKind::Down, at), left(MouseEventKind::Up, at)],
    );
    assert_eq!(clicked.say_line(), "8-9:30am AEDT, open in AU, UK and US");
}

fn esc() {
    let corner = left(MouseEventKind::Down, (0, 0));
    let dialog = press(team(), &chars("c"));
    assert_eq!(msgs(&dialog, corner), [Msg::Key(Key::Esc)]);
    let help = press(team(), &chars("?"));
    assert_eq!(msgs(&help, corner), [Msg::Key(Key::Esc)]);
    let map = press(update(team_monday(), Msg::Resize(120, 24)).0, &chars("m"));
    assert_eq!(msgs(&map, corner), [Msg::Key(Key::Esc)]);
}

fn focus_rows() {
    let sam = find(&team(), "Sam");
    let clicked = msgs(&team(), left(MouseEventKind::Down, sam));
    let focused = focus_on(team(), "Sam");
    assert_eq!(clicked, [Msg::Focus(focused.focus)]);
    assert_eq!(
        settled(send(team(), &[left(MouseEventKind::Down, sam)])),
        focused
    );
}

fn marks() {
    let (_, y) = find(&team(), "Sam");
    let clicked = msgs(&team(), left(MouseEventKind::Down, (0, y)));
    let sam = focus_on(team(), "Sam").focus;
    assert_eq!(clicked, [Msg::Focus(sam), key(' ')]);
    let toggled = settled(send(team(), &[left(MouseEventKind::Down, (0, y))]));
    assert_eq!(toggled, press(focus_on(team(), "Sam"), &chars(" ")));
    assert!(matches!(sam, Row::Person(_)));
}

fn more() {
    let folded = update(team_monday(), Msg::Resize(80, 20)).0;
    let buf = render(&folded, 80, 20);
    let at = (2, y_of(&buf, "○ +5 more"));
    let fold_row = press(folded.clone(), &chars("jj")).focus;
    assert_eq!(
        msgs(&folded, left(MouseEventKind::Down, at)),
        [Msg::Focus(fold_row), Msg::Key(Key::Enter)]
    );
    let clicked = settled(send(folded.clone(), &[left(MouseEventKind::Down, at)]));
    assert_eq!(
        clicked,
        press(folded, &[Key::Char('j'), Key::Char('j'), Key::Enter])
    );
}

fn groups() {
    let regions = find(&team(), "regions");
    assert_eq!(
        msgs(&team(), left(MouseEventKind::Down, regions)),
        [key('3')]
    );
    let all = find(&team(), "all");
    assert_eq!(msgs(&team(), left(MouseEventKind::Down, all)), [key('0')]);
}

#[test]
fn the_gap_after_the_strip_is_inert() {
    let (x, y) = find(&team(), "⋯");
    let gap = (x + 2, y);
    assert_eq!(keys_of(msgs(&team(), left(MouseEventKind::Down, gap))), []);
}

#[test]
fn the_wheel_scrubs_under_the_map_overlay_and_a_click_inside_it_does_nothing() {
    let open = press(update(team_monday(), Msg::Resize(120, 24)).0, &chars("m"));
    let inside = (60, 12);
    assert_eq!(
        msgs(
            &open,
            event(MouseEventKind::ScrollDown, inside, KeyModifiers::NONE)
        ),
        [key('l')]
    );
    assert_eq!(keys_of(msgs(&open, left(MouseEventKind::Down, inside))), []);
}

#[test]
fn a_failed_capture_switch_keeps_the_footer_true_to_the_terminal() {
    let mut broken = Recorder {
        broken: true,
        ..Recorder::default()
    };
    let failed = press_running(fixture(), &chars("M"), &mut broken);
    assert!(failed.mouse, "the terminal still captures the mouse");
    assert!(
        !footer(&failed).contains("mouse off"),
        "{}",
        footer(&failed)
    );
}

fn undo() {
    let dropped = press(focus_on(team(), "UK"), &chars(" 3"));
    let at = find(&dropped, "u undo");
    assert_eq!(msgs(&dropped, left(MouseEventKind::Down, at)), [key('u')]);
}

fn copy() {
    let at = find(&team(), "y copy");
    assert_eq!(msgs(&team(), left(MouseEventKind::Down, at)), [key('y')]);
}

fn copy_as() {
    let dialog = press(team(), &chars("c"));
    let at = find(&dialog, "Mon 26 Oct, 5-6pm");
    assert_eq!(msgs(&dialog, left(MouseEventKind::Down, at)), [key('d')]);
}

fn lanes() {
    let at = find(&team(), "morning/evening");
    assert_eq!(msgs(&team(), left(MouseEventKind::Down, at)), [key('b')]);
}

fn sort() {
    let who = find(&team(), " who ");
    assert_eq!(msgs(&team(), left(MouseEventKind::Down, who)), [key('s')]);
    let place = find(&team(), " where ");
    assert_eq!(msgs(&team(), left(MouseEventKind::Down, place)), [key('s')]);
}

fn down_at(at: (u16, u16)) -> MouseEvent {
    left(MouseEventKind::Down, at)
}

fn w_plus_x_equals() {
    let drifted = press(focus_on(team(), "UK"), &chars(" "));
    let tag = find(&drifted, "+@UK");
    let menu = send(drifted, &[down_at(tag)]);
    assert_eq!(menu.mode, crate::tui::app::Mode::DriftMenu);
    let save = find(&menu, "w save");
    assert_eq!(keys_of(msgs(&menu, down_at(save))), [key('w')]);
    let revert = find(&menu, "r revert");
    assert_eq!(keys_of(msgs(&menu, down_at(revert))), [key('r')]);
    assert!(send(menu, &[down_at(revert)]).drift().is_empty());
    let plus = find(&team(), "+  ⋯");
    assert_eq!(keys_of(msgs(&team(), down_at(plus))), [key('+')]);
    let regions = at_120(go_to(press(fixture(), &chars("3")), "26 oct 2-4pm"));
    let asks = find(&regions, "asks open");
    assert_eq!(keys_of(msgs(&regions, down_at(asks))), [key('=')]);
    let deploy = find(&team(), "deploy");
    let middle = event(
        MouseEventKind::Down(MouseButton::Middle),
        deploy,
        KeyModifiers::NONE,
    );
    assert_eq!(msgs(&team(), middle), [key('4'), key('x')]);
}

fn tab() {
    let dots = find(&team(), "⋯");
    assert_eq!(keys_of(msgs(&team(), down_at(dots))), [Msg::Key(Key::Tab)]);
}

fn a_e_d() {
    let map = (30, 6);
    let once = send(team(), &[down_at(map)]);
    let twice = msgs(&once, down_at(map));
    let added = twice
        .iter()
        .find_map(|msg| match msg {
            Msg::AddAt(lat, lon) => Some((*lat, *lon)),
            _ => None,
        })
        .expect("a double click on the map adds at the point");
    let area = ratatui::layout::Rect::new(0, 1, 120, 11);
    let projection = crate::tui::map::Projection::new(area, once.camera);
    assert_eq!(Some(added), projection.unproject(map.0, map.1));
    let opened = send(once, &[down_at(map)]);
    assert_eq!(opened.mode, crate::tui::app::Mode::AddMenu(Some(added)));
    let sam = find(&team(), "Sam");
    let once = send(team(), &[down_at(sam)]);
    let focused = focus_on(team(), "Sam").focus;
    assert_eq!(
        keys_of(msgs(&once, down_at(sam))),
        [key('e')],
        "a double click on a label edits its row"
    );
    assert!(msgs(&once, down_at(sam)).contains(&Msg::Focus(focused)));
}

/// Each keymap row with a mouse equivalent, by its keys cell, and the check that the mouse
/// sends what the row's key does.
const CASES: &[(&str, fn())] = &[
    ("h l, ← →", h_l),
    ("H L, shift-← →", shift_h_l),
    ("[ ]", brackets),
    (".", dot),
    ("v, o", v_o),
    ("{ } and < >", edges),
    ("n N", n_n),
    ("esc", esc),
    ("j k, ↑ ↓, g G", focus_rows),
    ("space, X", marks),
    ("enter", more),
    ("1-9, 0", groups),
    ("w, +, x, =", w_plus_x_equals),
    ("u", undo),
    ("tab", tab),
    ("y", copy),
    ("c", copy_as),
    ("b", lanes),
    ("s", sort),
    ("z", map_mode_wheel),
    ("a e d", a_e_d),
];

fn map_mode_wheel() {
    let mapping = press(team(), &chars("z"));
    let (over_map, over_grid) = ((30, 6), (60, 20));
    let wheel = |kind, at, modifiers| keys_of(msgs(&mapping, event(kind, at, modifiers)));
    let none = KeyModifiers::NONE;
    assert_eq!(wheel(MouseEventKind::ScrollUp, over_map, none), [key('k')]);
    assert_eq!(
        wheel(MouseEventKind::ScrollDown, over_map, none),
        [key('j')]
    );
    assert_eq!(
        wheel(MouseEventKind::ScrollLeft, over_map, none),
        [key('h')]
    );
    assert_eq!(
        wheel(MouseEventKind::ScrollRight, over_map, none),
        [key('l')]
    );
    let shift = KeyModifiers::SHIFT;
    assert_eq!(wheel(MouseEventKind::ScrollUp, over_map, shift), [key('h')]);
    assert_eq!(
        wheel(MouseEventKind::ScrollDown, over_map, shift),
        [key('l')]
    );
    assert!(wheel(MouseEventKind::ScrollDown, over_grid, none).is_empty());
    let scrolled = send(
        mapping.clone(),
        &[event(MouseEventKind::ScrollDown, over_map, none)],
    );
    assert_eq!(scrolled, press(mapping, &chars("j")));
    assert!(
        keys_of(msgs(
            &team(),
            event(MouseEventKind::ScrollDown, over_map, none)
        ))
        .is_empty(),
        "outside map mode the wheel over the map does what it did"
    );
}

const KEYS_SPEC: &str = include_str!("../../../../../docs/spec/keys.md");

/// The keymap rows whose milestone is built and whose mouse column names an equivalent.
fn built_mouse_rows() -> Vec<String> {
    KEYS_SPEC
        .lines()
        .skip_while(|line| !line.starts_with("| Key |"))
        .take_while(|line| line.starts_with('|'))
        .skip(2)
        .filter_map(|line| {
            let cells: Vec<&str> = line.trim_matches('|').split(" | ").map(str::trim).collect();
            let built = BUILT.iter().any(|m| cells[3].contains(m));
            (built && cells[2] != "none").then(|| cells[0].to_owned())
        })
        .collect()
}

#[test]
fn every_built_mouse_equivalent_in_the_keymap_sends_what_its_key_does() {
    let covered: Vec<&str> = CASES.iter().map(|(keys, _)| *keys).collect();
    assert_eq!(covered, built_mouse_rows());
    for (_, check) in CASES {
        check();
    }
}

/// The fixture app reading its config from a file holding the fixture, so the matrix can edit it.
fn editable() -> App {
    at_120(App::new(Config::parse(FIXTURE).unwrap(), now()).with_source(Some(FIXTURE.to_owned())))
}

/// The matrix open on team at 120 by 36.
fn matrix_open() -> App {
    press(editable(), &[Key::Tab])
}

fn group_names(app: &App) -> Vec<String> {
    app.roster().groups.iter().map(|g| g.name.clone()).collect()
}

fn in_matrix(app: &App) -> bool {
    crate::tui::matrix::of(app).is_some()
}

fn matrix_click_a_cell() {
    let open = matrix_open();
    let deploy = find(&open, "4 deploy");
    let (_, sam) = find(&open, "Sam ");
    let cell = (deploy.0 + 2, sam);
    let clicked = send(open.clone(), &[down_at(cell)]);
    let keyed = press(
        open,
        &[
            Key::Down,
            Key::Down,
            Key::Char('l'),
            Key::Char('l'),
            Key::Char('l'),
            Key::Char(' '),
        ],
    );
    assert_eq!(settled(clicked).config, keyed.config);
    let sam = keyed.roster().person_named("Sam").unwrap();
    assert!(keyed.roster().groups[3].people.contains(&sam));
}

fn matrix_click_a_name() {
    let open = matrix_open();
    let regions = find(&open, "3 regions");
    let used = send(
        open.clone(),
        &[down_at(regions), left(MouseEventKind::Up, regions)],
    );
    let keyed = press(open, &[Key::Char('l'), Key::Char('l'), Key::Enter]);
    assert!(!in_matrix(&used));
    assert_eq!(used.lens_name(), "regions");
    assert_eq!(settled(used).lens, keyed.lens);
}

fn matrix_drag_a_name() {
    let open = matrix_open();
    let deploy = find(&open, "4 deploy");
    let regions = find(&open, "3 regions");
    let team = find(&open, "1 team");
    let held = send(open, &[down_at(deploy)]);
    let over = |at: (u16, u16)| left(MouseEventKind::Drag, (at.0 + 1, at.1));
    let dragged = send(held, &[over(regions), over(regions), over(regions)]);
    assert_eq!(
        group_names(&dragged),
        ["team", "au", "deploy", "regions"],
        "every drag event inside one column moves the group only once"
    );
    let further = send(dragged, &[over(team), over(team)]);
    assert_eq!(group_names(&further), ["deploy", "team", "au", "regions"]);
    let released = send(further, &[left(MouseEventKind::Up, team)]);
    assert!(
        in_matrix(&released),
        "letting go after a drag does not use the group"
    );
}

fn matrix_right_click() {
    let open = matrix_open();
    let deploy = find(&open, "4 deploy");
    let right = event(
        MouseEventKind::Down(MouseButton::Right),
        deploy,
        KeyModifiers::NONE,
    );
    let renaming = send(open.clone(), &[right]);
    let keyed = press(
        open,
        &[
            Key::Char('l'),
            Key::Char('l'),
            Key::Char('l'),
            Key::Char('R'),
        ],
    );
    assert_eq!(
        crate::tui::matrix::of(&renaming).map(|m| m.naming.clone()),
        crate::tui::matrix::of(&keyed).map(|m| m.naming.clone())
    );
}

fn matrix_middle_click() {
    let open = matrix_open();
    let au = find(&open, "2 au");
    let middle = event(
        MouseEventKind::Down(MouseButton::Middle),
        au,
        KeyModifiers::NONE,
    );
    let closed = send(open, &[middle]);
    assert_eq!(group_names(&closed), ["team", "regions", "deploy"]);
}

/// Each mouse equivalent the matrix's keymap paragraph lists, and the check that it sends what
/// its key does.
const MATRIX_CASES: &[(&str, fn())] = &[
    (
        "click a cell to move there and toggle it",
        matrix_click_a_cell,
    ),
    ("click a column name to use that group", matrix_click_a_name),
    ("drag a column name to reorder", matrix_drag_a_name),
    ("right-click to rename", matrix_right_click),
    ("middle-click to close", matrix_middle_click),
];

/// The clauses of the keymap's `Mouse in the matrix:` sentence.
fn matrix_mouse_clauses() -> Vec<String> {
    let (_, after) = KEYS_SPEC
        .split_once("Mouse in the matrix: ")
        .expect("keys.md lists the matrix's mouse equivalents");
    let sentence = after.lines().next().unwrap_or_default();
    sentence
        .trim_end_matches('.')
        .split(", ")
        .map(str::to_owned)
        .collect()
}

#[test]
fn every_matrix_mouse_equivalent_in_the_keymap_sends_what_its_key_does() {
    let covered: Vec<&str> = MATRIX_CASES.iter().map(|(clause, _)| *clause).collect();
    assert_eq!(covered, matrix_mouse_clauses());
    for (_, check) in MATRIX_CASES {
        check();
    }
}

#[test]
fn a_click_outside_the_matrix_closes_it() {
    let closed = send(matrix_open(), &[down_at((0, 0))]);
    assert!(!in_matrix(&closed));
}

#[test]
fn two_presses_on_a_label_far_apart_on_the_loop_s_clock_are_not_a_double_click() {
    let sam = find(&editable(), "Sam");
    let press_at = |app: App, ms: i64| {
        let pointer = Pointer::from_crossterm(
            left(MouseEventKind::Down, sam),
            chrono::TimeDelta::milliseconds(ms),
        );
        update(app, Msg::Mouse(pointer)).0
    };
    let late = press_at(press_at(editable(), 0), 1000);
    assert_eq!(
        late.mode,
        crate::tui::app::Mode::Normal,
        "the fixed clock never advances"
    );
    let quick = press_at(press_at(editable(), 0), 300);
    assert!(
        matches!(quick.mode, crate::tui::app::Mode::Picker(_)),
        "{:?}",
        quick.mode
    );
}

#[test]
fn with_the_map_off_a_click_on_a_label_lands_on_the_row_drawn_there() {
    let hidden = press(team(), &chars("m"));
    let sam = find(&hidden, "Sam");
    assert!(sam.1 < 13, "the blocks moved up under the header: {sam:?}");
    let focused = focus_on(hidden.clone(), "Sam");
    assert_eq!(
        msgs(&hidden, left(MouseEventKind::Down, sam)),
        [Msg::Focus(focused.focus)]
    );
}

#[test]
fn a_click_on_the_fold_row_while_drawing_a_range_only_focuses_it() {
    let folded = update(team_monday(), Msg::Resize(80, 20)).0;
    let at = (2, y_of(&render(&folded, 80, 20), "○ +5 more"));
    let fold_row = press(folded.clone(), &chars("jj")).focus;
    let drawing = press(folded, &chars("v"));
    assert_eq!(
        msgs(&drawing, left(MouseEventKind::Down, at)),
        [Msg::Focus(fold_row)]
    );
    let clicked = send(drawing, &[left(MouseEventKind::Down, at)]);
    assert!(clicked.visual.is_some(), "the range is still being drawn");
}

#[test]
fn on_the_too_small_screen_the_mouse_does_nothing() {
    let small = update(team_monday(), Msg::Resize(30, 8)).0;
    let everywhere = (0..30).flat_map(|x| (0..8).map(move |y| (x, y)));
    for at in everywhere {
        assert_eq!(msgs(&small, left(MouseEventKind::Down, at)), [], "{at:?}");
    }
}
