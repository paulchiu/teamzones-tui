//! The headless CLI, run as a binary against the fixture with a fixed now and no copy.

use assert_cmd::Command;

const FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../teamzones-core/tests/fixtures/sample.toml"
);
const CLI_SPEC: &str = include_str!("../../../docs/spec/cli.md");
const NOW: &str = "2026-10-23T22:04:00Z";

/// The binary on the fixture at the golden instant, with stdin from `/dev/null`.
fn teamzones() -> Command {
    let mut command = std::process::Command::new(assert_cmd::cargo::cargo_bin("teamzones"));
    command
        .env("TEAMZONES_CONFIG", FIXTURE)
        .args(["--now", NOW])
        .stdin(std::process::Stdio::null());
    Command::from_std(command)
}

/// The body rows of the first table after `heading`, as trimmed cells.
fn table_rows(spec: &str, heading: &str) -> Vec<Vec<String>> {
    spec.split_once(heading)
        .unwrap_or_else(|| panic!("no {heading:?} in the spec"))
        .1
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

/// Runs one CLI golden row: stdout and exit 0, or the stderr text and exit 2.
fn cli_row(index: usize) {
    let rows = table_rows(CLI_SPEC, "## CLI golden table");
    let [command, output] = rows[index].as_slice() else {
        panic!("cli row {index} needs two cells")
    };
    let command = backticked(command)[0];
    let expected = backticked(output)[0];
    let assert = teamzones()
        .args(command.split(' '))
        .arg("--no-copy")
        .assert();
    if output.starts_with("(stderr)") {
        assert.code(2).stdout("").stderr(format!("{expected}\n"));
    } else {
        assert.success().stdout(format!("{expected}\n")).stderr("");
    }
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

#[test]
fn the_cli_golden_table_has_one_named_test_per_row() {
    assert_eq!(table_rows(CLI_SPEC, "## CLI golden table").len(), 14);
}

/// The `$ teamzones status ...` blocks in the CLI spec, as (command words, expected stdout).
fn status_blocks() -> Vec<(String, String)> {
    let fence = CLI_SPEC
        .split_once("```\n$ teamzones status")
        .unwrap()
        .1
        .split_once("```")
        .unwrap()
        .0;
    format!("$ teamzones status{fence}")
        .split("$ teamzones ")
        .skip(1)
        .map(|block| {
            let (command, lines) = block.split_once('\n').unwrap();
            (command.to_owned(), lines.to_owned())
        })
        .collect()
}

fn status_block(index: usize) {
    let (command, expected) = &status_blocks()[index];
    let (words, now) = command.split_once(" --now ").unwrap();
    assert_eq!(now, NOW, "status blocks are at the golden instant");
    teamzones()
        .args(words.split(' '))
        .assert()
        .success()
        .stdout(expected.clone())
        .stderr("");
}

golden! { status_block:
    status_regions = 0;
    status_team = 1;
}

#[test]
fn the_cli_spec_has_two_status_blocks() {
    assert_eq!(status_blocks().len(), 2);
}
