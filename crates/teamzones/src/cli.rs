//! The headless commands: argument parsing, then one core call per line printed.

use std::io::Write;

use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use clap::{Args, Parser, Subcommand};
use teamzones_core::clock::Clock;
use teamzones_core::config::{Config, Group, Roster, Rule};
use teamzones_core::copy::{Clause, Variant, say_line};
use teamzones_core::expr::{ExprError, parse_expr};
use teamzones_core::fit::resolve;
use teamzones_core::question::{Marks, Question};
use teamzones_core::status::status_lines;
use teamzones_core::time::Moment;
use thiserror::Error;

use crate::{paths, write};

/// Who's available and where it's busy, on one time axis.
#[derive(Debug, Parser)]
#[command(name = "teamzones", version, about)]
pub struct Cli {
    /// Fix "now" to an RFC 3339 instant, for tests and demos.
    #[arg(long, global = true, hide = true, value_parser = parse_now)]
    now: Option<DateTime<Utc>>,
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Print the say line for a who-expression and a time, and copy it.
    Say(SayArgs),
    /// Print where each row of a who-expression, or one place, is now and what comes next.
    Status {
        /// The who-expression or `@place`; the first group by default.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true, value_name = "EXPR | @place")]
        words: Vec<String>,
    },
    /// Print one line per group: its members and the question it asks.
    Groups,
    /// Find, create, edit or validate the config file.
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
}

#[derive(Debug, Subcommand)]
enum ConfigAction {
    /// Print the config file's path.
    Path,
    /// Write the commented template; fails if a file exists.
    Init,
    /// Open the file in $VISUAL, then $EDITOR, then vi, seeding the template first.
    Edit,
    /// Validate the file.
    Check,
}

#[derive(Debug, Args)]
struct SayArgs {
    /// List the counted places' zones instead of their states (`c z`).
    #[arg(long)]
    zones: bool,
    /// Name who is in each zone (`c w`).
    #[arg(long)]
    names: bool,
    /// Prefix the date (`c d`).
    #[arg(long)]
    date: bool,
    /// Drop the places' clause (`c s`).
    #[arg(long)]
    no_clause: bool,
    /// Print without copying.
    #[arg(long)]
    no_copy: bool,
    /// The who-expression, then the time, such as `team +@uk 5pm mon` or `deploy next`.
    #[arg(
        trailing_var_arg = true,
        allow_hyphen_values = true,
        value_name = "EXPR TIME"
    )]
    words: Vec<String>,
}

fn parse_now(text: &str) -> Result<DateTime<Utc>, chrono::ParseError> {
    DateTime::parse_from_rfc3339(text).map(|instant| instant.with_timezone(&Utc))
}

/// The long flags `say` and `status` accept after their words; `--now` takes a value.
const HOISTED: &[&str] = &[
    "--zones",
    "--names",
    "--date",
    "--no-clause",
    "--no-copy",
    "--help",
];

/// Moves known long flags that follow `say` or `status` to just after it, so clap reads them as
/// flags rather than as words of the expression.
pub fn hoist_flags(argv: Vec<String>) -> Vec<String> {
    let Some(at) = argv
        .iter()
        .position(|arg| arg == "say" || arg == "status")
    else {
        return argv;
    };
    let (head, tail) = argv.split_at(at + 1);
    let mut flags = Vec::new();
    let mut words = Vec::new();
    let mut rest = tail.iter();
    while let Some(arg) = rest.next() {
        if HOISTED.contains(&arg.as_str()) || arg.starts_with("--now=") {
            flags.push(arg.clone());
        } else if arg == "--now" {
            flags.push(arg.clone());
            flags.extend(rest.next().cloned());
        } else {
            words.push(arg.clone());
        }
    }
    head.iter().cloned().chain(flags).chain(words).collect()
}

/// Where a command's output goes, and the facts about the process it depends on.
pub struct Io<'a> {
    pub out: &'a mut dyn Write,
    pub err: &'a mut dyn Write,
    pub stdout_is_tty: bool,
    pub system_zone: Tz,
}

/// Why a command stopped; the Display is what goes to stderr.
#[derive(Debug, Error)]
pub enum Failure {
    /// No fit, or nothing to say: exit 1.
    #[error("{0}")]
    Nothing(String),
    /// A usage or config error: exit 2.
    #[error("{0}")]
    Usage(String),
}

