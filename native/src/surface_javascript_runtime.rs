//! Nonblocking JS owner for UI: each unconsumed operation occupies one slot; completed replies and thread termination wake separately.
//! Normal closure waits for take_exit before removing the owner; Drop is only the final leak-prevention fallback.
use super::{
    prepare::{prepare, Connection},
    *,
};
use crate::{device_authorization_config::Config, live_relay_control_runtime::Slot};
use std::{
    os::windows::io::{AsHandle, BorrowedHandle},
    thread::JoinHandle,
};
#[path = "surface_javascript_runtime_mailbox.rs"]
mod mailbox;
use mailbox::{Operation, Reply, Shared};
#[path = "surface_javascript_runtime_thread.rs"]
mod worker;
pub struct Runtime {
    lease: Arc<Lease>,
    shared: Arc<Shared>,
    thread: Option<JoinHandle<Result<(), Error>>>,
    slot: Option<Slot>,
    pending: bool,
    sequence: u64,
}
impl Runtime {
    pub fn start(
        config: Config,
        lease: Lease,
        destination: &Lease,
        current: MountedSurface,
        viewport: crate::surface_viewport::Viewport,
    ) -> io::Result<Self> {
        Self::spawn(
            Connection::Configured(config),
            lease,
            destination,
            current,
            Some(viewport),
        )
    }
    #[cfg(test)]
    pub(crate) fn start_prepared(
        client: crate::device_session_client::Client,
        lease: Lease,
        destination: &Lease,
        current: MountedSurface,
    ) -> io::Result<Self> {
        Self::spawn(
            Connection::Prepared(client),
            lease,
            destination,
            current,
            None,
        )
    }
    fn spawn(
        connection: Connection,
        lease: Lease,
        destination: &Lease,
        current: MountedSurface,
        viewport: Option<crate::surface_viewport::Viewport>,
    ) -> io::Result<Self> {
        let time = now().map_err(|_| io::ErrorKind::PermissionDenied)?;
        if !lease.same_grant(destination)
            || destination
                .validate(current.binding.origin(), time)
                .is_err()
        {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        let slot = Slot::acquire()?;
        let lease = Arc::new(lease);
        let shared = Arc::new(Shared::new(lease.expires_at_ms())?);
        let task = lease.clone();
        let mailbox = shared.clone();
        let thread = std::thread::Builder::new()
            .name("otter-surface-javascript".into())
            .spawn(move || {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    worker::run(connection, task.clone(), current, viewport, &mailbox)
                }))
                .unwrap_or(Err(Error::Panicked));
                // Mounted/child are reaped before thread completion; the UI must not treat invalid replies as deliverable results.
                task.revoke();
                result
            })?;
        Ok(Self {
            lease,
            shared,
            thread: Some(thread),
            slot: Some(slot),
            pending: true,
            sequence: 1,
        })
    }
    pub fn pending(&self) -> bool {
        self.pending
    }
    pub fn reply_ready(&self) -> bool {
        self.shared.has_reply()
    }
    pub(crate) fn hold_notification(&self, hold: bool) -> io::Result<()> {
        self.shared.hold_notification(hold)
    }
    pub fn finished(&self) -> bool {
        self.thread.is_none()
    }
    pub fn expires_at_ms(&self) -> u64 {
        self.shared.expires()
    }
    pub fn wait_handles(&self) -> impl Iterator<Item = BorrowedHandle<'_>> {
        [
            self.thread.as_ref().map(AsHandle::as_handle),
            self.thread.as_ref().map(|_| self.shared.reply_handle()),
        ]
        .into_iter()
        .flatten()
    }
    #[cfg(test)]
    pub(crate) fn wait_reply_for_test(&mut self) -> Result<(), Error> {
        use std::os::windows::io::AsRawHandle;
        use windows::Win32::{Foundation::*, System::Threading::WaitForMultipleObjects};
        // Successful replies are not termination notifications. Error exits must also wake tests and preserve original errors rather than waiting eight seconds.
        // Thread first: when both are ready, termination wins as in production settle_javascript, and stale replies are rejected.
        let handles: Vec<_> = self
            .wait_handles()
            .map(|h| HANDLE(h.as_raw_handle()))
            .collect();
        if handles.len() != 2 {
            return Err(Error::Stopped);
        }
        match unsafe { WaitForMultipleObjects(&handles, false, 8000) } {
            WAIT_OBJECT_0 => self
                .take_exit()
                .unwrap_or(Err(Error::Panicked))
                .and(Err(Error::Stopped)),
            result if result.0 == WAIT_OBJECT_0.0 + 1 => Ok(()),
            WAIT_TIMEOUT => Err(Error::Execution(io::ErrorKind::TimedOut)),
            _ => Err(Error::Execution(io::ErrorKind::Other)),
        }
    }
    /// Queue only lifecycle synchronization for the current snapshot; no JS/HTTP execution or background waiting.
    pub fn rebind(&mut self, current: MountedSurface) -> io::Result<()> {
        self.submit(current, Operation::Rebind)
    }
    /// HWND-verified targets enter the same background owner; listeners never execute in the UI.
    pub fn click(
        &mut self,
        current: MountedSurface,
        target: String,
        viewport: crate::surface_viewport::Viewport,
    ) -> io::Result<()> {
        if !crate::surface_javascript_event::safe(&target) {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        viewport.validate().map_err(io::Error::other)?;
        self.submit(current, Operation::Click(target, viewport))
    }
    /// Convergence snapshots still pass original entry/lease validation; execute in the same child without refetching or rebuilding scripts.
    pub fn update(
        &mut self,
        current: MountedSurface,
        viewport: crate::surface_viewport::Viewport,
    ) -> io::Result<()> {
        viewport.validate().map_err(io::Error::other)?;
        self.submit(current, Operation::Update(viewport))
    }
    pub(crate) fn local_input(
        &mut self,
        current: MountedSurface,
        input: crate::surface_javascript_wire::LocalInput,
        viewport: crate::surface_viewport::Viewport,
    ) -> io::Result<()> {
        input.validate()?;
        viewport.validate().map_err(io::Error::other)?;
        self.submit(current, Operation::LocalInput(input, viewport))
    }
    fn submit(&mut self, current: MountedSurface, operation: Operation) -> io::Result<()> {
        if current.snapshot.get().len() > crate::surface_scene::MAX_BYTES {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        if self.pending || self.finished() || self.lease.is_revoked() {
            return Err(io::ErrorKind::WouldBlock.into());
        }
        let next = self
            .sequence
            .checked_add(1)
            .ok_or(io::ErrorKind::InvalidData)?;
        self.shared.submit(next, current, operation)?;
        self.sequence = next;
        self.pending = true;
        Ok(())
    }
    pub fn animation_frame(
        &mut self,
        current: MountedSurface,
        viewport: crate::surface_viewport::Viewport,
    ) -> io::Result<()> {
        viewport.validate().map_err(io::Error::other)?;
        self.submit(current, Operation::AnimationFrame(viewport))
    }
    pub fn timer_tick(
        &mut self,
        current: MountedSurface,
        viewport: crate::surface_viewport::Viewport,
    ) -> io::Result<()> {
        viewport.validate().map_err(io::Error::other)?;
        self.submit(current, Operation::TimerTick(viewport))
    }
    /// Initial frame/ACK completion still requires checking the original Grant and current UI snapshot; no cross-generation or expired delivery.
    pub fn take_reply(
        &mut self,
        destination: &Lease,
        current: &MountedSurface,
    ) -> Option<Result<Option<Frame>, Error>> {
        let result = self.shared.take_reply();
        match result {
            Ok(None) => None,
            Err(error) => Some(Err(execution(error))),
            Ok(Some(Reply {
                sequence,
                binding,
                revision,
                frame,
                snapshot,
            })) => {
                self.pending = false;
                self.slot.take();
                let valid = (|| {
                    let time = now()?;
                    destination
                        .validate(current.binding.origin(), time)
                        .map_err(|_| Error::Stopped)?;
                    self.lease
                        .validate(current.binding.origin(), time)
                        .map_err(|_| Error::Stopped)?;
                    if !self.lease.same_grant(destination)
                        || sequence != self.sequence
                        || binding != current.binding
                        || snapshot.get() != current.snapshot.get()
                        || time >= self.expires_at_ms()
                        || current
                            .javascript_entry(time)
                            .map_err(Error::Source)?
                            .revision()
                            != revision
                    {
                        return Err(Error::Stopped);
                    }
                    Ok(frame)
                })();
                if valid.is_err() {
                    self.request_stop();
                }
                Some(valid)
            }
        }
    }
    pub fn request_stop(&self) {
        self.lease.revoke();
    }
    pub fn take_exit(&mut self) -> Option<Result<(), Error>> {
        if !self.thread.as_ref()?.is_finished() {
            return None;
        }
        self.request_stop();
        self.slot.take();
        self.pending = false;
        Some(self.thread.take()?.join().unwrap_or(Err(Error::Panicked)))
    }
    #[cfg(test)]
    pub(crate) fn probe(&self) -> Option<Arc<crate::bounded_worker_process::Probe>> {
        self.shared.probe.lock().unwrap().clone()
    }
    #[cfg(test)]
    pub(crate) fn fail_next_click_wake_for_test(&self) {
        self.shared
            .fail_click_wake
            .store(true, std::sync::atomic::Ordering::Release);
    }
}
impl Drop for Runtime {
    fn drop(&mut self) {
        self.request_stop();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
