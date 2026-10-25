//! The footer: the context's hints for this moment, dropped from the lowest priority up until
//! they fit, after a mode badge in range mode.

use super::keys::{Context, Hint, PINNED, Shown, hints};

const GAP: usize = 2;

/// What the conditional hints depend on: a committed range, being anywhere but now, and what
/// `m` does for the map.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Situation {
    pub range: bool,
    pub away: bool,
    pub map: MapKey,
}

/// What `m` does: nothing (the ascii glyph set forces the map off), open the map the hiding rule
/// found no room for, or flip the map's choice where there is room.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MapKey {
    Inert,
    Opens,
    Flips,
}

/// One footer: the mode badge, if any, then the hints that fit, in display order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Footer {
    pub badge: Option<&'static str>,
    pub hints: Vec<Hint>,
}

impl Footer {
    /// The footer as plain text, as it reads on screen.
    #[cfg(test)]
    pub fn text(&self) -> String {
        self.badge
            .into_iter()
            .map(str::to_owned)
            .chain(self.hints.iter().map(|h| format!("{} {}", h.keys, h.label)))
            .collect::<Vec<_>>()
            .join("  ")
    }
}

/// The badge that names a mode the footer is in.
pub fn badge(context: Context) -> Option<&'static str> {
    match context {
        Context::Range => Some("range"),
        Context::Main
        | Context::Global
        | Context::Prompt
        | Context::CopyMenu
        | Context::SortMenu
        | Context::Help => None,
    }
}

fn shows(shown: Shown, situation: Situation) -> bool {
    match shown {
        Shown::Always => true,
        Shown::WithRange => situation.range,
        Shown::AwayFromNow => situation.away,
        Shown::WithMap => match situation.map {
            MapKey::Inert => false,
            MapKey::Opens | MapKey::Flips => true,
        },
    }
}

/// The footer for `context` in `situation`, fitted to `columns`.
pub fn line(context: Context, situation: Situation, columns: u16) -> Footer {
    let badge = badge(context);
    let room = badge.map_or(usize::from(columns), |badge| {
        usize::from(columns).saturating_sub(badge.len() + GAP)
    });
    let candidates: Vec<(Hint, u8)> = hints(context)
        .into_iter()
        .filter(|(hint, _)| shows(hint.shown, situation))
        .map(|(hint, priority)| {
            let raised = hint.raised.filter(|_| situation.map == MapKey::Opens);
            (hint, raised.unwrap_or(priority))
        })
        .collect();
    Footer {
        badge,
        hints: fit(&candidates, room),
    }
}

fn width(hints: &[&Hint]) -> usize {
    let text: usize = hints.iter().map(|h| h.keys.len() + 1 + h.label.len()).sum();
    text + GAP * hints.len().saturating_sub(1)
}

/// The hints that fit in `columns`, in display order; pinned hints always stay.
fn fit(hints: &[(Hint, u8)], columns: usize) -> Vec<Hint> {
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
        if hints[index].1 != PINNED && width(&chosen) > columns {
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
    use crate::tui::keys::Context;

    const AT_NOW: Situation = Situation {
        range: false,
        away: false,
        map: MapKey::Flips,
    };
    const AWAY: Situation = Situation {
        range: false,
        away: true,
        map: MapKey::Flips,
    };
    const RANGED: Situation = Situation {
        range: true,
        away: true,
        map: MapKey::Flips,
    };

    #[test]
    fn m_drops_first_while_the_map_shows_and_ranks_below_h_l_while_it_is_hidden() {
        let hidden = Situation {
            map: MapKey::Opens,
            ..AWAY
        };
        assert_eq!(
            at(Context::Main, AWAY, 60),
            "h/l time  . now  v range  space mark  y copy  ? help  q quit"
        );
        assert_eq!(
            at(Context::Main, hidden, 60),
            "h/l time  . now  space mark  y copy  m map  ? help  q quit"
        );
    }

    #[test]
    fn m_is_never_offered_while_it_does_nothing() {
        let inert = Situation {
            map: MapKey::Inert,
            ..RANGED
        };
        assert!(!at(Context::Main, inert, 200).contains("m map"));
    }

    fn at(context: Context, situation: Situation, columns: u16) -> String {
        line(context, situation, columns).text()
    }

    #[test]
    fn at_now_the_main_footer_has_no_mode_hints_and_no_group_hint() {
        assert_eq!(
            at(Context::Main, AT_NOW, 120),
            "h/l time  v range  n next fit  space mark  y copy  c copy as  m map  ? help  q quit"
        );
        assert_eq!(
            at(Context::Main, AT_NOW, 80),
            "h/l time  v range  n next fit  space mark  y copy  c copy as  ? help  q quit"
        );
    }

    #[test]
    fn away_from_now_the_footer_offers_dot_now() {
        assert_eq!(
            at(Context::Main, AWAY, 120),
            "h/l time  . now  v range  n next fit  space mark  y copy  c copy as  m map  ? help  q quit"
        );
        assert_eq!(
            at(Context::Main, AWAY, 80),
            "h/l time  . now  v range  n next fit  space mark  y copy  m map  ? help  q quit"
        );
    }

    #[test]
    fn with_a_committed_range_the_footer_offers_esc_and_keeps_it_last_to_drop() {
        assert_eq!(
            at(Context::Main, RANGED, 120),
            "h/l time  . now  v range  n next fit  space mark  y copy  c copy as  m map  esc clear range  ? help  q quit"
        );
        assert_eq!(
            at(Context::Main, RANGED, 80),
            "h/l time  . now  v range  space mark  y copy  esc clear range  ? help  q quit"
        );
        assert_eq!(
            at(Context::Main, RANGED, 40),
            ". now  esc clear range  ? help  q quit"
        );
    }

    const SCREEN_SPEC: &str = include_str!("../../../../docs/spec/screen.md");

    /// The first backticked text after `marker` in the screen spec.
    fn spec_code_after(marker: &str) -> &'static str {
        let (_, rest) = SCREEN_SPEC
            .split_once(marker)
            .expect("the screen spec has the marker");
        rest.split('`')
            .nth(1)
            .expect("a backticked value follows the marker")
    }

    #[test]
    fn while_drawing_a_range_the_footer_leads_with_the_mode_and_only_its_keys() {
        let keys = spec_code_after("shows only the range keys: ");
        let expected = format!("range  {keys}  ? help");
        assert_eq!(at(Context::Range, RANGED, 120), expected);
        assert_eq!(at(Context::Range, RANGED, 80), expected);
        assert_eq!(
            at(Context::Range, RANGED, 56),
            "range  h/l move end  v/enter done  esc cancel  ? help"
        );
    }
}
