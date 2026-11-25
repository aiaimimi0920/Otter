//! Local paint is not server applied confirmation; open unsent input for the current layout only after the original serial heartbeat succeeds.
//! An admission fence, not authorization/a second queue; no pixels, deadline extensions, or replay of uncertain writes.
use super::{Key, State};

pub(crate) struct Acknowledged {
    generation: u64,
    key: Key,
}
pub(crate) struct Heartbeat {
    generation: u64,
    key: Option<Key>,
    applied: Option<u64>,
}
impl Heartbeat {
    /// Share the same Shared borrow used to generate heartbeat bodies; never hold the lock during network waits.
    pub(crate) fn capture(state: &mut State, key: Option<&Key>, applied: Option<u64>) -> Self {
        // Reduce authority before HTTP: the server may clear applied while its reply is in flight; later paints cannot reuse old ACKs.
        // Calls with expired keys do not revoke successor owners; normal same-layout renewal avoids unnecessary input pauses.
        if state.desired.as_ref() == key
            && (key.is_none_or(|key| applied != Some(key.revision))
                || state.black
                || state.frozen
                || state.identification.blocked())
        {
            state.input_applied = None;
        }
        Self {
            generation: state.generation,
            key: key.cloned(),
            applied,
        }
    }
    /// Called only by the original presenter after HTTP/strict ACK validation succeeds; one presenter has no parallel heartbeats.
    pub(crate) fn acknowledge(self, state: &mut State) {
        if state.stopped
            || state.expired
            || state.generation != self.generation
            || state.desired != self.key
        {
            return;
        }
        state.input_applied = self
            .key
            .filter(|key| {
                self.applied == Some(key.revision)
                    && !state.black
                    && !state.frozen
                    && !state.identification.blocked()
            })
            .map(|key| Acknowledged {
                generation: self.generation,
                key,
            });
    }
}
pub(crate) fn permits(state: &State, key: &Key) -> bool {
    state.input_applied.as_ref().is_some_and(|ack| {
        ack.generation == state.generation && &ack.key == key && state.desired.as_ref() == Some(key)
    })
}

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-surface")))]
#[path = "tile_presenter_admission_tests.rs"]
mod tests;
