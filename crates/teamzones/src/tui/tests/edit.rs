//! Editing groups and rows, and writing them back to the config file byte for byte.

use super::*;

/// The fixture with comments above a group and after a key, which every edit must keep.
fn commented() -> String {
    FIXTURE
        .replacen(
            "[[group]]\nname   = \"team\"",
            "# the people I tell first\n[[group]]\nname   = \"team\"",
            1,
        )
        .replacen(
            "source = \"sun\"",
            "source = \"sun\" # sunrise and sunset",
            1,
        )
}

/// The fixture app reading its config from a file holding `text`.
fn editable(text: &str) -> App {
    App::new(Config::parse(text).unwrap(), now()).with_source(Some(text.to_owned()))
}

/// The text each write effect puts in the file.
fn written(effects: &[Effect]) -> Vec<String> {
    effects
        .iter()
        .filter_map(|effect| match effect {
            Effect::Write { text, .. } => Some(text.clone()),
            Effect::Copy { .. }
            | Effect::CaptureMouse(_)
            | Effect::Reload
            | Effect::Edit { .. } => None,
        })
        .collect()
}

fn notice_text(app: &App) -> String {
    app.notice
        .as_ref()
        .map(|notice| notice.text.clone())
        .unwrap_or_default()
}

#[test]
fn space_on_uk_then_w_on_team_writes_its_places_and_nothing_else() {
    let before = commented();
    let (app, effects) = press_all(focus_on(editable(&before), "UK"), &chars(" w"));
    let people = "people = [\"Priya\", \"Sam\", \"Thandi\", \"Johan\"]\n";
    let after = before.replacen(people, &format!("{people}places = [\"UK\"]\n"), 1);
    assert_ne!(after, before);
    assert_eq!(written(&effects), [after]);
    assert_eq!(notice_text(&app), "w  team: +@UK saved  u undo");
    assert!(app.drift().is_empty(), "{:?}", app.drift());
}

/// `before` with the `[[group]]` table named `name` (and the blank line before it) cut out.
fn without_group(before: &str, name: &str) -> String {
    let start = before
        .find(&format!("\n[[group]]\nname   = \"{name}\"\n"))
        .unwrap();
    let end = before[start + 1..]
        .find("\n[")
        .map_or(before.len(), |at| start + 1 + at);
    format!("{}{}", &before[..start], &before[end..])
}

fn tick(app: App, seconds: i64) -> (App, Vec<Effect>) {
    update(app, Msg::Tick(now() + chrono::TimeDelta::seconds(seconds)))
}

#[test]
fn u_within_ten_seconds_writes_the_file_back_to_its_previous_bytes() {
    let before = commented();
    let saved = press(focus_on(editable(&before), "UK"), &chars(" w"));
    let (undone, effects) = press_all(saved.clone(), &chars("u"));
    assert_eq!(written(&effects), [before]);
    assert_eq!(undone.drift(), ["+@UK"], "the marks are back as they were");
    let (late, _) = tick(saved, 10);
    assert_eq!(notice_text(&late), "", "the notice is gone after 10 s");
    let (after, effects) = press_all(late.clone(), &chars("u"));
    assert!(written(&effects).is_empty(), "{effects:?}");
    assert_eq!(after.config, late.config);
}

fn strip_row(app: &App) -> String {
    let buf = render(app, 120, 36);
    row(&buf, y_of(&buf, " who "))
}

#[test]
fn x_on_deploy_leaves_the_strip_at_once_and_writes_only_after_ten_seconds() {
    let before = commented();
    let (closed, effects) = press_all(editable(&before), &chars("4x"));
    assert!(written(&effects).is_empty(), "{effects:?}");
    assert!(
        !strip_row(&closed).contains("deploy"),
        "{}",
        strip_row(&closed)
    );
    assert_eq!(
        notice_text(&closed),
        "x  closed deploy: its people and places stay  u undo"
    );
    let (waiting, effects) = tick(closed, 9);
    assert!(written(&effects).is_empty(), "{effects:?}");
    let (_, effects) = tick(waiting, 10);
    assert_eq!(written(&effects), [without_group(&before, "deploy")]);
}

#[test]
fn x_then_quit_writes_the_close_at_once() {
    let before = commented();
    let (quit, effects) = press_all(editable(&before), &chars("4xq"));
    assert!(!quit.running);
    assert_eq!(written(&effects), [without_group(&before, "deploy")]);
}

