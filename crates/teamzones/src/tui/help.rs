//! The `?` overlay: the context's keys, then the global keys, then the glyph legend.

use super::glyphs::Glyphs;
use super::keys::{BINDINGS, Context};

/// One help line: a heading (`keys` empty) or a key and what it does.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Line {
    pub keys: String,
    pub text: String,
}

fn heading(text: &str) -> Line {
    Line {
        keys: String::new(),
        text: text.to_owned(),
    }
}

fn keys_of(context: Context, glyphs: &Glyphs) -> impl Iterator<Item = Line> + '_ {
    BINDINGS
        .iter()
        .filter(move |binding| binding.context == context && !binding.chords.is_empty())
        .map(|binding| Line {
            keys: glyphs.text(binding.keys),
            text: glyphs.text(&binding.action.replace('`', "")),
        })
}

/// The overlay's lines for help opened from `context`, all drawn from the binding and glyph tables.
pub fn lines(context: Context, glyphs: &Glyphs) -> Vec<Line> {
    let legend = [
        format!(
            "{} day  {} morning/evening  {} working  {} night  {} off",
            glyphs.day, glyphs.dawn, glyphs.work, glyphs.night, glyphs.off
        ),
        format!(
            "{} closed  {} open  {} busy",
            glyphs.quiet, glyphs.off_peak, glyphs.peak
        ),
        format!(
            "{} home  {} counted  {} not counted  {} counted place  {} other place",
            glyphs.home, glyphs.person, glyphs.person_other, glyphs.place, glyphs.place_other
        ),
        format!(
            "{} fits  {} fits in part  {} cursor  {}{} range",
            glyphs.track_fit,
            glyphs.track_partial,
            glyphs.cursor,
            glyphs.range_start,
            glyphs.range_end
        ),
    ];
    let here: Vec<Line> = keys_of(context, glyphs).collect();
    let here_heading = (!here.is_empty()).then(|| heading("keys here"));
    here_heading
        .into_iter()
        .chain(here)
        .chain(std::iter::once(heading("everywhere")))
        .chain(keys_of(Context::Global, glyphs))
        .chain(std::iter::once(heading("glyphs")))
        .chain(legend.into_iter().map(|text| Line {
            keys: String::new(),
            text,
        }))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::glyphs::FULL;

    #[test]
    fn help_from_the_copy_dialog_lists_only_the_global_keys() {
        let from_dialog = lines(Context::CopyMenu, &FULL);
        assert_eq!(from_dialog[0].text, "everywhere");
        let globals = keys_of(Context::Global, &FULL).count();
        assert_eq!(from_dialog[globals + 1].text, "glyphs");
    }

    #[test]
    fn context_keys_come_first_then_global_keys_then_the_legend() {
        let lines = lines(Context::Main, &FULL);
        let at = |text: &str| lines.iter().position(|line| line.text == text).unwrap();
        assert_eq!(lines[1].keys, "h l, ← →");
        assert!(at("everywhere") > at("Copy the say line"));
        assert_eq!(lines[at("everywhere") + 1].keys, "esc");
        assert!(at("glyphs") > at("Help; quit; quit (always, even in prompts)"));
        assert_eq!(
            lines.last().unwrap().text,
            "═ fits  ╌ fits in part  ┃ cursor  ┣┫ range"
        );
    }

    #[test]
    fn unbuilt_keys_are_not_listed() {
        assert!(
            lines(Context::Main, &FULL)
                .iter()
                .all(|line| line.keys != "tab")
        );
    }
}
