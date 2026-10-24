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

/// The two example lines the `groups` row of the CLI spec gives.
fn groups_examples() -> Vec<String> {
    let rows = table_rows(CLI_SPEC, "# Headless CLI");
    let row = rows
        .iter()
        .find(|row| row[0] == "`teamzones groups`")
        .unwrap();
    backticked(&row[1])
        .into_iter()
        .map(str::to_owned)
        .collect()
}

#[test]
fn groups_prints_one_line_per_group_in_the_form_the_spec_shows() {
    let output = teamzones().arg("groups").assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    let lines: Vec<&str> = stdout.lines().collect();
    let examples = groups_examples();
    assert_eq!(lines.len(), 4, "{stdout}");
    assert_eq!(lines[0], examples[0]);
    assert_eq!(lines[2], examples[1]);
}

/// A fresh, empty directory for one test, under the system temp dir.
fn scratch(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir()
        .join("teamzones-cli-tests")
        .join(format!("{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// The binary on the config at `path`, at the golden instant, with stdin from `/dev/null`.
fn teamzones_on(path: &std::path::Path) -> Command {
    let mut command = teamzones();
    command.env("TEAMZONES_CONFIG", path);
    command
}

#[test]
fn config_path_prints_the_resolved_path() {
    let path = scratch("config-path").join("config.toml");
    teamzones_on(&path)
        .args(["config", "path"])
        .assert()
        .success()
        .stdout(format!("{}\n", path.display()));
}

#[test]
fn config_init_writes_a_commented_template_that_check_accepts() {
    let path = scratch("config-init").join("nested").join("config.toml");
    teamzones_on(&path).args(["config", "init"]).assert().success();
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.starts_with("## teamzones configuration"), "{text}");
    assert!(
        text.lines().all(|line| line.is_empty() || line.starts_with('#')),
        "every line is a comment:\n{text}"
    );
    teamzones_on(&path).args(["config", "check"]).assert().success();
}

#[test]
fn config_init_exits_2_and_leaves_an_existing_file_alone() {
    let path = scratch("config-init-exists").join("config.toml");
    std::fs::write(&path, "# mine\n").unwrap();
    teamzones_on(&path)
        .args(["config", "init"])
        .assert()
        .code(2)
        .stderr(format!("{} already exists\n", path.display()));
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "# mine\n");
}

#[test]
fn config_check_exits_2_naming_the_path_and_line_of_a_broken_file() {
    let path = scratch("config-check-broken").join("config.toml");
    std::fs::write(&path, "[home]\nname = \"Kim\"\ncity = \"Melbourne\"\ncolour = 1\n").unwrap();
    teamzones_on(&path)
        .args(["config", "check"])
        .assert()
        .code(2)
        .stderr(format!(
            "{}: line 4: unknown key \"colour\"\n",
            path.display()
        ));
}

#[test]
fn config_edit_seeds_the_template_before_opening_the_editor() {
    let path = scratch("config-edit").join("config.toml");
    teamzones_on(&path)
        .env("VISUAL", "true")
        .args(["config", "edit"])
        .assert()
        .success();
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.starts_with("## teamzones configuration"), "{text}");
}

/// A copy of the fixture in a fresh directory, so a test can write to it.
fn fixture_copy(name: &str) -> (std::path::PathBuf, String) {
    let path = scratch(name).join("config.toml");
    let text = std::fs::read_to_string(FIXTURE).unwrap();
    std::fs::write(&path, &text).unwrap();
    (path, text)
}

const TRAVEL: &str = "\n[travel]\nhere = \"London\"\nuntil = 2026-11-01\n";

#[test]
fn here_with_no_arguments_reports_home_when_not_travelling() {
    teamzones()
        .arg("here")
        .assert()
        .success()
        .stdout("home Melbourne\n");
}

#[test]
fn here_writes_travel_after_the_untouched_file_and_clear_removes_it() {
    let (path, original) = fixture_copy("here-set-clear");
    teamzones_on(&path)
        .args(["here", "london", "--until", "2026-11-01"])
        .assert()
        .success()
        .stdout("here London until 2026-11-01\n");
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        format!("{original}{TRAVEL}")
    );
    teamzones_on(&path)
        .arg("here")
        .assert()
        .success()
        .stdout("here London until 2026-11-01\n");
    teamzones_on(&path)
        .args(["here", "--clear"])
        .assert()
        .success()
        .stdout("home Melbourne\n");
    assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
}

#[test]
fn here_reports_home_once_until_has_passed_in_the_travel_city() {
    let (path, original) = fixture_copy("here-expired");
    std::fs::write(&path, format!("{original}{TRAVEL}")).unwrap();
    teamzones_on(&path)
        .args(["here", "--now", "2026-11-02T00:30:00Z"])
        .assert()
        .success()
        .stdout("home Melbourne\n");
}

#[test]
fn here_never_writes_over_a_file_that_does_not_parse() {
    let path = scratch("here-broken").join("config.toml");
    let broken = "[home]\nname = \"Kim\"\ncolour = 1\n";
    std::fs::write(&path, broken).unwrap();
    teamzones_on(&path)
        .args(["here", "london", "--until", "2026-11-01"])
        .assert()
        .code(2)
        .stderr(format!("{}: line 3: unknown key \"colour\"\n", path.display()));
    assert_eq!(std::fs::read_to_string(&path).unwrap(), broken);
}

#[test]
fn here_needs_until_when_setting() {
    let (path, original) = fixture_copy("here-no-until");
    teamzones_on(&path).args(["here", "london"]).assert().code(2);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
}

#[test]
fn exit_0_when_there_is_a_line_to_say() {
    teamzones()
        .args(["say", "team", "3pm", "--no-copy"])
        .assert()
        .code(0);
}

#[test]
fn exit_1_with_the_best_slot_on_stderr_when_nothing_fits_in_7_days() {
    let (path, original) = fixture_copy("exit-1-no-fit");
    let quiet = original.replace(
        "places  = [\"AU\", \"UK\", \"US\"]",
        "places  = [\"AU\", \"UK\", \"US\"]\nwhere   = \"quiet\"",
    );
    assert_ne!(quiet, original, "the regions group gained where = quiet");
    std::fs::write(&path, quiet).unwrap();
    let assert = teamzones_on(&path)
        .args(["say", "regions", "next", "--no-copy"])
        .assert()
        .code(1)
        .stdout("");
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
    assert!(
        stderr.starts_with("no full slot in 7 days, best 2/3 at "),
        "{stderr}"
    );
}

#[test]
fn exit_2_on_a_usage_error() {
    teamzones()
        .args(["say", "uk", "now", "--no-copy"])
        .assert()
        .code(2);
}

#[test]
fn exit_2_on_a_config_error() {
    let path = scratch("exit-2-config").join("config.toml");
    std::fs::write(&path, "[home]\n").unwrap();
    teamzones_on(&path)
        .args(["say", "3pm", "--no-copy"])
        .assert()
        .code(2);
}

#[test]
fn with_no_command_and_no_tty_it_prints_help_and_exits_2() {
    let assert = teamzones().assert().code(2);
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(stdout.contains("Usage: teamzones"), "{stdout}");
    assert!(stdout.contains("say"), "{stdout}");
}
