//! Runs the real binary on a real pseudo-terminal and reads back what it wrote, proving it
//! releases mouse capture, raw mode and the alternate screen on quit and around the editor
//! handoff. A `TestBackend` never sees these escape sequences, and `rustix` opens the pty
//! without any unsafe code.

use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use rustix::pty::{OpenptFlags, grantpt, openpt, ptsname, unlockpt};
use rustix::termios::{Winsize, tcsetwinsize};

/// How long any one wait in this file may take before the test fails rather than hangs.
const BACKSTOP: Duration = Duration::from_secs(20);

/// Why a pty master read stopped: macOS reports a closed slave as `Ok(0)` and Linux as `EIO`,
/// and both mean the end of output.
#[derive(Debug)]
enum End {
    Eof,
    EofViaEio,
    Err(std::io::Error),
}

/// One read from a pty master: more bytes, or the end it stopped at.
fn classify_pty_read(result: std::io::Result<usize>) -> Result<usize, End> {
    match result {
        Ok(0) => Err(End::Eof),
        Ok(n) => Ok(n),
        Err(error) if error.raw_os_error() == Some(rustix::io::Errno::IO.raw_os_error()) => {
            Err(End::EofViaEio)
        }
        Err(error) => Err(End::Err(error)),
    }
}

/// The escape sequence crossterm writes for `command`, so expectations follow its encoding.
fn ansi(command: impl crossterm::Command) -> String {
    let mut out = String::new();
    command
        .write_ansi(&mut out)
        .expect("encode an ansi command");
    out
}

/// A fresh pty: the master, and the slave's path.
fn open_pty() -> (File, PathBuf) {
    let master = openpt(OpenptFlags::RDWR | OpenptFlags::NOCTTY).expect("open a pty master");
    grantpt(&master).expect("grant the pty");
    unlockpt(&master).expect("unlock the pty");
    let path = ptsname(&master, Vec::new()).expect("name the pty slave");
    let path = PathBuf::from(path.to_string_lossy().into_owned());
    (File::from(master), path)
}

fn open_slave(path: &PathBuf) -> File {
    OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .expect("open the pty slave")
}

/// Starts a thread that forwards every chunk read from `master`, then the end it stopped at.
fn read_chunks(mut master: File) -> mpsc::Receiver<Result<Vec<u8>, End>> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        loop {
            let mut chunk = [0u8; 4096];
            match classify_pty_read(master.read(&mut chunk)) {
                Ok(n) => {
                    if tx.send(Ok(chunk[..n].to_vec())).is_err() {
                        return;
                    }
                }
                Err(end) => {
                    let _ = tx.send(Err(end));
                    return;
                }
            }
        }
    });
    rx
}

/// Collects output until `done` holds for it, failing the test past the backstop.
fn read_until(
    rx: &mpsc::Receiver<Result<Vec<u8>, End>>,
    output: &mut Vec<u8>,
    what: &str,
    done: impl Fn(&[u8], Option<&End>) -> bool,
) {
    let deadline = Instant::now() + BACKSTOP;
    while !done(output, None) {
        let left = deadline.saturating_duration_since(Instant::now());
        match rx.recv_timeout(left) {
            Ok(Ok(chunk)) => output.extend_from_slice(&chunk),
            Ok(Err(End::Err(error))) => panic!(
                "{what}: the pty read failed ({error}) after {:?}",
                String::from_utf8_lossy(output)
            ),
            Ok(Err(end)) => {
                assert!(
                    done(output, Some(&end)),
                    "{what}: output stopped at {end:?}: {:?}",
                    String::from_utf8_lossy(output)
                );
                return;
            }
            Err(_) => panic!(
                "{what}: timed out with {:?}",
                String::from_utf8_lossy(output)
            ),
        }
    }
}

fn wait_for_exit(child: &mut Child) {
    let deadline = Instant::now() + BACKSTOP;
    while child.try_wait().expect("poll the child").is_none() {
        if Instant::now() > deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("teamzones never exited after q");
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn contains(haystack: &[u8], needle: &str) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle.as_bytes())
}

/// The binary running on a 120 by 36 pty: the master to type into, its output, and the child.
struct Session {
    writer: File,
    rx: mpsc::Receiver<Result<Vec<u8>, End>>,
    child: Child,
    /// Held until the child is reaped, so the master reports end of output only after that.
    held: File,
    output: Vec<u8>,
}

