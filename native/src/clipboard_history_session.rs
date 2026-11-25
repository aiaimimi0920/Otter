//! Open history-menu session: sole owner of event coalescing, asynchronous refresh, and selectable snapshots.
use super::{events::Events, task::Task, Entry, Snapshot, Status};
use std::time::{Duration, Instant};
use windows::core::Result;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    DeleteFailed,
    Loading,
    Ready,
    Empty,
    Disabled,
    Denied,
    Unavailable,
}

pub struct Session {
    deleting: bool,
    events: Events,
    task: Option<Task>,
    snapshot: Option<Snapshot>,
    state: State,
    pending: bool,
    next_start: Instant,
    started: Instant,
}

impl Session {
    #[cfg(test)]
    pub(crate) fn test_snapshot(snapshot: Snapshot) -> Self {
        let events = Events::synthetic();
        events.take_changed();
        let mut session = Self::new(events, Instant::now());
        session.pending = false;
        session.accept(Ok(snapshot));
        session
    }

    #[cfg(test)]
    pub(crate) fn test_invalidate(&self) {
        self.events.signal();
    }

    /// Called by the menu-opening gesture; UI displays subscription failures, without secretly switching to system-history polling.
    pub fn open() -> Result<Self> {
        Ok(Self::new(Events::subscribe()?, Instant::now()))
    }

    fn new(events: Events, now: Instant) -> Self {
        Self {
            deleting: false,
            events,
            task: None,
            snapshot: None,
            state: State::Loading,
            pending: true,
            next_start: now,
            started: now,
        }
    }

    pub fn state(&self) -> State {
        self.state
    }

    pub fn entries(&self) -> &[Entry] {
        self.snapshot
            .as_ref()
            .map_or(&[], |snapshot| &snapshot.entries)
    }

    /// The UI timer only queries local channels/atomic flags; start actual reads only for pending changes.
    pub fn tick(&mut self) -> bool {
        self.tick_with(Instant::now(), Task::start)
    }

    fn tick_with(&mut self, now: Instant, start: impl FnOnce() -> Result<Task>) -> bool {
        let mut changed = false;
        if self.events.take_changed() {
            self.pending = true;
            self.snapshot = None;
            self.state = State::Loading;
            changed = true;
        }
        if let Some(task) = self.task.as_mut() {
            if let Some(result) = task.poll() {
                self.task = None;
                if self.deleting {
                    self.deleting = false;
                    self.pending = result.is_ok();
                    self.state = if self.pending {
                        State::Loading
                    } else {
                        State::DeleteFailed
                    };
                    return true;
                }
                // Changes occurred during the read: this snapshot is no longer selectable; wait for the next read.
                if !self.pending {
                    self.accept(result);
                    changed = true;
                }
            } else if now.saturating_duration_since(self.started) >= Duration::from_secs(6) {
                // UI timeout only closes the channel without waiting for the provider; the background permit still protects resource lifetime.
                self.task = None;
                self.deleting = false;
                self.state = State::Unavailable;
                changed = true;
            }
        }
        if self.task.is_none() && self.pending && now >= self.next_start {
            self.next_start = now + Duration::from_millis(250);
            match start() {
                Ok(task) => {
                    self.task = Some(task);
                    self.pending = false;
                    self.started = now;
                    self.state = State::Loading;
                    changed = true;
                }
                Err(_) => {
                    // Keep throttling retries while the provider holds the permit, without repeatedly rebuilding the same unavailable menu.
                    changed |= self.state != State::Unavailable;
                    self.state = State::Unavailable;
                }
            }
        }
        changed
    }

    fn accept(&mut self, result: Result<Snapshot>) {
        match result {
            Ok(snapshot) => {
                self.state = match snapshot.status {
                    Status::Ready if snapshot.entries.is_empty() => State::Empty,
                    Status::Ready => State::Ready,
                    Status::Disabled => State::Disabled,
                    Status::Denied => State::Denied,
                };
                self.snapshot = Some(snapshot);
            }
            Err(_) => {
                self.snapshot = None;
                self.state = State::Unavailable;
            }
        }
    }

    /// Consume events again before clicking so changes since the last draw cannot use an old hit index.
    /// Callers still check window/document targets afterward, then close the menu before delivering content.
    pub fn take_entry(&mut self, index: usize, painted_id: &str) -> Option<Entry> {
        if self.events.take_changed() {
            self.pending = true;
            self.snapshot = None;
            self.state = State::Loading;
        }
        if self.pending || self.task.is_some() || self.state != State::Ready {
            return None;
        }
        let snapshot = self.snapshot.as_mut()?;
        if snapshot.entries.get(index)?.id != painted_id {
            return None;
        }
        let entry = snapshot.entries.remove(index);
        self.snapshot = None;
        self.state = State::Empty;
        Some(entry)
    }
    /// Share latest-snapshot identity checks with retrieval; start bounded background work only after an explicit deletion gesture.
    pub fn delete_entry(&mut self, index: usize, painted_id: &str) -> bool {
        self.delete_with(index, painted_id, Task::delete)
    }
    fn delete_with(
        &mut self,
        index: usize,
        painted_id: &str,
        start: impl FnOnce(String) -> Result<Task>,
    ) -> bool {
        let Some(entry) = self.take_entry(index, painted_id) else {
            return false;
        };
        match start(entry.id) {
            Ok(task) => {
                self.task = Some(task);
                self.deleting = true;
                self.started = Instant::now();
                self.state = State::Loading;
            }
            Err(_) => self.state = State::DeleteFailed,
        }
        true
    }
}

#[cfg(test)]
#[path = "clipboard_history_session_tests.rs"]
mod tests;
