//! Latest-frame Live delivery; single producer, bounded consumers, shared resources instead of per-consumer copies/queues.
use std::sync::{Arc, Condvar, Mutex, MutexGuard, Weak};
#[path = "live_frame_wake.rs"]
pub mod wake;
pub const MAX_CONSUMERS: usize = 16;
pub struct Frame<T> {
    pub sequence: u64,
    pub resource: T,
}
pub struct Delivery<T> {
    pub frame: Arc<Frame<T>>,
    pub skipped: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Closed,
    NoConsumers,
    ConsumerLimit,
    SequenceExhausted,
    OwnerConflict,
}
struct State<T> {
    latest: Option<Arc<Frame<T>>>,
    sequence: u64,
    consumers: usize,
    closed: bool,
    idle: Option<Arc<dyn Fn() + Send + Sync>>,
    wakes: Vec<Weak<wake::Wake>>,
}
struct Shared<T> {
    state: Mutex<State<T>>,
    changed: Condvar,
}
impl<T> Shared<T> {
    fn lock(&self) -> MutexGuard<'_, State<T>> {
        // Locks cover only internal metadata, never external calls; move resource destruction outside locks.
        self.state.lock().unwrap_or_else(|error| error.into_inner())
    }
}
/// Noncloneable source owner; destruction closes all consumers without forcibly reclaiming borrowed frames.
pub struct Source<T> {
    shared: Arc<Shared<T>>,
}
pub struct Consumer<T> {
    shared: Arc<Shared<T>>,
    cursor: u64,
    wake: Option<Arc<wake::Wake>>,
}
impl<T> Default for Source<T> {
    fn default() -> Self {
        Self::new()
    }
}
impl<T> Source<T> {
    pub fn new() -> Self {
        Self {
            shared: Arc::new(Shared {
                state: Mutex::new(State {
                    latest: None,
                    sequence: 0,
                    consumers: 0,
                    closed: false,
                    idle: None,
                    wakes: Vec::new(),
                }),
                changed: Condvar::new(),
            }),
        }
    }
    pub fn subscribe(&self) -> Result<Consumer<T>, Error> {
        let mut state = self.shared.lock();
        if state.closed {
            return Err(Error::Closed);
        }
        if state.consumers == MAX_CONSUMERS {
            return Err(Error::ConsumerLimit);
        }
        state.consumers += 1;
        // New consumers may immediately take current frames, without counting presubscription frames as their own drops.
        let cursor = if state.latest.is_some() {
            state.sequence - 1
        } else {
            state.sequence
        };
        Ok(Consumer {
            shared: self.shared.clone(),
            cursor,
            wake: None,
        })
    }
    pub fn has_consumers(&self) -> bool {
        let state = self.shared.lock();
        !state.closed && state.consumers != 0
    }
    /// Capture owners register one final-consumer-exit notice; callbacks only wake and never run under state locks.
    pub fn on_idle(&self, wake: Arc<dyn Fn() + Send + Sync>) -> Result<(), Error> {
        let mut state = self.shared.lock();
        if state.closed {
            return Err(Error::Closed);
        }
        if state.idle.is_some() {
            return Err(Error::OwnerConflict);
        }
        let idle = state.consumers == 0;
        state.idle = Some(wake.clone());
        drop(state);
        if idle {
            wake();
        }
        Ok(())
    }
    pub fn publish(&self, resource: T) -> Result<u64, Error> {
        let mut state = self.shared.lock();
        if state.closed {
            return Err(Error::Closed);
        }
        if state.consumers == 0 {
            return Err(Error::NoConsumers);
        }
        let sequence = state
            .sequence
            .checked_add(1)
            .ok_or(Error::SequenceExhausted)?;
        let previous = state.latest.replace(Arc::new(Frame { sequence, resource }));
        state.sequence = sequence;
        let wakes = observers(&state);
        drop(state);
        self.shared.changed.notify_all();
        for wake in wakes.into_iter().flatten() {
            wake.notify();
        }
        drop(previous);
        Ok(sequence)
    }
    pub fn close(&self) {
        let mut state = self.shared.lock();
        if state.closed {
            return;
        }
        state.closed = true;
        let previous = state.latest.take();
        let idle = state.idle.take();
        let wakes = observers(&state);
        drop(state);
        self.shared.changed.notify_all();
        for wake in wakes.into_iter().flatten() {
            wake.notify();
        }
        if let Some(wake) = idle {
            wake();
        }
        drop(previous);
    }
}
impl<T> Drop for Source<T> {
    fn drop(&mut self) {
        self.close();
    }
}
impl<T> Consumer<T> {
    pub fn notification_failures(&self) -> u64 {
        self.wake.as_ref().map_or(0, |wake| wake.failures())
    }
    /// Nonblocking notification callback returns delivery success; bind once, invoke outside source locks, with no pixels/pointers.
    /// UI must validate its connection generation: cancellation cannot retract notices already queued by the system.
    pub fn set_waker(
        &mut self,
        post: impl Fn() -> bool + Send + Sync + 'static,
    ) -> Result<(), Error> {
        if self.wake.is_some() {
            return Err(Error::OwnerConflict);
        }
        let wake = Arc::new(wake::Wake::new(post));
        let mut state = self.shared.lock();
        if state.closed {
            return Err(Error::Closed);
        }
        state.wakes.retain(|entry| entry.strong_count() != 0);
        if state.wakes.len() == MAX_CONSUMERS {
            return Err(Error::ConsumerLimit);
        }
        state.wakes.push(Arc::downgrade(&wake));
        let ready = state
            .latest
            .as_ref()
            .is_some_and(|frame| frame.sequence > self.cursor);
        self.wake = Some(wake.clone());
        drop(state);
        if ready {
            wake.notify();
        }
        Ok(())
    }
    fn take(state: &State<T>, cursor: &mut u64) -> Result<Option<Delivery<T>>, Error> {
        if state.closed {
            return Err(Error::Closed);
        }
        let Some(frame) = state
            .latest
            .as_ref()
            .filter(|frame| frame.sequence > *cursor)
        else {
            return Ok(None);
        };
        let skipped = frame.sequence - *cursor - 1;
        *cursor = frame.sequence;
        Ok(Some(Delivery {
            frame: frame.clone(),
            skipped,
        }))
    }
    /// UI tries reads after existing event notices; no polling threads.
    pub fn try_next(&mut self) -> Result<Option<Delivery<T>>, Error> {
        let state = self.shared.lock();
        if let Some(wake) = &self.wake {
            wake.acknowledge();
        }
        Self::take(&state, &mut self.cursor)
    }
    /// Background consumers block here; source close wakes them without periodic timeouts.
    pub fn wait_next(&mut self) -> Result<Delivery<T>, Error> {
        let mut state = self.shared.lock();
        loop {
            if let Some(wake) = &self.wake {
                wake.acknowledge();
            }
            if let Some(frame) = Self::take(&state, &mut self.cursor)? {
                return Ok(frame);
            }
            state = self
                .shared
                .changed
                .wait(state)
                .unwrap_or_else(|error| error.into_inner());
        }
    }
}
impl<T> Drop for Consumer<T> {
    fn drop(&mut self) {
        let retired = self.wake.take();
        if let Some(wake) = &retired {
            wake.disable();
        }
        let mut state = self.shared.lock();
        if let Some(wake) = &retired {
            let identity = Arc::downgrade(wake);
            state.wakes.retain(|entry| !Weak::ptr_eq(entry, &identity));
        }
        state.consumers -= 1;
        let previous = if state.consumers == 0 {
            state.latest.take()
        } else {
            None
        };
        let idle = (state.consumers == 0).then(|| state.idle.clone()).flatten();
        drop(state);
        drop(retired);
        if let Some(wake) = idle {
            wake();
        }
        drop(previous);
    }
}
fn observers<T>(state: &State<T>) -> [Option<Arc<wake::Wake>>; MAX_CONSUMERS] {
    // Fixed stack arrays avoid hot-path notification-queue allocation; weak references do not extend consumer lifetimes.
    std::array::from_fn(|index| state.wakes.get(index).and_then(Weak::upgrade))
}
#[cfg(test)]
#[path = "live_frames_tests.rs"]
mod tests;