impl Session {
    /// Starts teamzones on the fixture at the golden instant, with `env` set and `VISUAL` unset,
    /// once it has captured the mouse and drawn.
    fn start(config: &str, env: &[(&str, &str)]) -> Self {
        let (master, slave_path) = open_pty();
        let held = open_slave(&slave_path);
        tcsetwinsize(
            &held,
            Winsize {
                ws_row: 36,
                ws_col: 120,
                ws_xpixel: 0,
                ws_ypixel: 0,
            },
        )
        .expect("size the pty");
        let writer = master.try_clone().expect("clone the master");
        let rx = read_chunks(master);
        let child = Command::new(env!("CARGO_BIN_EXE_teamzones"))
            .args(["--now", "2026-10-23T22:04:00Z"])
            .env("TEAMZONES_CONFIG", config)
            .env("NO_COLOR", "1")
            .env_remove("VISUAL")
            .envs(env.iter().copied())
            .stdin(Stdio::from(open_slave(&slave_path)))
            .stdout(Stdio::from(open_slave(&slave_path)))
            .stderr(Stdio::from(open_slave(&slave_path)))
            .spawn()
            .expect("spawn teamzones on the pty");
        let mut session = Self {
            writer,
            rx,
            child,
            held,
            output: Vec::new(),
        };
        let capture = ansi(crossterm::event::EnableMouseCapture);
        session.read_until("start", |out| {
            contains(out, &capture) && contains(out, "teamzones")
        });
        session
    }

    fn read_until(&mut self, what: &str, done: impl Fn(&[u8]) -> bool) {
        read_until(&self.rx, &mut self.output, what, |out, _| done(out));
    }

    fn send(&mut self, bytes: &[u8]) {
        self.writer.write_all(bytes).expect("type into the pty");
    }

    /// Sends `q`, waits for the child to exit and reads to the end; returns all of the output.
    fn quit(mut self) -> String {
        self.send(b"q");
        wait_for_exit(&mut self.child);
        drop(self.held);
        read_until(&self.rx, &mut self.output, "quit", |_, end| {
            matches!(end, Some(End::Eof | End::EofViaEio))
        });
        String::from_utf8_lossy(&self.output).into_owned()
    }
}

const FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../teamzones-core/tests/fixtures/sample.toml"
);

/// Mouse capture off and then the alternate screen left, as crossterm encodes them.
fn release() -> String {
    format!(
        "{}{}",
        ansi(crossterm::event::DisableMouseCapture),
        ansi(crossterm::terminal::LeaveAlternateScreen)
    )
}

#[test]
fn quitting_releases_mouse_capture_and_then_leaves_the_alternate_screen() {
    let output = Session::start(FIXTURE, &[]).quit();
    assert!(
        output.ends_with(&release()),
        "the output ends {:?}",
        &output[output.len().saturating_sub(120)..]
    );
}

/// What the stand-in editor prints: whether the terminal it was handed is in raw mode.
const COOKED: &str = "editor saw a cooked terminal";
const RAW: &str = "editor saw a raw terminal";

/// A stand-in `$EDITOR` that reports the terminal mode it was handed and exits.
fn reporting_editor(dir: &std::path::Path) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let path = dir.join("editor.sh");
    let script = format!(
        "#!/bin/sh\ncase \"$(stty -a)\" in *-icanon*) printf '{RAW}' ;; *) printf '{COOKED}' ;; esac\n"
    );
    std::fs::write(&path, script).expect("write the stand-in editor");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))
        .expect("make the stand-in editor executable");
    path
}

#[test]
fn the_editor_gets_a_released_terminal_and_the_tui_takes_it_back_after() {
    let dir = std::env::temp_dir().join(format!("teamzones-pty-editor-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("make a scratch dir");
    let config = dir.join("config.toml");
    std::fs::copy(FIXTURE, &config).expect("copy the fixture");
    let editor = reporting_editor(&dir);
    let mut session = Session::start(
        &config.display().to_string(),
        &[("EDITOR", &editor.display().to_string())],
    );
    let (enter, capture) = (
        ansi(crossterm::terminal::EnterAlternateScreen),
        ansi(crossterm::event::EnableMouseCapture),
    );
    session.send(b"E");
    session.read_until("editor", |out| {
        let out = String::from_utf8_lossy(out);
        out.split_once(COOKED)
            .or_else(|| out.split_once(RAW))
            .is_some_and(|(_, after)| after.contains(&enter) && after.contains(&capture))
    });
    // Past the 20 ms drain after the editor exits, so the q is read as a key.
    std::thread::sleep(Duration::from_millis(300));
    let output = session.quit();
    let (before, after) = output
        .split_once(COOKED)
        .unwrap_or_else(|| panic!("the editor never saw a cooked terminal: {output:?}"));
    assert!(
        before.ends_with(&format!("{}{}", release(), ansi(crossterm::cursor::Show))),
        "before the editor ran the output ends {:?}",
        &before[before.len().saturating_sub(120)..]
    );
    let entered = after
        .find(&enter)
        .expect("the alternate screen is entered again");
    let captured = after.find(&capture).expect("the mouse is captured again");
    assert!(entered < captured, "{after:?}");
    assert_eq!(
        std::fs::read_to_string(&config).expect("read the config back"),
        std::fs::read_to_string(FIXTURE).expect("read the fixture"),
        "the handoff wrote nothing"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_closed_slave_reads_as_the_end_on_either_platform() {
    assert!(matches!(classify_pty_read(Ok(0)), Err(End::Eof)));
    let eio = std::io::Error::from_raw_os_error(rustix::io::Errno::IO.raw_os_error());
    assert!(matches!(classify_pty_read(Err(eio)), Err(End::EofViaEio)));
    assert!(matches!(classify_pty_read(Ok(3)), Ok(3)));
}