impl Failure {
    /// The process exit code for this failure.
    pub fn code(&self) -> u8 {
        match self {
            Self::Nothing(_) => 1,
            Self::Usage(_) => 2,
        }
    }
}

fn usage(error: impl ToString) -> Failure {
    Failure::Usage(error.to_string())
}

fn output(error: std::io::Error) -> Failure {
    Failure::Usage(format!("cannot write output: {error}"))
}

/// Runs one parsed command line.
///
/// # Errors
///
/// A [`Failure`] carrying the exit code and the text for stderr.
pub fn run(cli: Cli, io: &mut Io) -> Result<(), Failure> {
    let now = cli.now.map_or(Clock::System, Clock::Fixed).now();
    match cli.command {
        Some(Command::Say(args)) => say(&args, now, io),
        Some(Command::Status { words: tokens }) => status(&tokens, now, io),
        Some(Command::Groups) => groups(io),
        Some(Command::Config { action }) => config(&action, io),
        None => Err(usage(
            "teamzones needs a command for now: try teamzones --help",
        )),
    }
}

fn load(io: &Io) -> Result<Config, Failure> {
    let path = paths::config_path().map_err(usage)?;
    paths::load(&path, io.system_zone).map_err(usage)
}

fn words(args: &[String]) -> Vec<&str> {
    args.iter().map(String::as_str).collect()
}

fn expr_failure(error: ExprError) -> Failure {
    match error {
        ExprError::NoGroups => Failure::Nothing(error.to_string()),
        _ => usage(error),
    }
}

fn say(args: &SayArgs, now: DateTime<Utc>, io: &mut Io) -> Result<(), Failure> {
    let SayArgs {
        zones,
        names,
        date,
        no_clause,
        no_copy: _,
        words: tokens,
    } = args;
    let config = load(io)?;
    let roster = &config.roster;
    let expr = parse_expr(&words(tokens), roster, now).map_err(expr_failure)?;
    let dated = expr.moment == Moment::Next;
    let when = resolve(expr.moment, roster, &expr.marks, now)
        .map_err(|no_fit| Failure::Nothing(no_fit.to_string()))?;
    let clause = match (zones, no_clause) {
        (true, _) => Clause::Zones,
        (false, true) => Clause::Drop,
        (false, false) => Clause::States,
    };
    let variant = Variant {
        date: *date || dated,
        names: *names,
        clause,
    };
    let line = say_line(roster, &expr.marks, when, variant);
    writeln!(io.out, "{line}").map_err(output)
}

/// The marks `status` reports on: one `@place` alone, or a who-expression at now.
fn status_marks(tokens: &[&str], config: &Config, now: DateTime<Utc>) -> Result<Marks, Failure> {
    let roster = &config.roster;
    if let [token] = tokens
        && let Some(name) = token.strip_prefix('@')
    {
        let place = roster
            .place_named(name)
            .ok_or_else(|| usage(format!("@{name}: no place has that name")))?;
        return Ok(Marks {
            places: [place].into(),
            ..Marks::default()
        });
    }
    let expr = parse_expr(tokens, roster, now).map_err(expr_failure)?;
    match expr.moment {
        Moment::Now => Ok(expr.marks),
        Moment::Next | Moment::Fixed(_) => Err(usage(
            "status shows now, so it takes a who-expression but no time",
        )),
    }
}

fn status(tokens: &[String], now: DateTime<Utc>, io: &mut Io) -> Result<(), Failure> {
    let config = load(io)?;
    let marks = status_marks(&words(tokens), &config, now)?;
    let lines = status_lines(&config.roster, &marks, now);
    if lines.is_empty() {
        return Err(Failure::Nothing(
            "nothing to show: the expression has no rows besides home".to_owned(),
        ));
    }
    lines
        .iter()
        .try_for_each(|line| writeln!(io.out, "{line}"))
        .map_err(output)
}

/// The word for what a group's places count as.
fn rule_word(rule: Rule) -> &'static str {
    match rule {
        Rule::OffPeak | Rule::Context => "off-peak",
        Rule::Quiet => "quiet",
    }
}