#[test]
fn u_after_x_puts_the_group_back_in_its_slot_and_nothing_is_written() {
    let before = commented();
    let (undone, effects) = press_all(editable(&before), &chars("4xu"));
    assert!(written(&effects).is_empty(), "{effects:?}");
    assert!(
        strip_row(&undone).contains("3 regions  4 deploy  0 all"),
        "{}",
        strip_row(&undone)
    );
    assert_eq!(undone.lens_name(), "deploy");
    let (_, effects) = tick(undone, 10);
    assert!(written(&effects).is_empty(), "{effects:?}");
}

#[test]
fn plus_saves_the_marks_as_a_new_group_named_from_the_expression() {
    let before = commented();
    let naming = press(focus_on(editable(&before), "UK"), &chars(" +"));
    assert_eq!(
        naming.mode,
        super::super::app::Mode::NewGroup("team_uk".to_owned())
    );
    assert!(
        strip_row(&naming).contains("+ team_uk_"),
        "{}",
        strip_row(&naming)
    );
    let (saved, effects) = press_all(naming, &[Key::Enter]);
    let group = "\n[[group]]\nname = \"team_uk\"\npeople = [\"Priya\", \"Sam\", \"Thandi\", \"Johan\"]\nplaces = [\"UK\"]\n";
    let after = before.replacen("\n[theme]", &format!("{group}\n[theme]"), 1);
    assert_eq!(written(&effects), [after]);
    assert_eq!(saved.lens_name(), "team_uk");
    assert!(saved.drift().is_empty(), "{:?}", saved.drift());
    assert!(
        strip_row(&saved).contains("5 team_uk"),
        "{}",
        strip_row(&saved)
    );
    assert_eq!(notice_text(&saved), "+  saved team_uk  u undo");
}

#[test]
fn w_on_all_asks_for_a_name_as_plus_does() {
    let app = press(editable(&commented()), &chars("0w"));
    assert_eq!(
        app.mode,
        super::super::app::Mode::NewGroup("all".to_owned())
    );
}

#[test]
fn a_name_already_taken_is_refused_and_the_file_is_left_alone() {
    let app = press(editable(&commented()), &chars("+"));
    assert_eq!(
        app.mode,
        super::super::app::Mode::NewGroup("team".to_owned())
    );
    let (refused, effects) = press_all(app, &[Key::Enter]);
    assert!(written(&effects).is_empty(), "{effects:?}");
    let notice = notice_text(&refused);
    assert!(notice.starts_with("+  not saved: line "), "{notice}");
    assert!(notice.contains("\"team\" is already a name"), "{notice}");
}

#[test]
fn equals_cycles_where_as_drift_until_w_skipping_context_without_people() {
    let rules = |app: &App, keys: &str| -> Vec<String> {
        (1..=keys.len())
            .map(|n| press(app.clone(), &chars(&keys[..n])).drift().join(" "))
            .collect()
    };
    let regions = press(editable(&commented()), &chars("3"));
    assert_eq!(rules(&regions, "=="), ["=closed", ""]);
    let deploy = press(editable(&commented()), &chars("4"));
    assert_eq!(rules(&deploy, "==="), ["=closed", "=context", ""]);
    let before = commented();
    let (_, effects) = press_all(press(editable(&before), &chars("3")), &chars("=w"));
    let places = "places  = [\"AU\", \"UK\", \"US\"]\n";
    let after = before.replacen(places, &format!("{places}where = \"closed\"\n"), 1);
    assert_eq!(written(&effects), [after]);
}

/// What `teamzones say` prints with `words` on a config file holding `text`.
fn say_on(text: &str, words: &[&str]) -> Result<String, String> {
    let path = scratch_dir("say").join("config.toml");
    std::fs::write(&path, text).unwrap();
    let argv = [
        "teamzones",
        "--now",
        "2026-10-23T22:04:00Z",
        "say",
        "--no-copy",
    ]
    .iter()
    .chain(words)
    .map(|word| (*word).to_owned())
    .collect();
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let mut io = crate::cli::Io {
        out: &mut out,
        err: &mut err,
        stdout_is_tty: false,
        system_zone: chrono_tz::UTC,
        config_path: path,
        clipboard: &mut NoClipboard,
    };
    use clap::Parser;
    let cli = crate::cli::Cli::parse_from(crate::cli::hoist_flags(argv));
    crate::cli::run(cli, &mut io).map_err(|failure| failure.to_string())?;
    Ok(String::from_utf8(out).unwrap())
}

