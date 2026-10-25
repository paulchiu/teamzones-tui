//! The `teamzones` binary: the TUI and the headless CLI over `teamzones-core`.

mod cli;
mod clipboard;
mod paths;
mod tui;
mod write;

use std::io::IsTerminal;
use std::process::ExitCode;

use clap::Parser;

fn main() -> ExitCode {
    let argv = cli::hoist_flags(
        std::env::args_os()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect(),
    );
    let parsed = cli::Cli::parse_from(argv);
    let config_path = match paths::config_path() {
        Ok(path) => path,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(2);
        }
    };
    let mut clipboard =
        clipboard::for_os(std::env::consts::OS, clipboard::System, clipboard::Terminal);
    if parsed.opens_tui() && std::io::stdin().is_terminal() && std::io::stdout().is_terminal() {
        return open_tui(&parsed, &config_path, &mut clipboard);
    }
    let mut stdout = std::io::stdout();
    let mut stderr = std::io::stderr();
    let mut io = cli::Io {
        out: &mut stdout,
        err: &mut stderr,
        stdout_is_tty: std::io::stdout().is_terminal(),
        system_zone: paths::system_zone(),
        config_path,
        clipboard: &mut clipboard,
    };
    let code = match cli::run(parsed, &mut io) {
        Ok(()) => 0,
        Err(cli::Failure::HelpShown) => 2,
        Err(failure) => {
            eprintln!("{failure}");
            failure.code()
        }
    };
    ExitCode::from(code)
}

/// Loads config and runs the TUI, exiting 2 on a config error and 1 on a terminal error.
fn open_tui(
    parsed: &cli::Cli,
    config_path: &std::path::Path,
    clipboard: &mut dyn clipboard::Clipboard,
) -> ExitCode {
    let system_zone = paths::system_zone();
    let loaded = paths::read(config_path).and_then(|text| {
        paths::load_text(config_path, text.as_deref(), system_zone).map(|config| (config, text))
    });
    let (config, text) = match loaded {
        Ok(loaded) => loaded,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(2);
        }
    };
    let file = tui::File {
        path: config_path,
        text,
        system_zone,
    };
    match tui::run(
        config,
        file,
        parsed.clock(),
        clipboard,
        tui::TerminalEnv::from_env(),
    ) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("terminal error: {error}");
            ExitCode::from(1)
        }
    }
}