/// `1 team: Priya, Sam, Thandi, Johan  asks who`; groups past the ninth get no number.
fn group_line(roster: &Roster, index: usize, group: &Group) -> String {
    let number = if index < 9 {
        (index + 1).to_string()
    } else {
        " ".to_owned()
    };
    let aliases = if group.aliases.is_empty() {
        String::new()
    } else {
        format!(" ({})", group.aliases.join(", "))
    };
    let members: Vec<String> = group
        .people
        .iter()
        .map(|id| roster.person(*id).name.clone())
        .chain(
            group
                .places
                .iter()
                .map(|id| format!("@{}", roster.place(*id).name)),
        )
        .collect();
    let asks = match Marks::of_group(group).question() {
        Some(Question::Who) => "who".to_owned(),
        Some(Question::Where) => rule_word(group.rule).to_owned(),
        Some(Question::Both) => format!("who and {}", rule_word(group.rule)),
        None => "nothing".to_owned(),
    };
    format!(
        "{number} {}{aliases}: {}  asks {asks}",
        group.name,
        members.join(", ")
    )
}

fn groups(io: &mut Io) -> Result<(), Failure> {
    let config = load(io)?;
    let roster = &config.roster;
    if roster.groups.is_empty() {
        return Err(Failure::Nothing("there are no groups in config".to_owned()));
    }
    roster
        .groups
        .iter()
        .enumerate()
        .try_for_each(|(index, group)| writeln!(io.out, "{}", group_line(roster, index, group)))
        .map_err(output)
}

/// The editor program and its arguments for `path`: `$VISUAL`, then `$EDITOR`, then `vi`, split
/// on whitespace.
fn editor_command(
    visual: Option<&str>,
    editor: Option<&str>,
    path: &std::path::Path,
) -> (String, Vec<String>) {
    let mut words = [visual, editor]
        .into_iter()
        .flatten()
        .find(|command| !command.trim().is_empty())
        .unwrap_or("vi")
        .split_whitespace()
        .map(str::to_owned);
    let program = words.next().unwrap_or_else(|| "vi".to_owned());
    let args = words
        .chain(std::iter::once(path.display().to_string()))
        .collect();
    (program, args)
}

fn config(action: &ConfigAction, io: &mut Io) -> Result<(), Failure> {
    let path = paths::config_path().map_err(usage)?;
    let shown = path.display();
    match action {
        ConfigAction::Path => writeln!(io.out, "{shown}").map_err(output),
        ConfigAction::Init => write::create_new(&path, write::TEMPLATE).map_err(|error| {
            if error.kind() == std::io::ErrorKind::AlreadyExists {
                usage(format!("{shown} already exists"))
            } else {
                usage(format!("{shown}: {error}"))
            }
        }),
        ConfigAction::Edit => {
            if paths::read(&path).map_err(usage)?.is_none() {
                write::create_new(&path, write::TEMPLATE)
                    .map_err(|error| usage(format!("{shown}: {error}")))?;
            }
            let visual = std::env::var("VISUAL").ok();
            let editor = std::env::var("EDITOR").ok();
            let (program, args) = editor_command(visual.as_deref(), editor.as_deref(), &path);
            let status = std::process::Command::new(&program)
                .args(&args)
                .status()
                .map_err(|error| usage(format!("cannot run {program}: {error}")))?;
            if status.success() {
                Ok(())
            } else {
                Err(usage(format!("{program} exited with {status}")))
            }
        }
        ConfigAction::Check => {
            let exists = paths::read(&path).map_err(usage)?.is_some();
            paths::load(&path, io.system_zone).map_err(usage)?;
            let verdict = if exists {
                "ok"
            } else {
                "no file, so the defaults apply"
            };
            writeln!(io.out, "{shown}: {verdict}").map_err(output)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn strings(words: &[&str]) -> Vec<String> {
        words.iter().map(|word| (*word).to_owned()).collect()
    }

    #[test]
    fn hoisting_moves_trailing_flags_and_now_ahead_of_the_words() {
        let argv = strings(&["teamzones", "say", "team", "-@au", "3pm", "--no-copy", "--now", "X"]);
        assert_eq!(
            hoist_flags(argv),
            strings(&["teamzones", "say", "--no-copy", "--now", "X", "team", "-@au", "3pm"])
        );
    }

    #[test]
    fn visual_wins_over_editor_and_its_arguments_are_kept() {
        let path = Path::new("/c.toml");
        assert_eq!(
            editor_command(Some("code -w"), Some("nano"), path),
            ("code".to_owned(), strings(&["-w", "/c.toml"]))
        );
        assert_eq!(
            editor_command(Some(" "), Some("nano"), path),
            ("nano".to_owned(), strings(&["/c.toml"]))
        );
        assert_eq!(
            editor_command(None, None, path),
            ("vi".to_owned(), strings(&["/c.toml"]))
        );
    }
}
