//! Nonblocking local Art owner for UI: hand off only Prepared/commands and claim frames; Local and child always belong to the background thread.
//! Normal cleanup stops first, waits for thread-handle termination, then calls take_exit; Drop's join is only the final leak-prevention fallback.
use super::{Frame, Local};
use crate::{
    art_execution_grant::wire::Binding, device_authorization_lease::Lease,
    live_relay_control_runtime::Slot,
};
use std::{
    io,
    os::windows::io::{AsHandle, BorrowedHandle},
    sync::Arc,
    thread::JoinHandle,
    time::Instant,
};
#[path = "surface_javascript_signed_mailbox.rs"]
mod mailbox;
#[path = "surface_javascript_signed_thread.rs"]
mod worker;
use crate::live_relay_http::art_prepare::service::progress;
pub(crate) use mailbox::Operation;
use mailbox::{Mailbox, Reply};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Mount(crate::live_relay_http::art_prepare::Error),
    Local(super::Error),
    Io(io::ErrorKind),
    Stopped,
    Panicked,
}
fn io_error(error: io::Error) -> Error {
    Error::Io(error.kind())
}
pub struct Runtime {
    binding: Binding,
    lease: Arc<Lease>,
    deadline: (u64, Instant),
    mailbox: Arc<Mailbox>,
    thread: Option<JoinHandle<Result<(), Error>>>,
    slot: Option<Slot>,
    pending: bool,
    sequence: u64,
    progress: Option<Arc<progress::Channel>>,
    trigger: Option<crate::art_live_trigger::ticket::Ticket>,
}
/// Immutable host-prepared inputs; deliver only through the original mount-resource protocol without adding script file/network permissions.
pub struct Content {
    pub snapshot: serde_json::Value,
    pub resources: serde_json::Value,
    pub images: Vec<super::super::ImageResource>,
}
impl Content {
    pub fn empty(snapshot: serde_json::Value) -> Self {
        Self {
            snapshot,
            resources: serde_json::json!({}),
            images: Vec::new(),
        }
    }
}
impl Runtime {
    /// reachability mode is fixed within this owner; host network changes must stop it rather than silently extend offline permission.
    pub fn start(
        prepared: crate::live_relay_http::art_prepare::Prepared,
        view: crate::surface_live_binding::Binding,
        snapshot: serde_json::Value,
        online: bool,
    ) -> io::Result<Self> {
        Self::start_notified(prepared, view, snapshot, online, None, Arc::new(|| {}))
    }
    /// notify must be brief and nonblocking, posting only host Wake; publish completion before the final notification.
    pub fn start_notified(
        prepared: crate::live_relay_http::art_prepare::Prepared,
        view: crate::surface_live_binding::Binding,
        snapshot: serde_json::Value,
        online: bool,
        viewport: Option<crate::surface_viewport::Viewport>,
        notify: Arc<dyn Fn() + Send + Sync>,
    ) -> io::Result<Self> {
        Self::start_content_notified(
            prepared,
            view,
            Content::empty(snapshot),
            online,
            viewport,
            notify,
        )
    }
    pub fn start_content_notified(
        prepared: crate::live_relay_http::art_prepare::Prepared,
        view: crate::surface_live_binding::Binding,
        content: Content,
        online: bool,
        viewport: Option<crate::surface_viewport::Viewport>,
        notify: Arc<dyn Fn() + Send + Sync>,
    ) -> io::Result<Self> {
        let lease = prepared.lease();
        let binding = prepared.binding().clone();
        let deadline = prepared.deadline(online);
        let progress = Arc::new(progress::Channel::new(notify.clone()));
        let observer = progress.clone();
        let mut runtime =
            Self::spawn_notified(binding, lease, deadline, online, notify, move || {
                prepared
                    .mount_content_observed(
                        view,
                        content,
                        online,
                        viewport,
                        Some(observer),
                        &|| false,
                    )
                    .map_err(Error::Mount)
            })?;
        runtime.progress = Some(progress);
        Ok(runtime)
    }
    #[cfg(test)]
    fn spawn(
        binding: Binding,
        lease: Arc<Lease>,
        deadline: (u64, Instant),
        online: bool,
        mount: impl FnOnce() -> Result<(Local, Frame), Error> + Send + 'static,
    ) -> io::Result<Self> {
        Self::spawn_notified(binding, lease, deadline, online, Arc::new(|| {}), mount)
    }
    fn spawn_notified(
        binding: Binding,
        lease: Arc<Lease>,
        deadline: (u64, Instant),
        online: bool,
        notify: Arc<dyn Fn() + Send + Sync>,
        mount: impl FnOnce() -> Result<(Local, Frame), Error> + Send + 'static,
    ) -> io::Result<Self> {
        let slot = Slot::acquire()?;
        let mailbox = Arc::new(Mailbox::new(notify)?);
        let worker_mailbox = mailbox.clone();
        let worker_lease = lease.clone();
        let current = binding.clone();
        let thread = std::thread::Builder::new()
            .name("otter-signed-art".into())
            .spawn(move || {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    worker::run(
                        mount,
                        &current,
                        &worker_lease,
                        deadline,
                        online,
                        &worker_mailbox,
                    )
                }))
                .unwrap_or(Err(Error::Panicked));
                // Local/child are destroyed before exit; neither thread termination nor revocation may leave claimable stale frames.
                worker_lease.revoke();
                worker_mailbox.finish();
                result
            })?;
        Ok(Self {
            binding,
            lease,
            deadline,
            mailbox,
            thread: Some(thread),
            slot: Some(slot),
            pending: true,
            sequence: 1,
            progress: None,
            trigger: None,
        })
    }
    pub fn binding(&self) -> &Binding {
        &self.binding
    }
    pub fn pending(&self) -> bool {
        self.pending
    }
    /// Submit through the original single slot; callers must still verify view belongs to the current declaration, with no animation queuing or implicit retry while busy.
    pub fn select_view(
        &mut self,
        current: &Binding,
        view_id: &str,
        viewport: crate::surface_viewport::Viewport,
    ) -> Result<(), Error> {
        let view_id = crate::art_surface_views::Id::parse(view_id)
            .ok_or(Error::Io(io::ErrorKind::InvalidInput))?;
        self.submit(current, Operation::SelectView(view_id, viewport))
    }
    pub(crate) fn take_progress(&self, current: &Binding) -> Option<progress::Update> {
        let progress = self.progress.as_ref()?;
        if !self.pending || self.validate(current).is_err() {
            progress.clear();
            return None;
        }
        progress.take()
    }
    pub fn finished(&self) -> bool {
        self.thread.is_none()
    }
    pub(crate) fn take_confirmation(
        &self,
        current: &Binding,
    ) -> Option<Arc<crate::live_relay_http::art_prepare::service::confirmation::Ticket>> {
        let progress = self.progress.as_ref()?;
        if !self.pending {
            return None;
        }
        if self.validate(current).is_err() {
            progress.confirmations.close();
            return None;
        }
        progress.confirmations.take()
    }
    pub fn wait_handles(&self) -> impl Iterator<Item = BorrowedHandle<'_>> {
        [
            self.thread.as_ref().map(AsHandle::as_handle),
            self.thread
                .as_ref()
                .filter(|_| !self.lease.execution_stopped())
                .map(|_| self.mailbox.reply_handle()),
        ]
        .into_iter()
        .flatten()
    }
    pub(crate) fn validate(&self, current: &Binding) -> Result<(), Error> {
        let observed = self.mailbox.observed_wall();
        let now = super::now().map_err(Error::Local)?;
        if current != &self.binding
            || self
                .trigger
                .as_ref()
                .is_some_and(|ticket| !ticket.current(current))
            || self.lease.execution_stopped()
            || now < observed
            || now >= self.deadline.0
            || Instant::now() >= self.deadline.1
            || self.mailbox.completed()
            || self.thread.as_ref().is_none_or(JoinHandle::is_finished)
        {
            return Err(Error::Stopped);
        }
        self.lease
            .validate(&current.origin, now)
            .map_err(|_| Error::Stopped)?;
        self.mailbox.observe(now);
        Ok(())
    }
    pub(crate) fn submit(&mut self, current: &Binding, operation: Operation) -> Result<(), Error> {
        if let Err(error) = self.validate(current) {
            self.request_stop();
            return Err(error);
        }
        if self.pending {
            return Err(Error::Io(io::ErrorKind::WouldBlock));
        }
        if let Operation::ClickAt(target, _) = &operation {
            if target.is_empty() || target.len() > 256 {
                return Err(Error::Io(io::ErrorKind::InvalidInput));
            }
        }
        #[cfg(test)]
        if let Operation::Click(target) = &operation {
            if target.is_empty() || target.len() > 256 {
                return Err(Error::Io(io::ErrorKind::InvalidInput));
            }
        }
        match &operation {
            Operation::Viewport(viewport)
            | Operation::SelectView(_, viewport)
            | Operation::Animation(viewport)
            | Operation::Timer(viewport)
            | Operation::ClickAt(_, viewport)
            | Operation::Input(_, viewport) => {
                viewport
                    .validate()
                    .map_err(|_| Error::Io(io::ErrorKind::InvalidInput))?;
            }
            _ => {}
        }
        if let Operation::LiveTrigger(ticket, viewport) = &operation {
            if !ticket.current(current) || viewport.validate().is_err() {
                return Err(Error::Stopped);
            }
        }
        if let Operation::Input(input, _) = &operation {
            input.validate().map_err(|error| Error::Io(error.kind()))?;
        }
        let sequence = self.sequence.checked_add(1).ok_or(Error::Stopped)?;
        if let Some(progress) = &self.progress {
            progress.clear();
        }
        let trigger = match &operation {
            Operation::LiveTrigger(ticket, _) => Some(ticket.clone()),
            _ => None,
        };
        if let Err(error) = self.mailbox.submit(sequence, operation) {
            self.request_stop();
            return Err(io_error(error));
        }
        self.sequence = sequence;
        self.trigger = trigger;
        self.pending = true;
        Ok(())
    }
    /// Hosts must also recheck node/selection/window identities before and after HWND submission; this method checks only execution binding and original authorization.
    pub fn take_reply(&mut self, current: &Binding) -> Option<Result<Frame, Error>> {
        let reply = match self.mailbox.take_reply() {
            Ok(Some(reply)) => reply,
            Ok(None) => return None,
            Err(error) => {
                self.request_stop();
                return Some(Err(io_error(error)));
            }
        };
        self.pending = false;
        if let Some(progress) = &self.progress {
            progress.clear();
        }
        let result = self.validate(current).and_then(|()| {
            if reply.sequence != self.sequence {
                Err(Error::Stopped)
            } else {
                Ok(reply.frame)
            }
        });
        self.trigger = None;
        if result.is_err() {
            self.request_stop();
        }
        Some(result)
    }
    pub fn request_stop(&self) {
        if let Some(progress) = &self.progress {
            progress.close();
        }
        self.lease.stop_execution();
    }
    pub fn take_exit(&mut self) -> Option<Result<(), Error>> {
        // completed means Local/child have been reaped; join waits at most for the brief notification to return, never networking/JS.
        if !self.mailbox.completed() && !self.thread.as_ref()?.is_finished() {
            return None;
        }
        self.request_stop();
        self.slot.take();
        self.pending = false;
        Some(self.thread.take()?.join().unwrap_or(Err(Error::Panicked)))
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
#[cfg(test)]
#[path = "surface_javascript_signed_runtime_tests.rs"]
pub(crate) mod tests;
