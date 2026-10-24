//! The headless commands: argument parsing, then one core call per line printed.

use std::io::Write;

use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use clap::{Args, Parser, Subcommand};
use teamzones_core::clock::Clock;
use teamzones_core::config::Config;
use teamzones_core::copy::{Clause, Variant, say_line};
use teamzones_core::expr::{ExprError, parse_expr};
use teamzones_core::fit::resolve;
use teamzones_core::time::Moment;
use thiserror::Error;

use crate::paths;

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
