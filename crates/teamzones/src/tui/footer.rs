//! The footer: the context's hints, dropped from the lowest priority up until they fit.

use super::keys::{Hint, PINNED};

const GAP: usize = 2;

fn width(hints: &[&Hint]) -> usize {
    let text: usize = hints.iter().map(|h| h.keys.len() + 1 + h.label.len()).sum();
    text + GAP * hints.len().saturating_sub(1)
}

/// The hints that fit in `columns`, in display order; pinned hints always stay.
pub fn fit(hints: &[(Hint, u8)], columns: u16) -> Vec<Hint> {
    let mut by_priority: Vec<usize> = (0..hints.len()).collect();
    by_priority.sort_by_key(|index| std::cmp::Reverse(hints[*index].1));
    let mut kept = vec![false; hints.len()];
    for index in by_priority {
        kept[index] = true;
        let chosen: Vec<&Hint> = hints
            .iter()
            .zip(&kept)
            .filter(|(_, keep)| **keep)
            .map(|((hint, _), _)| hint)
            .collect();
        if hints[index].1 != PINNED && width(&chosen) > usize::from(columns) {
            kept[index] = false;
        }
    }
    hints
        .iter()
        .zip(kept)
        .filter(|(_, keep)| *keep)
        .map(|((hint, _), _)| *hint)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::keys::{Context, hints};

    fn keys_at(columns: u16) -> Vec<&'static str> {
        fit(&hints(Context::Main), columns)
            .iter()
            .map(|hint| hint.keys)
            .collect()
    }

    #[test]
    fn at_120_columns_every_built_hint_fits() {
        assert_eq!(
            keys_at(120),
            ["h/l", "v", "n", "space", "1-9", "y", "c", "?", "q"]
        );
    }

    #[test]
    fn at_80_columns_copy_as_drops_first_and_help_and_quit_stay() {
        assert_eq!(
            keys_at(80),
            ["h/l", "v", "n", "space", "1-9", "y", "?", "q"]
        );
    }

    #[test]
    fn narrower_footers_drop_next_fit_then_range_then_time() {
        assert_eq!(keys_at(64), ["h/l", "v", "space", "1-9", "y", "?", "q"]);
        assert_eq!(keys_at(56), ["h/l", "space", "1-9", "y", "?", "q"]);
        assert_eq!(keys_at(46), ["space", "1-9", "y", "?", "q"]);
        assert_eq!(keys_at(10), ["?", "q"]);
    }
}
