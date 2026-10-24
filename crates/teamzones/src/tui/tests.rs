//! Frame tests: the fixture rendered through `TestBackend`, driven by key presses.

use chrono::{DateTime, Utc};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use teamzones_core::config::Config;

use super::app::App;
use super::glyphs::Glyphs;
use super::keys::Key;
use super::theme::{TerminalEnv, Theme};
use super::update::{Effect, Msg, update};
use super::view::{self, Look};

const FIXTURE: &str = include_str!("../../../teamzones-core/tests/fixtures/sample.toml");

/// Sat 24 Oct 2026, 9:04am AEDT.
fn now() -> DateTime<Utc> {
    "2026-10-23T22:04:00Z".parse().unwrap()
}

fn config(extra: &str) -> Config {
    Config::parse(&format!("{FIXTURE}{extra}")).unwrap()
}

fn fixture() -> App {
    App::new(config(""), now())
}

/// Each character as its key, so `"3"` is the key `3`.
fn chars(text: &str) -> Vec<Key> {
    text.chars().map(Key::Char).collect()
}

/// Presses each key in turn, returning the last state and every effect asked for.
fn press_all(app: App, keys: &[Key]) -> (App, Vec<Effect>) {
    keys.iter()
        .fold((app, Vec::new()), |(app, mut effects), key| {
            let (next, more) = update(app, Msg::Key(*key));
            effects.extend(more);
            (next, effects)
        })
}

fn press(app: App, keys: &[Key]) -> App {
    press_all(app, keys).0
}

/// Types `text` at the `t` prompt and submits it.
fn go_to(app: App, text: &str) -> App {
    let keys: Vec<Key> = std::iter::once(Key::Char('t'))
        .chain(chars(text))
        .chain(std::iter::once(Key::Enter))
        .collect();
    press(app, &keys)
}

/// The team frame the spec's mockup shows: Mon 26 Oct 5-6pm.
fn team_monday() -> App {
    go_to(fixture(), "26 oct 5-6pm")
}

fn render_with(app: &App, cols: u16, rows: u16, env: TerminalEnv) -> Buffer {
    let theme = Theme::resolve(&app.config.theme, env);
    let look = Look {
        theme: &theme,
        glyphs: Glyphs::of(app.config.theme.glyphs),
    };
    let mut terminal = Terminal::new(TestBackend::new(cols, rows)).unwrap();
    terminal.draw(|frame| view::draw(frame, app, look)).unwrap();
    terminal.backend().buffer().clone()
}

fn render(app: &App, cols: u16, rows: u16) -> Buffer {
    render_with(app, cols, rows, TerminalEnv::default())
}

fn row(buf: &Buffer, y: u16) -> String {
    (0..buf.area.width)
        .map(|x| buf[(x, y)].symbol())
        .collect::<String>()
        .trim_end()
        .to_owned()
}

