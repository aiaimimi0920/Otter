//! UI-thread authorization tasks/credential owner; revoked old tasks must never republish credentials.
use crate::{
    device_authorization_lease::{Grant, Lease},
    device_authorization_runtime::{Output, Runtime, Task},
};
use std::os::windows::io::BorrowedHandle;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Closed,
    Origin,
    Pending,
    Capacity,
    Start,
    Credential,
}
pub enum Event {
    OfflineCleared {
        origin: String,
    },
    Pairing {
        origin: String,
        device: String,
        changed: bool,
    },
    Authorized {
        origin: String,
        device: String,
    },
    Failed {
        origin: String,
        error: crate::device_authorization_runtime::Error,
    },
}
struct Pending {
    origin: String,
    runtime: Runtime,
    discarded: bool,
}
#[derive(Default)]
pub struct Owner {
    pending: Vec<Pending>,
    sessions: Vec<(String, Grant)>,
    retiring: Vec<(String, Grant)>,
    closed: bool,
    offline_denied: Vec<String>,
    offline_closed: bool,
}
impl Owner {
    // Each non-revoked in-flight origin already occupies a slot; same-origin renewal shares that slot with the cache.
    fn has_origin_capacity(&self, origin: &str) -> bool {
        let cached = |origin: &str| self.sessions.iter().any(|(o, _)| o == origin);
        cached(origin)
            || self.sessions.len()
                + self
                    .pending
                    .iter()
                    .filter(|p| !p.discarded && !cached(&p.origin))
                    .count()
                < 8
    }

