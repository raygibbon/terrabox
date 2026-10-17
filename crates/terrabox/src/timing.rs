use std::time::{Duration, Instant};

/// An optional frame or timer wake deadline, independent of terminal I/O.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Deadline(pub Instant);

impl Deadline {
    /// Returns None when the requested duration cannot be represented.
    pub fn after(duration: Duration) -> Option<Self> {
        Instant::now().checked_add(duration).map(Self)
    }

    pub fn remaining(self, now: Instant) -> Duration {
        self.0.saturating_duration_since(now)
    }

    pub fn is_due(self, now: Instant) -> bool {
        self.0 <= now
    }
}
