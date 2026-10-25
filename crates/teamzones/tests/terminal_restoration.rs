//! Runs the real binary on a real pseudo-terminal and reads back what it wrote, proving it
//! releases mouse capture and the alternate screen on quit. A `TestBackend` never sees these
//! escape sequences, and `rustix` opens the pty without any unsafe code.

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

#[test]
fn quitting_releases_mouse_capture_and_then_leaves_the_alternate_screen() {
    let (master, slave_path) = open_pty();
    // Held until the child is reaped, so the master reports end of output only after that.
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
    let fixture = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../teamzones-core/tests/fixtures/sample.toml"
    );
    let mut writer = master.try_clone().expect("clone the master");
    let rx = read_chunks(master);
    let mut child = Command::new(env!("CARGO_BIN_EXE_teamzones"))
        .args(["--now", "2026-10-23T22:04:00Z"])
        .env("TEAMZONES_CONFIG", fixture)
        .env("NO_COLOR", "1")
        .stdin(Stdio::from(open_slave(&slave_path)))
        .stdout(Stdio::from(open_slave(&slave_path)))
        .stderr(Stdio::from(open_slave(&slave_path)))
        .spawn()
        .expect("spawn teamzones on the pty");

    let capture = ansi(crossterm::event::EnableMouseCapture);
    let mut output = Vec::new();
    read_until(&rx, &mut output, "start", |out, _| {
        contains(out, &capture) && contains(out, "teamzones")
    });
    writer.write_all(b"q").expect("send q");
    wait_for_exit(&mut child);
    drop(held);
    read_until(&rx, &mut output, "quit", |_, end| {
        matches!(end, Some(End::Eof | End::EofViaEio))
    });

    let output = String::from_utf8_lossy(&output);
    let release = format!(
        "{}{}",
        ansi(crossterm::event::DisableMouseCapture),
        ansi(crossterm::terminal::LeaveAlternateScreen)
    );
    assert!(
        output.ends_with(&release),
        "the output ends {:?}",
        &output[output.len().saturating_sub(120)..]
    );
}

#[test]
fn a_closed_slave_reads_as_the_end_on_either_platform() {
    assert!(matches!(classify_pty_read(Ok(0)), Err(End::Eof)));
    let eio = std::io::Error::from_raw_os_error(rustix::io::Errno::IO.raw_os_error());
    assert!(matches!(classify_pty_read(Err(eio)), Err(End::EofViaEio)));
    assert!(matches!(classify_pty_read(Ok(3)), Ok(3)));
}