    /// Allow independent offline proof only for origins not yet involved in online sessions in the new process; no downgrade bypass for capacity/explicit invalidation.
    pub(crate) fn can_resume_local(&self, origin: &str) -> bool {
        !self.closed
            && !self.offline_closed
            && !self.offline_denied.iter().any(|v| v == origin)
            && !self.sessions.iter().any(|(v, _)| v == origin)
            && !self.pending.iter().any(|p| p.origin == origin)
    }
    fn deny_offline(&mut self, origin: &str) {
        if self.offline_denied.iter().any(|v| v == origin) {
            return;
        }
        if self.offline_denied.len() >= 32 {
            self.offline_closed = true;
        } else {
            self.offline_denied.push(origin.into());
        }
    }
    pub fn new() -> Self {
        Self::default()
    }
    pub fn start(&mut self, task: Task) -> Result<(), Error> {
        if self.closed {
            return Err(Error::Closed);
        }
        let origin = task.origin().to_owned();
        if self
            .pending
            .iter()
            .any(|p| !p.discarded && p.origin == origin)
        {
            return Err(Error::Pending);
        }
        if self.pending.len() >= 4 {
            return Err(Error::Capacity);
        }
        if !self.has_origin_capacity(&origin) {
            return Err(Error::Capacity);
        }
        self.pending.try_reserve(1).map_err(|_| Error::Capacity)?;
        let runtime = Runtime::start(task).map_err(|_| Error::Start)?;
        self.deny_offline(&origin);
        self.pending.push(Pending {
            origin,
            runtime,
            discarded: false,
        });
        Ok(())
    }
    pub fn wait_handles(&self) -> impl Iterator<Item = BorrowedHandle<'_>> {
        self.pending.iter().filter_map(|p| p.runtime.wait_handle())
    }
    /// Local Loom already completed TLS and signed-device authorization; installation still obeys the same shutdown/capacity/credential boundaries.
    pub(crate) fn install(
        &mut self,
        origin: &str,
        session: crate::device_session_attempt::Session,
        now: u64,
    ) -> Result<(), Error> {
        let canonical =
            crate::device_session_attempt::canonical_origin(origin).map_err(|_| Error::Origin)?;
        let origin = canonical.as_str();
        if self.closed {
            return Err(Error::Closed);
        }
        session
            .token_for(origin, now)
            .map_err(|_| Error::Credential)?;
        if self
            .pending
            .iter()
            .any(|p| !p.discarded && p.origin == origin)
        {
            return Err(Error::Pending);
        }
        if !self.has_origin_capacity(origin) {
            return Err(Error::Capacity);
        }
        self.retiring
            .retain(|(_, grant)| grant.expires_at_ms() > now);
        // Automatic renewal does not prematurely interrupt in-flight projection; old leases retain their own expiry, and explicit revocation cancels both generations.
        let previous = self.sessions.iter().position(|(o, _)| o == origin);
        if previous.is_some() && self.retiring.len() >= 8 {
            return Err(Error::Capacity);
        }
        self.deny_offline(origin);
        if let Some(index) = previous {
            self.retiring.push(self.sessions.swap_remove(index));
        }
        self.sessions.push((origin.into(), Grant::new(session)));
        Ok(())
    }
    pub fn poll(&mut self) -> Vec<Event> {
        let mut events = Vec::new();
        let mut index = 0;
        while index < self.pending.len() {
            let Some(result) = self.pending[index].runtime.try_join() else {
                index += 1;
                continue;
            };
            let pending = self.pending.swap_remove(index);
            if pending.discarded || self.closed {
                continue;
            }
            let origin = pending.origin;
            match result {
                Ok(Output::OfflineCleared) => events.push(Event::OfflineCleared { origin }),
                Ok(Output::Pairing { device, changed }) => {
                    // Successful explicit re-pairing must revoke this origin's old authorization even if its ID is unchanged.
                    self.sessions.retain(|s| s.0 != origin);
                    self.retiring.retain(|s| s.0 != origin);
                    events.push(Event::Pairing {
                        origin,
                        device,
                        changed,
                    });
                }
                Ok(Output::Session(session)) => {
                    let device = session.device_id().to_owned();
                    self.sessions.retain(|s| s.0 != origin);
                    self.retiring.retain(|s| s.0 != origin);
                    self.sessions.push((origin.clone(), Grant::new(session)));
                    events.push(Event::Authorized { origin, device });
                }
                Err(error) => events.push(Event::Failed { origin, error }),
            }
        }
        events
    }
    pub fn token(&self, origin: &str, now_ms: u64) -> Option<&str> {
        if self.closed {
            return None;
        }
        let origin = crate::device_session_attempt::canonical_origin(origin).ok()?;
        self.sessions
            .iter()
            .find(|s| s.0 == origin)?
            .1
            .token_for(&origin, now_ms)
    }
    /// Hand background work an independent cancellation lease; never copy tokens into UI/message queues.
    pub fn lease(&mut self, origin: &str, now_ms: u64) -> Result<Lease, Error> {
        if self.closed {
            return Err(Error::Closed);
        }
        let origin =
            crate::device_session_attempt::canonical_origin(origin).map_err(|_| Error::Origin)?;
        self.sessions
            .iter_mut()
            .find(|s| s.0 == origin)
            .ok_or(Error::Credential)?
            .1
            .lease(&origin, now_ms)
            .map_err(|error| match error {
                crate::device_authorization_lease::Error::Capacity => Error::Capacity,
                _ => Error::Credential,
            })
    }
    /// Call on server revocation, connection switch, or consumer close. Only the same origin is affected; old requests are permanently marked for discard.
    pub fn invalidate(&mut self, origin: &str) -> Result<(), Error> {
        let origin =
            crate::device_session_attempt::canonical_origin(origin).map_err(|_| Error::Origin)?;
        self.deny_offline(&origin);
        self.sessions.retain(|s| s.0 != origin);
        self.retiring.retain(|s| s.0 != origin);
        for pending in self.pending.iter_mut().filter(|p| p.origin == origin) {
            pending.discarded = true;
            pending.runtime.request_stop();
        }
        Ok(())
    }
    pub fn close(&mut self) {
        self.closed = true;
        self.sessions.clear();
        self.retiring.clear();
        for pending in &mut self.pending {
            pending.discarded = true;
            pending.runtime.request_stop();
        }
    }
    pub fn finished(&self) -> bool {
        self.pending.is_empty()
    }
}
impl Drop for Owner {
    fn drop(&mut self) {
        // Normal application shutdown closes first and pumps messages until finished; Runtime Drop is only a blocking fallback.
        self.close();
    }
}