fn typed(text: &str) -> Vec<Key> {
    chars(text)
}

#[test]
fn matrix_changes_are_written_together_on_esc_and_the_old_name_still_says() {
    let before = commented();
    let open = press(editable(&before), &[Key::Char('4'), Key::Tab]);
    let mut keys = typed("jj R");
    keys.extend(std::iter::repeat_n(Key::Backspace, "deploy".len()));
    keys.extend(typed("release"));
    keys.extend([Key::Enter, Key::Char('<')]);
    let (changed, effects) = press_all(open, &keys);
    assert!(
        written(&effects).is_empty(),
        "nothing is written before esc"
    );
    let (closed, effects) = press_all(changed, &[Key::Esc]);
    let after = written(&effects);
    assert_eq!(after.len(), 1, "{after:?}");
    let groups: Vec<(String, Vec<String>, Vec<String>)> = Config::parse(&after[0])
        .unwrap()
        .roster
        .groups
        .iter()
        .map(|g| {
            (
                g.name.clone(),
                g.aliases.clone(),
                g.people
                    .iter()
                    .map(|id| closed.roster().person(*id).name.clone())
                    .collect(),
            )
        })
        .collect();
    let names: Vec<&str> = groups.iter().map(|(name, _, _)| name.as_str()).collect();
    assert_eq!(names, ["team", "au", "release", "regions"]);
    assert_eq!(groups[2].1, ["deploy"]);
    assert_eq!(groups[2].2, ["Priya", "Sam", "Thandi"]);
    assert!(after[0].contains("# the people I tell first\n[[group]]\nname   = \"team\""));
    assert_eq!(
        say_on(&after[0], &["deploy", "next"]),
        say_on(&before, &["deploy", "next"]),
        "Sam works Priya's hours in her zone, so deploy's next fit reads the same"
    );
    assert_eq!(notice_text(&closed), "tab  groups saved  u undo");
}

/// The matrix the app shows.
fn matrix_of(app: &App) -> &super::super::matrix::Matrix {
    super::super::matrix::of(app).expect("the matrix is open")
}

#[test]
fn the_matrix_draws_people_then_places_against_every_group_with_its_question() {
    let open = press(
        focus_on(editable(&commented()), "US"),
        &[Key::Char('4'), Key::Tab],
    );
    let buf = render(&open, 120, 36);
    let line = |needle: &str| {
        let line = row(&buf, y_of(&buf, needle));
        line.split('│').nth(1).unwrap_or_default().to_owned()
    };
    assert!(line("1 team").contains("2 au") && line("1 team").contains("+ new"));
    let asks_line = line("asks");
    let asks: Vec<&str> = asks_line.split_whitespace().skip(1).collect();
    assert_eq!(asks, ["who", "who", "where", "both"]);
    let cells = |label: &str| -> String {
        let line = line(label);
        line.split_once(label)
            .map(|(_, cells)| cells.split_whitespace().collect())
            .unwrap_or_default()
    };
    assert_eq!(cells("Priya   Brisbane"), "●●·●");
    assert_eq!(cells("Paul    Melbourne"), "⌂⌂⌂⌂");
    assert_eq!(cells("US      venue profile"), "··◆·");
    assert!(line("cursor").contains("space adds the place US to deploy (its state, not its zone)"));
    assert!(line("cli ").contains("teamzones say deploy now"));
    insta::assert_snapshot!("matrix_120x36", text(&buf));
}

#[test]
fn u_in_the_matrix_steps_back_one_change_and_nothing_is_written() {
    let open = press(editable(&commented()), &[Key::Char('4'), Key::Tab]);
    let toggled = press(open.clone(), &typed("jj "));
    assert_ne!(toggled.config, open.config);
    let (undone, effects) = press_all(toggled, &typed("u"));
    assert_eq!(undone.config, open.config);
    assert!(written(&effects).is_empty(), "{effects:?}");
    let (closed, effects) = press_all(undone, &[Key::Esc]);
    assert!(written(&effects).is_empty(), "{effects:?}");
    assert_eq!(closed.mode, super::super::app::Mode::Normal);
}

