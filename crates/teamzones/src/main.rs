//! The `teamzones` binary: the TUI and the headless CLI over `teamzones-core`.

use clap::Parser;

/// Who's available and where it's busy, on one time axis.
#[derive(Parser)]
#[command(name = "teamzones", version, about)]
struct Cli {}

fn main() {
    Cli::parse();
}
