//! Mouse capture and every mouse equivalent in the keymap.

use super::*;
use crate::tui::{Modes, run_effect};

/// Records every mouse capture switch the effects layer asks the terminal for.
#[derive(Default)]
struct Recorder {
    capture: Vec<bool>,
}

impl Modes for Recorder {
    fn capture_mouse(&mut self, on: bool) -> std::io::Result<()> {
        self.capture.push(on);
        Ok(())
    }
}

struct NoClipboard;

impl crate::clipboard::Clipboard for NoClipboard {
    fn copy(&mut self, _text: &str) -> Result<(), String> {
        Ok(())
    }
}

/// Presses `keys` and runs every effect through `modes`, as the shell does.
fn press_running(app: App, keys: &[Key], modes: &mut Recorder) -> App {
    let (app, effects) = press_all(app, keys);
    effects.into_iter().fold(app, |app, effect| {
        update(app, run_effect(effect, &mut NoClipboard, modes)).0
    })
}

fn footer(app: &App) -> String {
    row(&render(app, 120, 36), 35)
}

#[test]
fn capital_m_releases_the_mouse_and_says_so_until_m_again_recaptures_it() {
    let mut terminal = Recorder::default();
    let released = press_running(fixture(), &chars("M"), &mut terminal);
    assert_eq!(terminal.capture, [false]);
    assert!(
        footer(&released).ends_with("mouse off"),
        "{}",
        footer(&released)
    );
    let recaptured = press_running(released, &chars("M"), &mut terminal);
    assert_eq!(terminal.capture, [false, true]);
    assert!(
        !footer(&recaptured).contains("mouse off"),
        "{}",
        footer(&recaptured)
    );
}
