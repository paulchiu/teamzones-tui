//! Copying the say line: the system clipboard, then OSC 52 to the terminal (on Linux, both).

/// Somewhere a line can be copied to.
pub trait Clipboard {
    /// Copies `text`.
    ///
    /// # Errors
    ///
    /// Why the copy failed, as text for stderr.
    fn copy(&mut self, text: &str) -> Result<(), String>;
}

/// The system clipboard, through arboard.
pub struct System;

impl Clipboard for System {
    fn copy(&mut self, text: &str) -> Result<(), String> {
        arboard::Clipboard::new()
            .and_then(|mut clipboard| clipboard.set_text(text))
            .map_err(|error| error.to_string())
    }
}

/// The terminal's clipboard, by writing an OSC 52 sequence to `/dev/tty`; it reaches the local
/// machine over SSH.
pub struct Terminal;

impl Clipboard for Terminal {
    fn copy(&mut self, text: &str) -> Result<(), String> {
        use std::io::Write;
        std::fs::OpenOptions::new()
            .write(true)
            .open("/dev/tty")
            .and_then(|mut tty| tty.write_all(osc52(text).as_bytes()))
            .map_err(|error| format!("/dev/tty: {error}"))
    }
}

/// Tries `primary`, then `fallback`, failing only when both do.
pub struct Fallback<A, B>(pub A, pub B);

impl<A: Clipboard, B: Clipboard> Clipboard for Fallback<A, B> {
    fn copy(&mut self, text: &str) -> Result<(), String> {
        let Self(primary, fallback) = self;
        primary.copy(text).or_else(|first| {
            fallback
                .copy(text)
                .map_err(|second| format!("{first}; {second}"))
        })
    }
}

/// Gives `text` to both, failing only when both do.
pub struct Both<A, B>(pub A, pub B);

impl<A: Clipboard, B: Clipboard> Clipboard for Both<A, B> {
    fn copy(&mut self, text: &str) -> Result<(), String> {
        let Self(first, second) = self;
        match (first.copy(text), second.copy(text)) {
            (Err(first), Err(second)) => Err(format!("{first}; {second}")),
            (Ok(()), _) | (_, Ok(())) => Ok(()),
        }
    }
}

/// How the system clipboard and the terminal combine on one operating system.
pub enum Chain<A, B> {
    Fallback(Fallback<A, B>),
    Both(Both<A, B>),
}

impl<A: Clipboard, B: Clipboard> Clipboard for Chain<A, B> {
    fn copy(&mut self, text: &str) -> Result<(), String> {
        match self {
            Self::Fallback(chain) => chain.copy(text),
            Self::Both(chain) => chain.copy(text),
        }
    }
}

/// The chain for `os` (as `std::env::consts::OS` names it): both on Linux, where the system
/// clipboard's contents can leave with the process that set them, else the terminal as a fallback.
pub fn for_os<A, B>(os: &str, system: A, terminal: B) -> Chain<A, B> {
    if os == "linux" {
        Chain::Both(Both(system, terminal))
    } else {
        Chain::Fallback(Fallback(system, terminal))
    }
}

/// The chain the binary copies through: the system clipboard and the terminal, combined for the
/// operating system it runs on.
pub fn system_chain() -> Chain<System, Terminal> {
    for_os(std::env::consts::OS, System, Terminal)
}

/// The OSC 52 sequence that sets the terminal's clipboard to `text`.
pub fn osc52(text: &str) -> String {
    format!("\x1b]52;c;{}\x07", base64(text.as_bytes()))
}

/// Standard base64 with padding (RFC 4648), which is all OSC 52 needs.
fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    bytes
        .chunks(3)
        .flat_map(|chunk| {
            let [a, b, c] = [0, 1, 2].map(|i| chunk.get(i).copied().unwrap_or(0));
            let joined = u32::from(a) << 16 | u32::from(b) << 8 | u32::from(c);
            (0..4).map(move |i| {
                if i <= chunk.len() {
                    char::from(ALPHABET[(joined >> (18 - 6 * i) & 63) as usize])
                } else {
                    '='
                }
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Refuses;

    impl Clipboard for Refuses {
        fn copy(&mut self, _: &str) -> Result<(), String> {
            Err("no display".to_owned())
        }
    }

    #[derive(Default)]
    struct Records(Vec<String>);

    impl Clipboard for Records {
        fn copy(&mut self, text: &str) -> Result<(), String> {
            self.0.push(text.to_owned());
            Ok(())
        }
    }

    #[test]
    fn the_fallback_gets_the_text_only_when_the_primary_fails() {
        let mut working = Fallback(Records::default(), Records::default());
        working.copy("5-6pm AEDT").unwrap();
        assert_eq!((working.0.0.len(), working.1.0.len()), (1, 0));

        let mut failing = Fallback(Refuses, Records::default());
        failing.copy("5-6pm AEDT").unwrap();
        assert_eq!(failing.1.0, ["5-6pm AEDT"]);
    }

    #[test]
    fn both_failing_reports_both_reasons() {
        assert_eq!(
            Fallback(Refuses, Refuses).copy("x"),
            Err("no display; no display".to_owned())
        );
    }

    /// Whether `os`'s chain gives the terminal the text even after the system clipboard took it.
    fn writes_both(os: &str) -> bool {
        let mut chain = for_os(os, Records::default(), Records::default());
        chain.copy("5-6pm AEDT").unwrap();
        let (Chain::Fallback(Fallback(_, terminal)) | Chain::Both(Both(_, terminal))) = &chain;
        terminal.0 == ["5-6pm AEDT"]
    }

    #[test]
    fn on_linux_the_terminal_gets_the_text_even_when_the_system_clipboard_takes_it() {
        assert!(writes_both("linux"));
        assert!(!writes_both("macos"));
    }

    #[test]
    fn the_chain_the_binary_copies_through_writes_both_exactly_when_built_for_linux() {
        let both = matches!(system_chain(), Chain::Both(_));
        assert_eq!(both, cfg!(target_os = "linux"));
    }

    #[test]
    fn both_succeeds_when_either_takes_the_text_and_fails_with_both_reasons() {
        let mut first_only = Both(Records::default(), Refuses);
        assert_eq!(first_only.copy("x"), Ok(()));
        let mut second_only = Both(Refuses, Records::default());
        assert_eq!(second_only.copy("x"), Ok(()));
        assert_eq!(second_only.1.0, ["x"]);
        assert_eq!(
            Both(Refuses, Refuses).copy("x"),
            Err("no display; no display".to_owned())
        );
    }

    #[test]
    fn osc52_wraps_rfc_4648_base64_in_the_set_clipboard_sequence() {
        assert_eq!(osc52("foobar"), "\x1b]52;c;Zm9vYmFy\x07");
        assert_eq!(osc52("fo"), "\x1b]52;c;Zm8=\x07");
        assert_eq!(osc52("f"), "\x1b]52;c;Zg==\x07");
        assert_eq!(osc52(""), "\x1b]52;c;\x07");
    }
}
