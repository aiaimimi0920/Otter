//! One in-flight probe per connection; a timeout only clears the measurement, not authorization or connection-close policy.
use std::time::{Duration, Instant};

const INTERVAL: Duration = Duration::from_secs(2);
const EXPIRY: Duration = Duration::from_secs(10);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Clock,
    Exhausted,
}

pub struct Liveness {
    last: Instant,
    sequence: u64,
    pending: Option<(u64, Instant)>,
    rtt: Option<Duration>,
}

impl Liveness {
    /// Create a new instance for each connection; do not carry in-flight state across reconnections.
    pub fn new(now: Instant) -> Self {
        Self {
            last: now,
            sequence: 0,
            pending: None,
            rtt: None,
        }
    }

    pub fn poll(&mut self, now: Instant) -> Result<Option<[u8; 8]>, Error> {
        let elapsed = now.checked_duration_since(self.last).ok_or(Error::Clock)?;
        if let Some((_, sent)) = self.pending {
            if now.checked_duration_since(sent).ok_or(Error::Clock)? >= EXPIRY {
                self.pending = None;
                self.rtt = None;
            }
        }
        if self.pending.is_some() || elapsed < INTERVAL {
            return Ok(None);
        }
        let sequence = self.sequence.checked_add(1).ok_or(Error::Exhausted)?;
        self.sequence = sequence;
        self.last = now;
        self.pending = Some((sequence, now));
        Ok(Some(sequence.to_be_bytes()))
    }

    /// Late, out-of-order, or foreign-connection Pong messages cannot update RTT.
    pub fn observe(&mut self, payload: &[u8], now: Instant) -> bool {
        let Some((sequence, sent)) = self.pending else {
            return false;
        };
        let Some(elapsed) = now.checked_duration_since(sent) else {
            return false;
        };
        if elapsed >= EXPIRY {
            self.pending = None;
            self.rtt = None;
            return false;
        }
        if payload != sequence.to_be_bytes() {
            return false;
        }
        self.rtt = Some(elapsed);
        self.pending = None;
        true
    }

    pub fn rtt(&self) -> Option<Duration> {
        self.rtt
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cadence_exact_match_and_one_outstanding_probe() {
        let now = Instant::now();
        let mut state = Liveness::new(now);
        assert_eq!(
            state.poll(now + INTERVAL - Duration::from_nanos(1)),
            Ok(None)
        );
        let ping = state.poll(now + INTERVAL).unwrap().unwrap();
        assert_eq!(state.poll(now + INTERVAL * 2), Ok(None));
        assert!(!state.observe(&[1], now + INTERVAL * 2));
        assert!(!state.observe(&2u64.to_be_bytes(), now + INTERVAL * 2));
        assert!(state.observe(&ping, now + INTERVAL * 2));
        assert_eq!(state.rtt(), Some(INTERVAL));
        assert!(!state.observe(&ping, now + INTERVAL * 2));
        let next = state.poll(now + INTERVAL * 2).unwrap().unwrap();
        assert_ne!(ping, next);
        assert_eq!(state.rtt(), Some(INTERVAL));
        assert!(!state.observe(&ping, now + INTERVAL * 2));
    }

    #[test]
    fn expiry_clears_old_rtt_and_late_pong_cannot_match_new_probe() {
        let now = Instant::now();
        let mut state = Liveness::new(now);
        let first = state.poll(now + INTERVAL).unwrap().unwrap();
        assert!(state.observe(&first, now + INTERVAL));
        let old = state.poll(now + INTERVAL * 2).unwrap().unwrap();
        let expired = now + INTERVAL * 2 + EXPIRY;
        let new = state.poll(expired).unwrap().unwrap();
        assert_eq!(state.rtt(), None);
        assert!(!state.observe(&old, expired));
        assert!(!state.observe(&new, expired + EXPIRY));
        assert_eq!(state.pending, None);
    }

    #[test]
    fn clock_and_sequence_fail_closed_without_reusing_id() {
        let now = Instant::now();
        let mut state = Liveness::new(now);
        assert_eq!(state.poll(now - Duration::from_secs(1)), Err(Error::Clock));
        let ping = state.poll(now + INTERVAL).unwrap().unwrap();
        assert!(!state.observe(&ping, now));
        assert!(state.pending.is_some());
        state.pending = None;
        state.sequence = u64::MAX;
        assert_eq!(state.poll(now + INTERVAL * 2), Err(Error::Exhausted));
        assert_eq!(state.pending, None);
    }
}
