//! Session media-publication gate; fixed state -> frames lock order prevents interleaving publication with close/authorization invalidation.
use crate::{
    live_relay_frame::Metadata,
    live_relay_pixels::PixelFrame,
    live_relay_viewer_frames::{Commit, Cursor, ViewerPixels},
    live_relay_viewer_identity::{self as authority, Identity, Snapshot},
};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
#[path = "live_relay_frame_notice.rs"]
mod notice;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Closed,
    Suspended,
    Epoch,
    StalePermit,
    Authority,
    Cache,
    Poisoned,
    Notice,
}
#[derive(Clone)]
pub struct Permit {
    owner: Arc<()>,
    generation: u64,
    epoch: u64,
    stop: Arc<AtomicBool>,
}
impl Permit {
    /// Only a worker/codec cancellation signal; publication authorization still requires same-lock commit revalidation.
    pub fn stop(&self) -> &AtomicBool {
        &self.stop
    }
    pub fn epoch(&self) -> u64 {
        self.epoch
    }
}
struct State {
    business: Option<Arc<crate::live_relay_viewer_business::Business>>,
    active: bool,
    closed: bool,
    generation: u64,
    frames: ViewerPixels,
    revision: u64,
    stop: Arc<AtomicBool>,
}
pub struct ViewerSession {
    owner: Arc<()>,
    identity: Identity,
    device: String,
    state: Mutex<State>,
    changed: notice::Notice,
}
impl ViewerSession {
    pub(super) fn identity(&self) -> &Identity {
        &self.identity
    }
    pub(crate) fn notify_input(&self) {
        self.changed.signal();
    }
    pub(super) fn matches_media_target(&self, session: &str, device: &str) -> bool {
        self.identity.session() == session && self.device == device
    }
    /// New owners always start paused; identity/device come from authenticated joining, never media packets.
    pub fn new(identity: Identity, device: String) -> Result<Self, Error> {
        if !authority::identifier(&device) {
            return Err(Error::Authority);
        }
        let frames = ViewerPixels::new(identity.epoch()).map_err(|_| Error::Epoch)?;
        Ok(Self {
            changed: notice::Notice::new().map_err(|_| Error::Notice)?,
            owner: Arc::new(()),
            identity,
            device,
            state: Mutex::new(State {
                business: None,
                active: false,
                closed: false,
                generation: 0,
                revision: 0,
                stop: Arc::new(AtomicBool::new(true)),
                frames,
            }),
        })
    }
    /// Only trusted control owners call after authentication, bootstrap replay, and business validation.
    /// This method validates membership/source contracts, not authentication; every activation revokes old work permits and discards old caches.
    pub fn activate(&self, snapshot: &Snapshot) -> Result<(), Error> {
        let mut state = self.state.lock().map_err(|_| Error::Poisoned)?;
        if state.closed {
            return Err(Error::Closed);
        }
        // Late snapshots cannot overwrite newer control state or unjustifiably revoke new-generation work.
        if snapshot.revision() < state.revision {
            return Err(Error::Authority);
        }
        self.invalidate(&mut state)?;
        if snapshot.renewal(&self.identity, &self.device).is_err() {
            state.closed = true;
            state.stop.store(true, Ordering::Release);
            state.frames.close();
            return Err(Error::Authority);
        }
        state.revision = snapshot.revision();
        state.active = true;
        Ok(())
    }
    fn invalidate(&self, state: &mut State) -> Result<(), Error> {
        state.business = None;
        state.active = false;
        state.stop.store(true, Ordering::Release);
        state.frames.close();
        self.changed.signal();
        let Some(next) = state.generation.checked_add(1) else {
            state.closed = true;
            return Err(Error::Closed);
        };
        state.generation = next;
        state.stop = Arc::new(AtomicBool::new(false));
        state.frames = ViewerPixels::new(self.identity.epoch()).map_err(|_| Error::Epoch)?;
        Ok(())
    }
    pub fn suspend(&self) -> Result<(), Error> {
        let mut state = self.state.lock().map_err(|_| Error::Poisoned)?;
        if state.closed {
            return Err(Error::Closed);
        }
        self.invalidate(&mut state)
    }
    /// Old-worker reclamation requests must not revoke a later-activated new generation.
    pub fn suspend_if_current(&self, permit: &Permit) -> Result<(), Error> {
        let mut state = self.state.lock().map_err(|_| Error::Poisoned)?;
        self.verify(&state, permit)?;
        self.invalidate(&mut state)
    }
    pub fn admit(&self, epoch: u64) -> Result<Permit, Error> {
        let state = self.state.lock().map_err(|_| Error::Poisoned)?;
        Self::ready(&state)?;
        if epoch != self.identity.epoch() {
            return Err(Error::Epoch);
        }
        Ok(Permit {
            owner: self.owner.clone(),
            generation: state.generation,
            epoch,
            stop: state.stop.clone(),
        })
    }
    pub fn commit(
        &self,
        permit: &Permit,
        frame: PixelFrame,
        allow_warmup: bool,
    ) -> Result<Commit, Error> {
        let state = self.state.lock().map_err(|_| Error::Poisoned)?;
        self.verify(&state, permit)?;
        if frame.metadata().epoch != permit.epoch {
            return Err(Error::Epoch);
        }
        // Hold the gate lock until cache commit completes; no decoding, network I/O, or worker waits under this lock.
        let result = state
            .frames
            .commit(frame, allow_warmup)
            .map_err(|_| Error::Cache)?;
        if result == Commit::Accepted {
            self.changed.signal();
        }
        Ok(result)
    }
    /// Recheck fixed permits before worker execution/control sending; readmission cannot bypass generation invalidation.
    pub fn validate(&self, permit: &Permit) -> Result<(), Error> {
        let state = self.state.lock().map_err(|_| Error::Poisoned)?;
        self.verify(&state, permit)
    }
    /// Recovery reads only actually committed pixel watermarks; UI cache consumption preserves them, and wire/queued data cannot impersonate decode success.
    pub(super) fn committed(&self, permit: &Permit) -> Result<Cursor, Error> {
        let state = self.state.lock().map_err(|_| Error::Poisoned)?;
        self.verify(&state, permit)?;
        state
            .frames
            .status()
            .map(|status| status.cursor)
            .map_err(|_| Error::Cache)
    }
    /// Control progress for the same permit updates only revision, not frame caches; old control owners cannot revive new generations.
    pub(super) fn advance_control(&self, permit: &Permit, revision: u64) -> Result<(), Error> {
        let mut state = self.state.lock().map_err(|_| Error::Poisoned)?;
        self.verify(&state, permit)?;
        if revision < state.revision {
            return Err(Error::Authority);
        }
        state.revision = revision;
        Ok(())
    }
    fn verify(&self, state: &State, permit: &Permit) -> Result<(), Error> {
        Self::ready(state)?;
        if !Arc::ptr_eq(&self.owner, &permit.owner) || state.generation != permit.generation {
            return Err(Error::StalePermit);
        }
        Ok(())
    }
    pub fn latest_after(&self, cursor: Cursor) -> Result<Option<Metadata>, Error> {
        let state = self.state.lock().map_err(|_| Error::Poisoned)?;
        Self::ready(&state)?;
        state.frames.latest_after(cursor).map_err(|_| Error::Cache)
    }
    pub fn take(&self, cursor: Cursor) -> Result<Option<PixelFrame>, Error> {
        let state = self.state.lock().map_err(|_| Error::Poisoned)?;
        Self::ready(&state)?;
        state.frames.take(cursor).map_err(|_| Error::Cache)
    }
    /// One UI consumer waits on this coalesced event and calls poll_frame; the handle cannot outlive Session.
    #[cfg(windows)]
    pub fn change_handle(&self) -> std::os::windows::io::BorrowedHandle<'_> {
        self.changed.handle()
    }
    /// Clear notification/take the latest frame under one lock; pause/close also wakes consumers, which must remove old display resources.
    pub fn poll_frame(&self, after: Cursor) -> Result<Option<PixelFrame>, Error> {
        self.poll_update(after).map(|update| update.frame)
    }
    /// Get frame/business snapshot and acknowledge notification within one lock, avoiding lost wakeups from interleaved business updates/reset.
    pub fn poll_update(&self, after: Cursor) -> Result<Update, Error> {
        let state = self.state.lock().map_err(|_| Error::Poisoned)?;
        self.changed.reset();
        Self::ready(&state)?;
        let frame = match state.frames.latest_after(after).map_err(|_| Error::Cache)? {
            Some(metadata) => state
                .frames
                .take(metadata.into())
                .map_err(|_| Error::Cache)?,
            None => None,
        };
        Ok(Update {
            frame,
            business: state.business.clone(),
        })
    }
    pub(super) fn publish_business(
        &self,
        permit: &Permit,
        business: Arc<crate::live_relay_viewer_business::Business>,
    ) -> Result<(), Error> {
        let mut state = self.state.lock().map_err(|_| Error::Poisoned)?;
        self.verify(&state, permit)?;
        state.business = Some(business);
        self.changed.signal();
        Ok(())
    }
    fn ready(state: &State) -> Result<(), Error> {
        if state.closed {
            Err(Error::Closed)
        } else if !state.active {
            Err(Error::Suspended)
        } else {
            Ok(())
        }
    }
    pub fn close(&self) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        state.active = false;
        state.closed = true;
        state.business = None;
        state.stop.store(true, Ordering::Release);
        state.frames.close();
        self.changed.signal();
    }
    /// Handle only authoritative revocation of the current media connection; old connections cannot close later-activated generations.
    pub fn close_if_current(&self, permit: &Permit) -> Result<(), Error> {
        let mut state = self.state.lock().map_err(|_| Error::Poisoned)?;
        self.verify(&state, permit)?;
        state.active = false;
        state.closed = true;
        state.business = None;
        state.stop.store(true, Ordering::Release);
        state.frames.close();
        self.changed.signal();
        Ok(())
    }
    /// Late open indicates only an open remote snapshot, not local-gate activation. HTTP errors cannot be submitted as closure evidence.
    pub fn observe_terminal(&self, bytes: &[u8]) -> Result<bool, Error> {
        let closed = authority::terminal_snapshot(bytes, &self.identity, &self.device)
            .map_err(|_| Error::Authority)?;
        let mut state = self.state.lock().map_err(|_| Error::Poisoned)?;
        if closed {
            state.active = false;
            state.closed = true;
            state.business = None;
            state.stop.store(true, Ordering::Release);
            state.frames.close();
            self.changed.signal();
        }
        Ok(state.closed)
    }
}
pub struct Update {
    pub frame: Option<PixelFrame>,
    pub business: Option<Arc<crate::live_relay_viewer_business::Business>>,
}
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-live")))]
#[path = "live_relay_viewer_session_tests.rs"]
mod tests;