fn text(buf: &Buffer) -> String {
    (0..buf.area.height)
        .map(|y| row(buf, y))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The first row whose text contains `needle`.
fn y_of(buf: &Buffer, needle: &str) -> u16 {
    (0..buf.area.height)
        .find(|y| row(buf, *y).contains(needle))
        .unwrap_or_else(|| panic!("no row holds {needle:?} in\n{}", text(buf)))
}

/// The say line: the say panel's first inner row, up to the summary.
fn say_line(buf: &Buffer) -> String {
    let y = y_of(buf, " say ") + 1;
    row(buf, y)
        .trim_start_matches(['│', '|', ' '])
        .split("   ")
        .next()
        .unwrap()
        .to_owned()
}

#[test]
fn the_team_frame_at_120_by_36_sits_on_the_row_budget() {
    let buf = render(&team_monday(), 120, 36);
    assert!(
        row(&buf, 0).starts_with("teamzones   now Sat 24 Oct 9:04am AEDT   viewing Mon 26 Oct +2d")
    );
    assert!(
        (1..13).all(|y| row(&buf, y).is_empty()),
        "the map area is blank until the map lands"
    );
    assert!(row(&buf, 13).starts_with("Mon 26 Oct AEDT"));
    assert!(row(&buf, 14).starts_with("▸ 5-6pm AEDT 1h"));
    assert!(row(&buf, 15).starts_with(" who "));
    assert!(row(&buf, 16).starts_with("⌂ Paul       5-6pm AEDT"));
    assert!(row(&buf, 21).starts_with("≡ in hours          5/5"));
    assert!(row(&buf, 24).starts_with(" where  not asked"));
    assert!(row(&buf, 25).starts_with("≡ where              --  not asked: team has no places"));
    assert!(row(&buf, 30).starts_with("╭ say team "));
    assert_eq!(say_line(&buf), "5-6pm AEDT / 4-5pm AEST / 8-9am SAST");
    assert_eq!(row(&buf, 34), "");
    assert!(row(&buf, 35).starts_with("h/l time"));
    insta::assert_snapshot!(text(&buf));
}

/// The rows that must hold still: the ruler, both blocks' top borders, the where block's bottom
/// border and the say panel's top border.
fn anchors(buf: &Buffer) -> [u16; 5] {
    let say = y_of(buf, " say ");
    [
        y_of(buf, " AEDT    "),
        y_of(buf, " who "),
        y_of(buf, " where "),
        say - 1,
        say,
    ]
}

#[test]
fn switching_groups_never_moves_the_ruler_the_borders_or_the_say_panel() {
    let team = team_monday();
    let regions = press(team.clone(), &chars("3"));
    let deploy = press(regions.clone(), &chars("4"));
    let [team, regions, deploy] = [&team, &regions, &deploy].map(|app| render(app, 120, 36));
    assert_eq!(anchors(&team), anchors(&regions));
    assert_eq!(anchors(&regions), anchors(&deploy));
    assert_eq!(
        say_line(&regions),
        "5-6pm AEDT, open in UK and US, busy in AU"
    );
    assert_eq!(
        say_line(&deploy),
        "5-6pm AEDT / 4-5pm AEST / 8-9am SAST, open in UK, busy in AU"
    );
}

#[test]
fn n_from_saturday_on_deploy_lands_on_monday_and_shortens_the_range() {
    let app = press(fixture(), &chars("4n"));
    let buf = render(&app, 120, 36);
    assert_eq!(
        row(&buf, 34),
        "n  next fit for deploy: Mon 5-5:30pm AEDT, range shortened from 1h to 30m (AU busy from 5:30pm)"
    );
    let axis = app.roster().axis();
    let fits: Vec<String> = teamzones_core::fit::fits(app.roster(), &app.marks, app.day())
        .iter()
        .map(|range| {
            teamzones_core::copy::span_text(
                &range.start().start().with_timezone(&axis),
                &range.end().start().with_timezone(&axis),
            )
        })
        .collect();
    assert_eq!(fits, ["5-5:30pm"]);
    // 5pm is cell 68 of the 96, after the 24 label columns.
    let track: Vec<String> = (92..94).map(|x| buf[(x, 14)].symbol().to_owned()).collect();
    assert_eq!(track, ["┣", "┫"]);
    assert!(row(&buf, 14).starts_with("▸ 5-5:30pm AEDT 30m"));
}

#[test]
fn repeated_n_steps_run_by_run() {
    let app = press(go_to(press(fixture(), &chars("3")), "mon 2pm"), &chars("n"));
    assert_eq!(app.say_line(), "9-10pm AEDT, open in AU, UK and US");
}

#[test]
fn shift_n_lands_on_the_start_of_the_previous_run() {
    let app = press(go_to(press(fixture(), &chars("3")), "mon 9pm"), &chars("N"));
    assert_eq!(app.say_line(), "2-3pm AEDT, open in AU, UK and US");
}

#[test]
fn n_with_nothing_in_seven_days_says_so_and_moves_to_the_best_slot() {
    let quiet = App::new(
        Config::parse(&FIXTURE.replace(
            "places  = [\"AU\", \"UK\", \"US\"]",
            "places  = [\"AU\", \"UK\", \"US\"]\nwhere   = \"quiet\"",
        ))
        .unwrap(),
        now(),
    );
    let app = press(quiet, &chars("3n"));
    assert_eq!(
        app.notice.as_ref().unwrap().text,
        "n  no full slot in 7 days, best 2/3 at Sat 3pm AEDT"
    );
    assert_eq!(app.say_line(), "3pm AEDT, closed in UK and US, open in AU");
}

/// Moves focus down until it sits on the row named `name`.
fn focus_on(app: App, name: &str) -> App {
    let mut app = press(app, &chars("g"));
    for _ in 0..20 {
        if focus_name(&app) == name {
            return app;
        }
        app = press(app, &chars("j"));
    }
    panic!("no row named {name}")
}

const COPY_SPEC: &str = include_str!("../../../../docs/spec/copy.md");

/// The copy golden table's rows: setup, keys and clipboard, with backticks kept.
fn copy_rows() -> Vec<[String; 3]> {
    COPY_SPEC
        .split_once("## Copy golden table")
        .unwrap()
        .1
        .lines()
        .skip_while(|line| !line.starts_with('|'))
        .take_while(|line| line.starts_with('|'))
        .skip(2)
        .map(|line| {
            let cells: Vec<String> = line
                .trim_matches('|')
                .split(" | ")
                .map(|cell| cell.trim().to_owned())
                .collect();
            [cells[0].clone(), cells[1].clone(), cells[2].clone()]
        })
        .collect()
}

fn unticked(cell: &str) -> &str {
    cell.trim_matches('`')
}

/// Builds the copy golden row's state through keys: its group's number, `t` and its range,
/// then any `space` or focus.
fn copy_setup(setup: &str) -> App {
    let mut parts = setup.split(", ");
    let group = parts.next().unwrap();
    let base = fixture();
    let index = base
        .roster()
        .groups
        .iter()
        .position(|g| g.name == group)
        .unwrap();
    let digit = char::from_digit(u32::try_from(index + 1).unwrap(), 10).unwrap();
    parts.fold(press(base, &[Key::Char(digit)]), |app, part| {
        if let Some(name) = part.strip_prefix("`space` on ") {
            press(focus_on(app, name), &chars(" "))
        } else if let Some(name) = part.strip_prefix("focus ") {
            focus_on(app, name)
        } else {
            let words: Vec<&str> = part.split_whitespace().collect();
            let dated = words
                .get(1)
                .is_some_and(|w| w.bytes().all(|b| b.is_ascii_digit()));
            go_to(app, &words[usize::from(dated)..].join(" "))
        }
    })
}

fn copied(effects: &[Effect]) -> Vec<String> {
    effects
        .iter()
        .map(|effect| match effect {
            Effect::Copy { text, .. } => text.clone(),
        })
        .collect()
}

#[test]
fn every_copy_golden_row_comes_from_its_keys() {
    let rows = copy_rows();
    assert_eq!(rows.len(), 9, "the copy golden table has nine rows");
    for [setup, keys, clipboard] in rows {
        let keys: Vec<Key> = unticked(&keys)
            .split(' ')
            .map(|k| Key::Char(k.chars().next().unwrap()))
            .collect();
        let (app, effects) = press_all(copy_setup(&setup), &keys);
        assert_eq!(
            copied(&effects),
            [unticked(&clipboard)],
            "{setup}, {keys:?}"
        );
        if keys == [Key::Char('y')] {
            assert_eq!(
                say_line(&render(&app, 120, 36)),
                unticked(&clipboard),
                "{setup} on screen"
            );
        }
    }
}

#[test]
fn c_a_copies_the_line_for_all() {
    let app = team_monday();
    assert_eq!(
        copied(&press_all(app, &chars("ca")).1),
        [
            "5-6pm AEDT / 4-5pm AEST / 8-9am SAST / 6-7am GMT / 1-2am CDT, open in UK and US, busy in AU"
        ]
    );
}

#[test]
fn a_copy_raises_copied_or_shows_the_line_when_no_clipboard_works() {
    let worked = update(
        fixture(),
        Msg::Copied {
            key: "y".to_owned(),
            text: "3pm AEDT".to_owned(),
            result: Ok(()),
        },
    )
    .0;
    let failed = update(
        fixture(),
        Msg::Copied {
            key: "y".to_owned(),
            text: "3pm AEDT".to_owned(),
            result: Err("no display".to_owned()),
        },
    )
    .0;
    assert_eq!(worked.notice.unwrap().text, "y  copied");
    assert_eq!(
        failed.notice.unwrap().text,
        "y  clipboard unavailable: 3pm AEDT"
    );
}

#[test]
fn space_on_uk_in_team_drifts_and_a_switch_drops_it_until_u() {
    let marked = press(focus_on(team_monday(), "UK"), &chars(" "));
    let buf = render(&marked, 120, 36);
    assert!(row(&buf, y_of(&buf, " say ")).contains("say team +@UK"));
    assert!(row(&buf, y_of(&buf, " where ")).contains("asks open"));
    let golden = copy_rows()
        .into_iter()
        .find(|[setup, _, _]| setup.contains("`space` on UK"))
        .unwrap();
    assert_eq!(say_line(&buf), unticked(&golden[2]));

    let dropped = press(marked, &chars("3"));
    assert_eq!(
        dropped.notice.as_ref().unwrap().text,
        "3  dropped team +@UK  u undo"
    );
    assert_eq!(dropped.expression(), "regions");

    let restored = press(dropped, &chars("u"));
    assert_eq!(restored.expression(), "team +@UK");
    assert_eq!(say_line(&render(&restored, 120, 36)), unticked(&golden[2]));
}

#[test]
fn undo_expires_ten_seconds_after_the_drop() {
    let dropped = press(
        press(focus_on(team_monday(), "UK"), &chars(" ")),
        &chars("3"),
    );
    let later = update(dropped, Msg::Tick(now() + chrono::TimeDelta::seconds(11))).0;
    assert_eq!(press(later, &chars("u")).expression(), "regions");
}

#[test]
fn the_notice_clears_on_the_next_key_or_after_ten_seconds() {
    let dropped = press(
        press(focus_on(team_monday(), "UK"), &chars(" ")),
        &chars("3"),
    );
    assert!(press(dropped.clone(), &chars("j")).notice.is_none());
    let later = update(dropped, Msg::Tick(now() + chrono::TimeDelta::seconds(10))).0;
    assert!(later.notice.is_none());
}

#[test]
fn pressing_the_active_group_again_reverts_its_drift() {
    let marked = press(focus_on(team_monday(), "UK"), &chars(" "));
    assert_eq!(press(marked, &chars("1")).expression(), "team");
}

#[test]
fn x_clears_the_marks_to_home_only_which_asks_who() {
    let app = press(team_monday(), &chars("X"));
    assert_eq!(app.say_line(), "5-6pm AEDT");
    assert_eq!(app.expression(), "team -Priya -Sam -Thandi -Johan");
}

#[test]
fn a_move_from_now_snaps_to_the_grid() {
    assert_eq!(
        press(fixture(), &chars("l")).say_line(),
        "9:15am AEDT / 8:15am AEST / 12:15am SAST"
    );
    assert_eq!(
        press(fixture(), &chars("h")).say_line(),
        "9am AEDT / 8am AEST / 12am SAST"
    );
    assert_eq!(
        press(fixture(), &chars("L")).say_line(),
        "10am AEDT / 9am AEST / 1am SAST"
    );
}

#[test]
fn dot_goes_back_to_now_and_clears_the_range() {
    let app = press(team_monday(), &chars("."));
    assert_eq!(
        (app.range, app.say_line()),
        (None, "9:04am AEDT / 8:04am AEST / 12:04am SAST".to_owned())
    );
}

#[test]
fn visual_mode_draws_a_range_and_esc_restores_the_one_before() {
    let drawn = press(fixture(), &chars("lvll"));
    assert_eq!(
        drawn.say_line(),
        "9:15-10am AEDT / 8:15-9am AEST / 12:15-1am SAST"
    );
    let committed = press(drawn.clone(), &chars("v"));
    assert_eq!(
        (committed.visual, committed.say_line()),
        (None, drawn.say_line())
    );
    let restored = press(drawn, &[Key::Esc]);
    assert_eq!(
        restored.say_line(),
        "9:15am AEDT / 8:15am AEST / 12:15am SAST"
    );
}

#[test]
fn the_range_edges_move_on_their_own_keys() {
    let app = press(team_monday(), &chars("}>>"));
    assert_eq!(
        app.say_line(),
        "5:15-6:30pm AEDT / 4:15-5:30pm AEST / 8:15-9:30am SAST"
    );
}

#[test]
fn esc_collapses_the_range_to_its_start() {
    let app = press(team_monday(), &[Key::Esc]);
    assert_eq!(app.say_line(), "5pm AEDT / 4pm AEST / 8am SAST");
}

#[test]
fn a_bad_typed_time_says_why_in_the_notice() {
    let app = go_to(fixture(), "9-5");
    assert_eq!(
        app.notice.unwrap().text,
        "t  9-5 needs am/pm or HH:MM, such as 9am-5pm or 09:00-17:00"
    );
}

#[test]
fn the_footer_at_80_columns_drops_copy_as_and_keeps_help_and_quit() {
    let buf = render(&fixture(), 80, 24);
    assert_eq!(
        row(&buf, 23),
        "h/l time  v range  n next fit  space mark  1-9 group  y copy  ? help  q quit"
    );
}

#[test]
fn the_help_overlay_lists_context_keys_then_global_keys_then_glyphs() {
    let buf = render(&press(fixture(), &chars("?")), 120, 36);
    let (here, everywhere, glyphs) = (
        y_of(&buf, "keys here"),
        y_of(&buf, "everywhere"),
        y_of(&buf, "glyphs"),
    );
    assert!(here < y_of(&buf, "Copy the say line") && y_of(&buf, "Copy the say line") < everywhere);
    assert!(everywhere < y_of(&buf, "Help; quit") && y_of(&buf, "Help; quit") < glyphs);
    let closed = press(press(fixture(), &chars("?")), &[Key::Esc]);
    assert_eq!(closed.mode, super::app::Mode::Normal);
}

#[test]
fn no_color_renders_mono() {
    use ratatui::style::Color;
    let env = TerminalEnv {
        no_color: true,
        truecolor: true,
    };
    for app in [team_monday(), press(team_monday(), &chars("4"))] {
        let buf = render_with(&app, 120, 36, env);
        let colours: Vec<(Color, Color)> = buf.content.iter().map(|c| (c.fg, c.bg)).collect();
        assert!(
            colours
                .iter()
                .all(|(fg, bg)| matches!(fg, Color::Reset | Color::DarkGray) && *bg == Color::Reset),
            "{colours:?}"
        );
    }
}

#[test]
fn ascii_glyphs_render_no_non_ascii_character_anywhere() {
    let ascii = App::new(config("glyphs = \"ascii\"\n"), now());
    let team = go_to(ascii, "26 oct 5-6pm");
    let frames = [
        render(&team, 120, 36),
        render(&press(team.clone(), &chars("4n")), 120, 36),
        render(&press(team.clone(), &chars("?")), 120, 36),
        render(&team, 80, 24),
    ];
    for buf in frames {
        assert!(text(&buf).is_ascii(), "{}", text(&buf));
    }
}

#[test]
fn the_frames_at_120_by_24_and_80_by_24_sit_on_the_row_budget() {
    let regions = press(fixture(), &chars("3"));
    for (cols, snapshot) in [(120, "regions_120x24"), (80, "regions_80x24")] {
        let buf = render(&regions, cols, 24);
        assert_eq!(
            (
                y_of(&buf, " who "),
                y_of(&buf, " where "),
                y_of(&buf, " say ")
            ),
            (3, 12, 18),
            "{cols} columns"
        );
        assert!(row(&buf, 1).starts_with("Sat 24 Oct AEDT"));
        insta::assert_snapshot!(snapshot, text(&buf));
    }
}

/// Every key the main footer or binding table names, as probes for the modal tests.
fn main_keys() -> Vec<Key> {
    super::keys::BINDINGS
        .iter()
        .filter(|b| {
            matches!(
                b.context,
                super::keys::Context::Main | super::keys::Context::Global
            )
        })
        .flat_map(|b| b.chords.iter().map(|(key, _)| *key))
        .collect()
}

#[test]
fn no_advertised_key_acts_inside_a_prompt_and_each_is_typed_instead() {
    for key in main_keys() {
        let Key::Char(c) = key else { continue };
        let (app, effects) = press_all(fixture(), &[Key::Char('t'), key]);
        assert_eq!(app.mode, super::app::Mode::Prompt(c.to_string()), "{key:?}");
        assert!(effects.is_empty(), "{key:?}");
    }
}

#[test]
fn the_prompt_acts_on_the_keys_its_footer_advertises() {
    let typed = press(fixture(), &[Key::Char('t'), Key::Char('3'), Key::Char('p')]);
    let erased = press(typed.clone(), &[Key::Backspace]);
    assert_eq!(erased.mode, super::app::Mode::Prompt("3".to_owned()));
    let cancelled = press(typed.clone(), &[Key::Esc]);
    assert_eq!(cancelled.mode, super::app::Mode::Normal);
    let submitted = press(typed, &[Key::Char('m'), Key::Enter]);
    assert_eq!(submitted.say_line(), "3pm AEDT / 2pm AEST / 6am SAST");
}

#[test]
fn each_modal_context_swallows_the_keys_it_does_not_advertise() {
    for opener in ["c", "s", "?"] {
        let open = press(fixture(), &chars(opener));
        let context = super::update::context(&open.mode);
        let own: Vec<Key> = super::keys::BINDINGS
            .iter()
            .filter(|b| b.context == context || b.context == super::keys::Context::Global)
            .flat_map(|b| b.chords.iter().map(|(key, _)| *key))
            .collect();
        for key in main_keys().into_iter().filter(|key| !own.contains(key)) {
            let (after, effects) = press_all(open.clone(), &[key]);
            assert_eq!(after, open, "{opener} then {key:?}");
            assert!(effects.is_empty(), "{opener} then {key:?}");
        }
    }
}

#[test]
fn focus_stays_on_a_visible_row_across_a_group_switch() {
    let team = focus_on(fixture(), "Thandi");
    let deploy = press(team.clone(), &chars("4"));
    let index = |app: &App| app.focus_rows().iter().position(|row| *row == app.focus);
    assert_eq!((index(&team), index(&deploy)), (Some(3), Some(2)));
    assert_eq!(focus_name(&deploy), "Thandi");
    let app = deploy;
    assert!(app.focus_rows().contains(&app.focus));
}

#[test]
fn focus_follows_an_unmarked_row_below_the_line() {
    let app = press(focus_on(fixture(), "Priya"), &chars(" "));
    assert_eq!(focus_name(&app), "Priya");
    assert!(app.who_rows().below.contains(&app.focus));
}

#[test]
fn a_reload_that_drops_the_focused_row_lands_on_the_row_now_in_its_place() {
    let app = focus_on(fixture(), "Marcus");
    let without_marcus = FIXTURE.replace(
        "[[person]]\nname  = \"Marcus\"\nlabel = \"Austin\"\ntz    = \"America/Chicago\"\nat    = [30.27, -97.74]\n",
        "",
    );
    assert_ne!(without_marcus, FIXTURE);
    let reloaded = update(
        app,
        Msg::Reload(Box::new(Config::parse(&without_marcus).unwrap())),
    )
    .0;
    assert!(reloaded.focus_rows().contains(&reloaded.focus));
    assert_eq!(focus_name(&reloaded), "AU");
}

fn focus_name(app: &App) -> String {
    match app.focus {
        teamzones_core::question::Row::Person(id) => app.roster().person(id).name.clone(),
        teamzones_core::question::Row::Place(id) => app.roster().place(id).name.clone(),
    }
}

#[test]
fn the_update_layer_names_no_io_crate() {
    let sources = [
        ("update.rs", include_str!("update.rs")),
        ("app.rs", include_str!("app.rs")),
        ("prompt.rs", include_str!("prompt.rs")),
        ("notice.rs", include_str!("notice.rs")),
        ("say.rs", include_str!("say.rs")),
    ];
    let forbidden = [
        "std::fs",
        "std::io",
        "std::env",
        "std::process",
        "std::time",
        "Utc::now",
        "Clock::System",
        "arboard",
        "crossterm",
        "ratatui",
        "crate::clipboard",
        "Clipboard",
    ];
    let named: Vec<(&str, &str)> = sources
        .iter()
        .flat_map(|(file, source)| {
            forbidden
                .iter()
                .filter(|name| source.contains(*name))
                .map(|name| (*file, *name))
        })
        .collect();
    assert!(named.is_empty(), "the update layer names {named:?}");
}

#[test]
fn small_terminals_draw_without_panicking_until_folding_lands() {
    for (cols, rows) in [(1, 1), (30, 8), (44, 12), (60, 20), (200, 60)] {
        for app in [
            team_monday(),
            press(team_monday(), &chars("?")),
            press(fixture(), &chars("3n")),
        ] {
            render(&app, cols, rows);
        }
    }
}

#[test]
fn s_n_sorts_each_block_by_name_and_s_o_by_offset() {
    let by_name = press(team_monday(), &chars("sn"));
    let names = |app: &App| -> Vec<String> {
        app.who_rows()
            .above
            .iter()
            .map(|row| match row {
                teamzones_core::question::Row::Person(id) => app.roster().person(*id).name.clone(),
                teamzones_core::question::Row::Place(id) => app.roster().place(*id).name.clone(),
            })
            .collect()
    };
    assert_eq!(names(&by_name), ["Paul", "Johan", "Priya", "Sam", "Thandi"]);
    assert_eq!(
        names(&press(by_name, &chars("so"))),
        ["Paul", "Priya", "Sam", "Thandi", "Johan"]
    );
}

#[test]
fn b_draws_people_lanes_as_plain_day_and_night() {
    let plain = render(&press(team_monday(), &chars("b")), 120, 36);
    let paul = row(&plain, 16);
    assert!(
        !paul.contains('▓') && !paul.contains('▚') && paul.contains('▒'),
        "{paul}"
    );
}

#[test]
fn brackets_move_the_range_a_day_and_back() {
    let monday = team_monday();
    let range = monday.range.unwrap();
    let tuesday = press(monday, &chars("]"));
    assert_eq!(tuesday.range.unwrap().start(), range.start().offset(96));
    assert_eq!(press(tuesday, &chars("[")).range, Some(range));
}

#[test]
fn o_swaps_the_active_end_so_h_grows_the_range_earlier() {
    assert_eq!(
        press(team_monday(), &chars("vhv")).say_line(),
        "5-5:45pm AEDT / 4-4:45pm AEST / 8-8:45am SAST"
    );
    assert_eq!(
        press(team_monday(), &chars("vohv")).say_line(),
        "4:45-6pm AEDT / 3:45-5pm AEST / 7:45-9am SAST"
    );
}

#[test]
fn g_and_capital_g_focus_the_first_and_last_rows() {
    let last = press(fixture(), &chars("G"));
    assert_eq!(focus_name(&last), "US");
    assert_eq!(focus_name(&press(last, &chars("g"))), "Paul");
}

#[test]
fn shift_arrows_move_the_range_an_hour() {
    assert_eq!(
        press(team_monday(), &[Key::ShiftRight]).say_line(),
        "6-7pm AEDT / 5-6pm AEST / 9-10am SAST"
    );
    assert_eq!(
        press(team_monday(), &[Key::ShiftLeft]).say_line(),
        "4-5pm AEDT / 3-4pm AEST / 7-8am SAST"
    );
}

#[test]
fn ruler_labels_follow_the_local_clock_on_a_daylight_saving_change() {
    let buf = render(&go_to(fixture(), "2026-10-04 3pm"), 120, 36);
    let y = y_of(&buf, "Sun 4 Oct AEDT");
    let column_of = |label: &str| {
        let cells: Vec<&str> = (0..buf.area.width).map(|x| buf[(x, y)].symbol()).collect();
        let label: Vec<String> = label.chars().map(String::from).collect();
        cells
            .windows(label.len())
            .position(|window| window == label.as_slice())
    };
    assert_eq!(
        (column_of("12a"), column_of("3a"), column_of("6a")),
        (Some(24), Some(24 + 8), Some(24 + 20)),
        "Melbourne skips 2am to 3am on 4 Oct, so 3am is two hours in"
    );
}

#[test]
fn the_person_detail_names_their_hours_and_work_days() {
    let buf = render(&focus_on(team_monday(), "Priya"), 120, 36);
    let detail = row(&buf, y_of(&buf, "▶ Priya"));
    assert!(
        detail.contains(
            "▶ Priya  Brisbane  AEST UTC+10  hours 9am-5pm Mon-Fri  groups team, au, deploy  Mon 26 Oct 4-5pm in hours"
        ),
        "{detail}"
    );
}

#[test]
fn capital_n_names_the_previous_fit() {
    let app = press(go_to(fixture(), "26 oct 7pm"), &chars("N"));
    assert!(
        app.notice
            .unwrap()
            .text
            .starts_with("N  previous fit for team: Mon 5-6pm AEDT"),
    );
}

const VENUE: &str = "name  = \"venue\"\nkind  = \"trading\"\n";

/// The fixture with the venue profile, which every place uses, given `labels`.
fn venue_labelled(labels: &str) -> String {
    FIXTURE.replacen(VENUE, &format!("{VENUE}labels = {labels}\n"), 1)
}

fn where_border(app: &App) -> String {
    let buf = render(app, 120, 36);
    row(&buf, y_of(&buf, " where "))
}

#[test]
fn the_where_legend_names_the_default_levels() {
    assert!(
        where_border(&team_monday()).ends_with("▁ closed  ▄ open  █ busy"),
        "{}",
        where_border(&team_monday())
    );
}

#[test]
fn the_where_legend_shows_labels_every_place_shares_and_the_defaults_otherwise() {
    let labels = "{ closed = \"shut\", open = \"trading hours\", busy = \"rush\" }";
    let shared = Config::parse(&venue_labelled(labels)).unwrap();
    let app = go_to(App::new(shared, now()), "26 oct 5-6pm");
    assert!(
        where_border(&app).ends_with("▁ shut  ▄ trading hours  █ rush"),
        "{}",
        where_border(&app)
    );
    let mixed = venue_labelled(labels).replacen(
        "tz      = \"America/Chicago\"\ntrading = \"venue\"",
        "tz      = \"America/Chicago\"\ntrading = \"desk\"",
        1,
    ) + "\n[[profile]]\nname = \"desk\"\nkind = \"trading\"\nopen = \"7am-11pm\"\n";
    let app = go_to(
        App::new(Config::parse(&mixed).unwrap(), now()),
        "26 oct 5-6pm",
    );
    assert!(
        where_border(&app).ends_with("▁ closed  ▄ open  █ busy"),
        "{}",
        where_border(&app)
    );
}

#[test]
fn a_place_row_shows_its_profile_s_word_for_its_state() {
    let labels = "{ closed = \"shut\", open = \"trading hours\", busy = \"rush\" }";
    let app = go_to(
        App::new(Config::parse(&venue_labelled(labels)).unwrap(), now()),
        "26 oct 5-6pm",
    );
    let buf = render(&app, 120, 36);
    assert!(row(&buf, y_of(&buf, "◇ AU")).contains("rush"));
    assert!(row(&buf, y_of(&buf, "◇ UK")).contains("shut"));
}
