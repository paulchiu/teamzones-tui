//! Golden tests: every row comes from the spec tables in `docs/spec/`, never retyped here.

use chrono::{DateTime, Utc};
use teamzones_core::clock::When;
use teamzones_core::config::{Config, PersonId, Roster};
use teamzones_core::copy::{Clause, Formats, Variant, format_line, say_line};
use teamzones_core::expr::parse_expr;
use teamzones_core::fit::resolve;
use teamzones_core::question::{Marks, Row};
use teamzones_core::status::status_lines;
use teamzones_core::time::{Moment, parse_time};

const FIXTURE: &str = include_str!("fixtures/sample.toml");
const CONFIG_SPEC: &str = include_str!("../../../docs/spec/config.md");
const COPY_SPEC: &str = include_str!("../../../docs/spec/copy.md");
const CLI_SPEC: &str = include_str!("../../../docs/spec/cli.md");

/// Sat 24 Oct 2026, 9:04am AEDT.
fn now() -> DateTime<Utc> {
    "2026-10-23T22:04:00Z".parse().unwrap()
}

fn roster() -> Roster {
    Config::parse(FIXTURE).unwrap().roster
}

/// The body rows of the first table after `heading`, as trimmed cells.
fn table_rows(spec: &str, heading: &str) -> Vec<Vec<String>> {
    let after = spec
        .split_once(heading)
        .unwrap_or_else(|| panic!("no {heading:?} in the spec"))
        .1;
    after
        .lines()
        .skip_while(|line| !line.starts_with('|'))
        .take_while(|line| line.starts_with('|'))
        .skip(2)
        .map(|line| {
            line.trim_matches('|')
                .split(" | ")
                .map(|cell| cell.trim().to_owned())
                .collect()
        })
        .collect()
}

/// The text inside each pair of backticks in `cell`.
fn backticked(cell: &str) -> Vec<&str> {
    cell.split('`').skip(1).step_by(2).collect()
}

fn fixed(text: &str, roster: &Roster) -> When {
    match parse_time(text, roster, now()).unwrap() {
        Moment::Fixed(when) => when,
        other => panic!("{text} is not a fixed time: {other:?}"),
    }
}

/// Reads "Mon 26 Oct 5-6pm" as "26 Oct 5-6pm": the grammar takes one day, and the date wins.
fn without_weekday_before_date(text: &str) -> String {
    let tokens: Vec<&str> = text.split_whitespace().collect();
    let skip = usize::from(
        tokens
            .get(1)
            .is_some_and(|t| t.bytes().all(|b| b.is_ascii_digit())),
    );
    tokens[skip..].join(" ")
}

fn copy_row(index: usize) {
    let rows = table_rows(COPY_SPEC, "## Copy golden table");
    let [setup, keys, clipboard] = rows[index].as_slice() else {
        panic!("copy row {index} needs three cells")
    };
    let roster = roster();
    let mut parts = setup.split(", ");
    let group = roster.group_named(parts.next().unwrap()).unwrap();
    let mut marks = Marks::of_group(group);
    let (mut when, mut focus) = (None, None);
    for part in parts {
        if let Some(name) = part.strip_prefix("`space` on ") {
            let place = roster.place_named(name).unwrap();
            if !marks.places.remove(&place) {
                marks.places.insert(place);
            }
        } else if let Some(name) = part.strip_prefix("focus ") {
            focus = Some(Row::Place(roster.place_named(name).unwrap()));
        } else {
            when = Some(fixed(&without_weekday_before_date(part), &roster));
        }
    }
    let when = when.unwrap();
    let key = match backticked(keys)[0] {
        "y" => 'y',
        other => other
            .strip_prefix("c ")
            .and_then(|letter| letter.chars().next())
            .unwrap_or_else(|| panic!("no format for keys {other:?}")),
    };
    let formats = Formats::builtin();
    let format = formats.get(key).unwrap();
    let focus = focus.unwrap_or(Row::Person(PersonId::HOME));
    let line = format_line(&roster, &marks, focus, when, format);
    assert_eq!(
        line,
        backticked(clipboard)[0],
        "copy row {}: {setup}, {keys}",
        index + 1
    );
}

