//! Copying the say line: the system clipboard first, then OSC 52 to the terminal.

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

/// The OSC 52 sequence that sets the terminal's clipboard to `text`.
pub fn osc52(text: &str) -> String {
    format!("\x1b]52;c;{}\x07", base64(text.as_bytes()))
}

/// Standard base64 with padding (RFC 4648), which is all OSC 52 needs.
fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
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

    #[test]
    fn osc52_wraps_rfc_4648_base64_in_the_set_clipboard_sequence() {
        assert_eq!(osc52("foobar"), "\x1b]52;c;Zm9vYmFy\x07");
        assert_eq!(osc52("fo"), "\x1b]52;c;Zm8=\x07");
        assert_eq!(osc52("f"), "\x1b]52;c;Zg==\x07");
        assert_eq!(osc52(""), "\x1b]52;c;\x07");
    }
}
