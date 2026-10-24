//! The notice row's one value: what it says and when it was raised.

use chrono::{DateTime, TimeDelta, Utc};

/// How long a notice shows, and how long `u` stays live after the action it undoes.
pub const TIMEOUT: TimeDelta = TimeDelta::seconds(10);

/// A notice: its key, two spaces, then the text, as in `n  next fit for deploy: ...`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Notice {
    pub text: String,
    pub raised_at: DateTime<Utc>,
}

impl Notice {
    pub fn new(key: &str, text: &str, now: DateTime<Utc>) -> Self {
        Self {
            text: format!("{key}  {text}"),
            raised_at: now,
        }
    }

    /// Whether it still shows at `now`, decided from the sampled now rather than a stored flag.
    pub fn visible(&self, now: DateTime<Utc>) -> bool {
        now - self.raised_at < TIMEOUT
    }
}
