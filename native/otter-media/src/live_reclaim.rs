//! Explicit shell-recall state for external Live sources; no OS calls, locks, allocations, or global input listeners.
//! The host may record only verified, non-injected shell input; foreground changes alone do not grant authorization.
pub const INTENT_LIFETIME_MS: u64 = 2000;

#[derive(Clone, Copy, Debug)]
pub struct Intent {
    sequence: u64,
    issued_ms: u64,
    origin: usize,
}
impl Intent {
    fn valid_at(self, now_ms: u64) -> bool {
        now_ms
            .checked_sub(self.issued_ms)
            .is_some_and(|age| age <= INTENT_LIFETIME_MS)
    }
}

/// The UI owner serializes recording and cancellation; callers must read the current snapshot each time, not cache a canceled Intent.
#[derive(Default)]
pub struct IntentTracker {
    sequence: u64,
    current: Option<Intent>,
}
impl IntentTracker {
    /// now_ms comes from the same monotonic clock; origin is the foreground HWND before the input, not the restoration target.
    /// Conservatively revoke authorization when sequence numbers are exhausted; never wrap or use timestamps as sequence numbers.
    pub fn record(&mut self, now_ms: u64, origin: usize) -> bool {
        self.current = None;
        let Some(sequence) = self.sequence.checked_add(1) else {
            return false;
        };
        self.sequence = sequence;
        self.current = Some(Intent {
            sequence,
            issued_ms: now_ms,
            origin,
        });
        true
    }
    pub fn cancel(&mut self) {
        self.current = None;
    }
    pub fn snapshot(&self, now_ms: u64) -> Option<Intent> {
        self.current.filter(|intent| intent.valid_at(now_ms))
    }
}

/// One instance per shared visibility owner, not per frame consumer.
/// User recall restores only visibility; callers must not release capture/input leases as a result.
pub struct Activation {
    consumed: u64,
    foreground: usize,
}
impl Activation {
    /// Absorb existing intent when logical hiding starts so a pre-hide gesture cannot immediately restore the source.
    pub fn new(current: Option<Intent>, foreground: usize) -> Self {
        Self {
            consumed: current.map_or(0, |intent| intent.sequence),
            foreground,
        }
    }
    pub fn observe(
        &mut self,
        foreground: usize,
        source: usize,
        iconic: bool,
        current: Option<Intent>,
        now_ms: u64,
    ) -> bool {
        let previous = self.foreground;
        self.foreground = foreground;
        let Some(intent) = current else {
            return false;
        };
        // Clicking the taskbar may minimize a transparent source already in the foreground; other minimized sources cannot consume this gesture.
        let transition = source != 0
            && ((iconic && intent.origin == source)
                || (!iconic && foreground == source && previous != source));
        if !transition || intent.sequence <= self.consumed {
            return false;
        }
        self.consumed = intent.sequence;
        intent.valid_at(now_ms)
    }
}

#[cfg(test)]
#[path = "live_reclaim_tests.rs"]
mod tests;