/// Runs `say` the way the CLI will: flags pulled out, EXPR and TIME parsed, `next` dated.
fn cli_row(index: usize) {
    let rows = table_rows(CLI_SPEC, "## CLI golden table");
    let [command, output] = rows[index].as_slice() else {
        panic!("cli row {index} needs two cells")
    };
    let roster = roster();
    let command = backticked(command)[0];
    let mut variant = Variant::default();
    let words = command
        .strip_prefix("say ")
        .unwrap_or_else(|| panic!("{command} is not a say"))
        .split(' ');
    let tokens: Vec<&str> = words
        .filter(|token| {
            match *token {
                "--zones" => variant.clause = Clause::Zones,
                "--names" => variant.names = true,
                "--date" => variant.date = true,
                "--no-clause" => variant.clause = Clause::Drop,
                "--no-copy" => {}
                _ => return true,
            }
            false
        })
        .collect();
    let expected = backticked(output)[0];
    let result = parse_expr(&tokens, &roster, now()).map(|expr| {
        let dated = expr.moment == Moment::Next;
        let when = resolve(expr.moment, &roster, &expr.marks, now()).unwrap();
        say_line(
            &roster,
            &expr.marks,
            when,
            Variant {
                date: variant.date || dated,
                ..variant
            },
        )
    });
    if output.starts_with("(stderr)") {
        assert_eq!(
            result.unwrap_err().to_string(),
            expected,
            "cli row {}: {command}",
            index + 1
        );
    } else {
        assert_eq!(
            result.unwrap(),
            expected,
            "cli row {}: {command}",
            index + 1
        );
    }
}

/// The `$ teamzones status ...` blocks in the CLI spec, as (expression tokens, expected lines).
fn status_blocks() -> Vec<(String, Vec<String>)> {
    let fence = CLI_SPEC
        .split_once("```\n$ teamzones status")
        .unwrap()
        .1
        .split_once("```")
        .unwrap()
        .0;
    format!("$ teamzones status{fence}")
        .split("$ teamzones status ")
        .skip(1)
        .map(|block| {
            let mut lines = block.lines();
            let command = lines.next().unwrap();
            let expr = command.split(" --now").next().unwrap().to_owned();
            (expr, lines.map(str::to_owned).collect())
        })
        .collect()
}

fn status_block(index: usize) {
    let (expr, expected) = &status_blocks()[index];
    let roster = roster();
    let tokens: Vec<&str> = expr.split(' ').collect();
    let parsed = parse_expr(&tokens, &roster, now()).unwrap();
    assert_eq!(
        status_lines(&roster, &parsed.marks, now()),
        *expected,
        "status {expr}"
    );
}

macro_rules! golden {
    ($runner:ident: $($name:ident = $index:expr;)*) => {
        $(
            #[test]
            fn $name() {
                $runner($index);
            }
        )*
    };
}

golden! { copy_row:
    copy_1_team_mon_5_6pm_y = 0;
    copy_2_regions_mon_2_4pm_y = 1;
    copy_3_regions_mon_2_4pm_c_d = 2;
    copy_4_regions_mon_2_4pm_c_z = 3;
    copy_5_deploy_mon_5_530pm_y = 4;
    copy_6_deploy_mon_5_530pm_c_w = 5;
    copy_7_deploy_mon_5_530pm_c_s = 6;
    copy_8_team_space_on_uk_mon_5_6pm_y = 7;
    copy_9_regions_mon_2_4pm_focus_uk_c_t = 8;
}

golden! { cli_row:
    cli_1_say_team_3pm = 0;
    cli_2_say_au_3pm = 1;
    cli_3_say_3pm = 2;
    cli_4_say_apps_3pm = 3;
    cli_5_say_apps_3pm_zones = 4;
    cli_6_say_regions_2_4pm_mon = 5;
    cli_7_say_regions_minus_us_530pm_mon = 6;
    cli_8_say_regions_minus_au_place_530pm_mon = 7;
    cli_9_say_team_plus_uk_5pm_mon = 8;
    cli_10_say_deploy_next = 9;
    cli_11_say_regions_next = 10;
    cli_12_say_team_next = 11;
    cli_13_say_uk_now_is_a_place = 12;
    cli_14_say_regions_minus_au_people = 13;
}

golden! { status_block:
    status_regions = 0;
    status_team = 1;
}

#[test]
fn the_cli_golden_table_and_status_blocks_have_one_named_test_each() {
    assert_eq!(table_rows(CLI_SPEC, "## CLI golden table").len(), 14);
    assert_eq!(status_blocks().len(), 2);
}

#[test]
fn the_copy_golden_table_has_one_named_test_per_row() {
    assert_eq!(table_rows(COPY_SPEC, "## Copy golden table").len(), 9);
}

#[test]
fn the_fenced_example_in_the_config_spec_is_the_fixture_byte_for_byte() {
    let fenced = CONFIG_SPEC
        .split_once("```toml\n")
        .unwrap()
        .1
        .split_once("```")
        .unwrap()
        .0;
    assert_eq!(fenced, FIXTURE);
}
