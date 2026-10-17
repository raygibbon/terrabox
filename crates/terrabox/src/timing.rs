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

/// Caller-driven frame pacing. No thread, timer, render loop or catch-up burst.
pub struct FrameClock {
    interval: Duration,
    next: Option<Instant>,
    last: Option<Instant>,
    paused: bool,
}
#[derive(Clone, Copy, Debug)]
pub struct FrameTiming {
    pub elapsed: Duration,
    pub lateness: Duration,
}
impl FrameClock {
    pub fn new(interval: Duration, now: Instant) -> Option<Self> {
        if interval.is_zero() || now.checked_add(interval).is_none() {
            return None;
        }
        Some(Self {
            interval,
            next: Some(now),
            last: None,
            paused: false,
        })
    }
    pub fn deadline(&self) -> Option<Deadline> {
        self.next.map(Deadline)
    }
    pub fn paused(&self) -> bool {
        self.paused
    }
    pub fn pause(&mut self) {
        self.paused = true;
        self.next = None;
        self.last = None;
    }
    pub fn resume(&mut self, now: Instant) {
        self.paused = false;
        self.next = Some(now);
        self.last = None;
    }
    pub fn set_interval(&mut self, interval: Duration, now: Instant) -> bool {
        let Some(next) = now.checked_add(interval) else {
            return false;
        };
        if interval.is_zero() {
            return false;
        }
        self.interval = interval;
        if !self.paused {
            self.next = Some(next);
        }
        true
    }
    pub fn poll(&mut self, now: Instant) -> Option<FrameTiming> {
        let deadline = self.next?;
        if now < deadline {
            return None;
        }
        let sample = FrameTiming {
            elapsed: self
                .last
                .map(|last| now.saturating_duration_since(last))
                .unwrap_or_default(),
            lateness: now.saturating_duration_since(deadline),
        };
        self.last = Some(now);
        // Schedule from the actual frame; late callers skip missed frames.
        self.next = now.checked_add(self.interval);
        Some(sample)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pacing_pause_speed_and_late_frames_are_explicit() {
        let now = Instant::now();
        let mut clock = FrameClock::new(Duration::from_millis(20), now).unwrap();
        assert_eq!(clock.poll(now).unwrap().elapsed, Duration::ZERO);
        assert!(clock.poll(now).is_none());
        let late = now + Duration::from_millis(100);
        let timing = clock.poll(late).unwrap();
        assert_eq!(timing.elapsed, Duration::from_millis(100));
        assert_eq!(timing.lateness, Duration::from_millis(80));
        assert_eq!(
            clock.deadline(),
            Some(Deadline(late + Duration::from_millis(20)))
        );
        clock.pause();
        assert!(clock.deadline().is_none());
        assert!(clock.poll(late).is_none());
        clock.resume(late);
        assert_eq!(clock.poll(late).unwrap().elapsed, Duration::ZERO);
        assert!(!clock.set_interval(Duration::ZERO, late));
        assert!(clock.set_interval(Duration::from_millis(40), late));
        assert_eq!(
            clock.deadline(),
            Some(Deadline(late + Duration::from_millis(40)))
        );
        assert!(FrameClock::new(Duration::ZERO, now).is_none());
    }
}
