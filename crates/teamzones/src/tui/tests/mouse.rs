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
    mouse::msgs(app, Pointer::from_crossterm(event))
        .into_iter()
        .filter(|msg| !matches!(msg, Msg::Clicked(..)))
        .collect()
}

/// Sends each mouse event through the update, as the shell does.
fn send(app: App, events: &[MouseEvent]) -> App {
    events.iter().fold(app, |app, event| {
        update(app, Msg::Mouse(Pointer::from_crossterm(*event))).0
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

const TRACK: u16 = 14;
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
    let area = ratatui::layout::Rect::new(0, 1, 120, 12);
    assert_eq!(added, crate::tui::map::unproject(area, map.0, map.1));
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
    ("1-9, 0", groups),
    ("w, +, x, =", w_plus_x_equals),
    ("u", undo),
    ("tab", tab),
    ("y", copy),
    ("c", copy_as),
    ("b", lanes),
    ("s", sort),
    ("a e d", a_e_d),
];

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