#[test]
fn x_in_the_matrix_closes_the_column_and_the_cursor_stays_on_the_grid() {
    let before = commented();
    let open = press(editable(&before), &[Key::Char('4'), Key::Tab]);
    let closed = press(open, &typed("x"));
    assert_eq!(closed.roster().groups.len(), 3);
    assert_eq!(matrix_of(&closed).column, 2);
    assert!(closed.focus_rows().contains(&closed.focus));
    let (_, effects) = press_all(closed, &[Key::Esc]);
    assert_eq!(written(&effects), [without_group(&before, "deploy")]);
}

#[test]
fn enter_in_the_matrix_uses_the_group_under_the_cursor() {
    let open = press(editable(&commented()), &[Key::Tab]);
    let used = press(open, &[Key::Char('l'), Key::Char('l'), Key::Enter]);
    assert_eq!(used.lens_name(), "regions");
    assert_eq!(used.mode, super::super::app::Mode::Normal);
}

#[test]
fn home_is_locked_in_every_group() {
    let open = press(editable(&commented()), &[Key::Tab]);
    let (pressed, _) = press_all(open.clone(), &typed(" "));
    assert_eq!(pressed.config, open.config);
    assert_eq!(notice_text(&pressed), "tab  home belongs to every group");
}

#[test]
fn a_in_the_matrix_adds_a_named_group_as_a_new_column() {
    let open = press(editable(&commented()), &[Key::Tab]);
    let mut keys = typed("a");
    keys.extend(typed("oncall"));
    keys.push(Key::Enter);
    let added = press(open, &keys);
    assert_eq!(
        added.roster().groups.last().map(|g| g.name.as_str()),
        Some("oncall")
    );
    assert_eq!(matrix_of(&added).column, 4);
}

/// Each field's text then `enter`, as typed into the picker.
fn fill(fields: &[&str]) -> Vec<Key> {
    fields
        .iter()
        .flat_map(|field| chars(field).into_iter().chain([Key::Enter]))
        .collect()
}

#[test]
fn a_p_adds_a_person_after_the_last_one_through_the_picker() {
    let before = commented();
    let mut keys = chars("ap");
    keys.extend(fill(&["Kofi", "Accra", "8am-4pm"]));
    let (added, effects) = press_all(editable(&before), &keys);
    let person = "\n[[person]]\nname = \"Kofi\"\ncity = \"Accra\"\nhours = \"8am-4pm\"\n";
    let after = before.replacen("\n[[place]]", &format!("{person}\n[[place]]"), 1);
    assert_eq!(written(&effects), [after]);
    assert_eq!(notice_text(&added), "a  added Kofi  u undo");
    assert_eq!(super::focus_name(&added), "Kofi");
}

#[test]
fn a_l_adds_a_place_after_the_last_one_through_the_picker() {
    let before = commented();
    let mut keys = chars("al");
    keys.extend(fill(&["NZ", "Pacific/Auckland", "venue", "-41.3, 174.8"]));
    let (added, effects) = press_all(editable(&before), &keys);
    let place = "\n[[place]]\nname = \"NZ\"\ntz = \"Pacific/Auckland\"\ntrading = \"venue\"\nat = [-41.3, 174.8]\n";
    let after = before.replacen("\n[[profile]]", &format!("{place}\n[[profile]]"), 1);
    assert_eq!(written(&effects), [after]);
    assert_eq!(super::focus_name(&added), "NZ");
}

#[test]
fn a_place_without_a_marker_takes_a_city_in_its_zone() {
    let mut keys = chars("al");
    keys.extend(fill(&["NZ", "Pacific/Auckland", "", ""]));
    let (added, effects) = press_all(editable(&commented()), &keys);
    assert_eq!(written(&effects).len(), 1, "{}", notice_text(&added));
    let nz = added.roster().place_named("NZ").unwrap();
    let at = added.roster().place(nz).location;
    assert!(
        (at.lat + 36.8).abs() < 1.0 && (at.lon - 174.8).abs() < 1.0,
        "{at:?}"
    );
}

