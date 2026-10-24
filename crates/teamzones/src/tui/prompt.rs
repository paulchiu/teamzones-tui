//! The `t` prompt: editing its text, and what the typed time means.

use chrono::{DateTime, Utc};
use teamzones_core::config::Roster;
use teamzones_core::time::{Moment, parse_time};

/// The input after a typed character, or after `backspace` when `typed` is `None`.
pub fn edit(input: &str, typed: Option<char>) -> String {
    let mut next = input.to_owned();
    match typed {
        Some(c) => next.push(c),
        None => {
            next.pop();
        }
    }
    next
}

/// What the typed text names, or the parse error as the notice should say it.
///
/// # Errors
///
/// The core's typed-time error text, for the notice row.
pub fn submit(input: &str, roster: &Roster, now: DateTime<Utc>) -> Result<Moment, String> {
    parse_time(input, roster, now).map_err(|error| error.to_string())
}