#[test]
fn e_edits_the_focused_person_s_hours_and_writes_only_that_line() {
    let before = commented();
    let mut keys = vec![Key::Char('e'), Key::Enter, Key::Enter];
    keys.extend(std::iter::repeat_n(Key::Backspace, "9am-5pm".len()));
    keys.extend(fill(&["10am-6pm"]));
    let (edited, effects) = press_all(focus_on(editable(&before), "Priya"), &keys);
    let after = before.replacen(
        "name  = \"Priya\"\ncity  = \"Brisbane\"\nhours = \"9am-5pm\"",
        "name  = \"Priya\"\ncity  = \"Brisbane\"\nhours = \"10am-6pm\"",
        1,
    );
    assert_ne!(after, before);
    assert_eq!(written(&effects), [after]);
    assert_eq!(notice_text(&edited), "e  saved Priya  u undo");
}

#[test]
fn renaming_a_person_renames_them_in_every_group() {
    let mut keys = vec![Key::Char('e')];
    keys.extend(std::iter::repeat_n(Key::Backspace, "Priya".len()));
    keys.extend(fill(&["Pria", "", ""]));
    let (_, effects) = press_all(focus_on(editable(&commented()), "Priya"), &keys);
    let after = &written(&effects)[0];
    assert!(!after.contains("\"Priya\""), "{after}");
    assert_eq!(after.matches("\"Pria\"").count(), 4, "{after}");
}

#[test]
fn d_deletes_the_row_from_config_and_every_group_and_u_puts_it_back() {
    let before = commented();
    let (deleted, effects) = press_all(focus_on(editable(&before), "Priya"), &chars("d"));
    let after = written(&effects);
    assert_eq!(after.len(), 1);
    let config = Config::parse(&after[0]).unwrap();
    assert!(config.roster.person_named("Priya").is_none());
    assert!(!after[0].contains("Priya"), "{}", after[0]);
    assert_eq!(
        notice_text(&deleted),
        "d  deleted Priya from config and 3 groups  u undo"
    );
    assert!(deleted.focus_rows().contains(&deleted.focus));
    let (_, effects) = press_all(deleted, &chars("u"));
    assert_eq!(written(&effects), [before]);
}

#[test]
fn d_on_a_row_in_no_group_names_no_groups() {
    let (deleted, _) = press_all(focus_on(editable(&commented()), "Ellie"), &chars("d"));
    assert_eq!(
        notice_text(&deleted),
        "d  deleted Ellie from config  u undo"
    );
}

#[test]
fn home_cannot_be_deleted() {
    let (kept, effects) = press_all(editable(&commented()), &chars("d"));
    assert!(written(&effects).is_empty());
    assert_eq!(notice_text(&kept), "d  not saved: home cannot be deleted");
}

#[test]
fn r_with_a_broken_file_keeps_the_old_config_and_shows_the_parse_error() {
    let path = scratch_dir("reload").join("config.toml");
    let broken = FIXTURE.replacen("hours = \"9am-5pm\"", "hour = \"9am-5pm\"", 1);
    std::fs::write(&path, &broken).unwrap();
    let app = editable(FIXTURE);
    let (pressed, effects) = press_all(app.clone(), &chars("r"));
    let reloaded = run_against(pressed, effects, &path, &mut Recorder::default());
    assert_eq!(reloaded.config, app.config);
    assert_eq!(
        notice_text(&reloaded),
        "r  config error line 14: unknown key \"hour\", kept the last good config"
    );
    std::fs::write(&path, FIXTURE.replacen("Ellie", "Elena", 1)).unwrap();
    let (pressed, effects) = press_all(reloaded, &chars("r"));
    let reloaded = run_against(pressed, effects, &path, &mut Recorder::default());
    assert!(reloaded.roster().person_named("Elena").is_some());
    assert_eq!(notice_text(&reloaded), "r  config reloaded");
}

#[test]
fn the_effects_layer_writes_the_bytes_and_refuses_a_file_changed_behind_its_back() {
    let path = scratch_dir("write").join("config.toml");
    let before = commented();
    std::fs::write(&path, &before).unwrap();
    let (saved, effects) = press_all(focus_on(editable(&before), "UK"), &chars(" w"));
    let expected = written(&effects);
    let saved = run_against(saved, effects, &path, &mut Recorder::default());
    assert_eq!([std::fs::read_to_string(&path).unwrap()], *expected);
    assert_eq!(notice_text(&saved), "w  team: +@UK saved  u undo");
    std::fs::write(&path, "# edited elsewhere\n").unwrap();
    let (undone, effects) = press_all(saved, &chars("u"));
    let undone = run_against(undone, effects, &path, &mut Recorder::default());
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        "# edited elsewhere\n"
    );
    assert!(
        notice_text(&undone).contains("changed outside teamzones"),
        "{}",
        notice_text(&undone)
    );
}

/// Runs the shell's event loop on a 120 by 36 test terminal until the app quits, with `editor`
/// as the editor and `feed` as the terminal's input.
fn run_loop(
    app: App,
    path: &std::path::Path,
    feed: &Shared,
    editor: &mut Editor,
    modes: &mut Recorder,
) -> App {
    let mut terminal = Terminal::new(TestBackend::new(120, 36)).unwrap();
    let mut events = feed.clone();
    let mut io = super::super::Io {
        clipboard: &mut NoClipboard,
        modes,
        events: &mut events,
        opener: editor,
        config: path,
        system_zone: chrono_tz::Australia::Melbourne,
        editors: (None, Some("nano -w".to_owned())),
        repaint: false,
    };
    let clock = teamzones_core::clock::Clock::Fixed(now());
    super::super::event_loop(&mut terminal, app, &clock, TerminalEnv::default(), &mut io).unwrap()
}

#[test]
fn capital_e_hands_the_terminal_to_the_editor_and_drops_its_input_then_reloads() {
    let ms = std::time::Duration::from_millis;
    let path = scratch_dir("editor").join("config.toml");
    std::fs::write(&path, FIXTURE).unwrap();
    let feed = Shared::default();
    feed.0.borrow_mut().key_after(ms(1), 'E');
    let mut editor = Editor::new(&feed);
    // Typed while the editor runs, then a late terminal reply 10 ms after it exits: none of it
    // may reach the update. The q 50 ms after it exits is the user's, and quits.
    editor.typed = vec!['3', 'q'];
    editor.after = vec![(ms(10), 'x'), (ms(50), 'q')];
    editor.writes = Some((path.clone(), FIXTURE.replacen("Ellie", "Elena", 1)));
    let mut modes = Recorder::default();
    let app = run_loop(editable(FIXTURE), &path, &feed, &mut editor, &mut modes);
    assert_eq!(
        editor.runs,
        [(
            "nano".to_owned(),
            vec!["-w".to_owned(), path.display().to_string()]
        )]
    );
    assert_eq!(
        (modes.suspended, modes.resumed.as_slice()),
        (1, [true].as_slice())
    );
    assert_eq!(
        app.lens_name(),
        "team",
        "the 3 typed into the editor never reached the update"
    );
    assert_eq!(
        app.roster().groups.len(),
        4,
        "the late x never closed a group"
    );
    assert!(
        app.roster().person_named("Elena").is_some(),
        "reloaded on return"
    );
    assert!(!app.running, "the q after the grace window quit");
}

#[test]
fn a_failing_editor_still_gives_the_terminal_back_and_reloads_nothing() {
    let path = scratch_dir("editor-fails").join("config.toml");
    std::fs::write(&path, FIXTURE).unwrap();
    let mut feed = Shared::default();
    let mut editor = Editor::new(&feed);
    editor.succeeds = Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "no such editor",
    ));
    let mut modes = Recorder::default();
    let mut io = super::super::Io {
        clipboard: &mut NoClipboard,
        modes: &mut modes,
        events: &mut feed,
        opener: &mut editor,
        config: &path,
        system_zone: chrono_tz::Australia::Melbourne,
        editors: (None, None),
        repaint: false,
    };
    let result = super::super::editor::hand_off(&mut io, false);
    assert_eq!(result, Err("could not run vi: no such editor".to_owned()));
    assert_eq!(
        (modes.suspended, modes.resumed.as_slice()),
        (1, [false].as_slice())
    );
    let app = update(
        editable(FIXTURE),
        Msg::Reloaded {
            key: "E".to_owned(),
            result,
        },
    )
    .0;
    assert_eq!(app.config, editable(FIXTURE).config);
    assert_eq!(notice_text(&app), "E  could not run vi: no such editor");
}
