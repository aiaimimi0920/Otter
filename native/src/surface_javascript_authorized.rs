//! JS sessions holding the original Grant; Job or persistent Runtime background owners execute blocking operations.
//! This layer owns neither remote-action owners nor FIFO; valid returned emits must still pass existing application submission gates.
use super::{Frame, Session};
use crate::{
    device_authorization_lease::{Lease, Notification},
    live_relay_http::{JavaScriptEntry, MountedSurface, ResourceError},
    surface_live_binding::Lifecycle,
};
use std::{io, os::windows::io::BorrowedHandle, sync::Arc};
#[path = "surface_javascript_mount_job.rs"]
mod job;
pub use job::{Job, Ready};
#[path = "surface_javascript_prepare.rs"]
mod prepare;
#[path = "surface_javascript_runtime.rs"]
mod runtime;
pub use runtime::Runtime;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Stopped,
    Source(ResourceError),
    Inactive,
    Execution(io::ErrorKind),
    Panicked,
}
pub struct Mounted {
    lease: Arc<Lease>,
    notice: Notification,
    entry: JavaScriptEntry,
    session: Option<Session>,
    initial: Option<Frame>,
}
pub(super) fn now() -> Result<u64, Error> {
    let time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| Error::Stopped)?;
    u64::try_from(time.as_millis()).map_err(|_| Error::Stopped)
}
fn stopped(lease: &Lease, expires: u64) -> bool {
    lease.is_revoked() || now().map_or(true, |now| now >= expires.min(lease.expires_at_ms()))
}
fn execution(error: io::Error) -> Error {
    Error::Execution(error.kind())
}
impl Mounted {
    pub fn expires_at_ms(&self) -> u64 {
        self.entry.expires_at_ms().min(self.lease.expires_at_ms())
    }
    pub fn revocation_handle(&self) -> BorrowedHandle<'_> {
        self.notice.handle()
    }
    pub fn is_alive(&self) -> bool {
        self.session.is_some()
    }
    fn fail<T>(&mut self, error: Error) -> Result<T, Error> {
        self.initial.take();
        self.session.take();
        Err(error)
    }
    fn validate(&mut self, current: &MountedSurface, update: bool) -> Result<(), Error> {
        let result = (|| {
            let now = now()?;
            self.lease
                .validate(current.binding.origin(), now)
                .map_err(|_| Error::Stopped)?;
            if current.binding.device() != self.lease.device_id()
                || current.binding.lifecycle() == Lifecycle::Disposed
            {
                return Err(Error::Stopped);
            }
            let next = self
                .entry
                .current(current, now, update)
                .map_err(Error::Source)?;
            let session = self.session.as_mut().ok_or(Error::Stopped)?;
            if !update && next.revision() != session.revision {
                return Err(Error::Stopped);
            }
            // Lifecycle ACKs may advance binding, but cannot change instance/generation or advance snapshot revision through it.
            session.binding = current.binding.clone();
            self.entry = next;
            Ok(())
        })();
        match result {
            Ok(()) => Ok(()),
            Err(error) => self.fail(error),
        }
    }
    pub fn refresh(&mut self, current: &MountedSurface) -> Result<(), Error> {
        self.validate(current, false)
    }
    pub fn take_initial(&mut self, current: &MountedSurface) -> Result<Frame, Error> {
        self.validate(current, false)?;
        let frame = self.initial.take().ok_or(Error::Stopped)?;
        if !frame.events.is_empty() && current.binding.lifecycle() != Lifecycle::Active {
            return self.fail(Error::Inactive);
        }
        // Lifecycle ACKs may arrive while Ready waits; initial emits' immutable identities cannot be rebound through them.
        if frame.events.iter().any(|event| {
            event.binding() != &current.binding || event.revision() != self.entry.revision()
        }) {
            return self.fail(Error::Stopped);
        }
        Ok(frame)
    }
    fn operate(
        &mut self,
        current: &MountedSurface,
        update: bool,
        execute: impl FnOnce(&mut Session, &Lease, u64) -> io::Result<Frame>,
    ) -> Result<Frame, Error> {
        self.validate(current, update)?;
        // Deliver the initial frame, including emits, to the owner first; update/click must not overtake it and later retrieve an old frame.
        if self.initial.is_some() {
            return self.fail(Error::Stopped);
        }
        let result = execute(
            self.session.as_mut().ok_or(Error::Stopped)?,
            &self.lease,
            self.entry.expires_at_ms(),
        );
        let frame = match result {
            Ok(frame) => frame,
            Err(error) => return self.fail(execution(error)),
        };
        self.validate(current, false)?;
        if !frame.events.is_empty() && current.binding.lifecycle() != Lifecycle::Active {
            return self.fail(Error::Inactive);
        }
        Ok(frame)
    }
    pub fn click(&mut self, current: &MountedSurface, target: &str) -> Result<Frame, Error> {
        self.validate(current, false)?;
        if current.binding.lifecycle() != Lifecycle::Active {
            return self.fail(Error::Inactive);
        }
        self.operate(current, false, |session, lease, expires| {
            session.click(target, &|| stopped(lease, expires))
        })
    }
    pub fn update(&mut self, current: &MountedSurface) -> Result<Frame, Error> {
        self.operate(current, true, |session, lease, expires| {
            let value =
                serde_json::from_str(current.snapshot.get()).map_err(|_| super::wire::invalid())?;
            session.update(value, serde_json::json!({}), &|| stopped(lease, expires))
        })
    }
    pub(crate) fn local_input(
        &mut self,
        current: &MountedSurface,
        input: super::wire::LocalInput,
    ) -> Result<Frame, Error> {
        self.validate(current, false)?;
        if current.binding.lifecycle() != Lifecycle::Active {
            return self.fail(Error::Inactive);
        }
        self.operate(current, false, |session, lease, expires| {
            session.local_input(input, &|| stopped(lease, expires))
        })
    }
    pub fn animation_frame(&mut self, current: &MountedSurface) -> Result<Frame, Error> {
        self.validate(current, false)?;
        if current.binding.lifecycle() != Lifecycle::Active {
            return self.fail(Error::Inactive);
        }
        self.operate(current, false, |session, lease, expires| {
            session.animation_frame(&|| stopped(lease, expires))
        })
    }
    pub fn suspend(&mut self, current: &MountedSurface) -> Result<Frame, Error> {
        self.operate(current, false, |session, lease, expires| {
            session.suspend(&|| stopped(lease, expires))
        })
    }
    pub fn timer_tick(&mut self, current: &MountedSurface) -> Result<Frame, Error> {
        self.validate(current, false)?;
        if current.binding.lifecycle() != Lifecycle::Active {
            return self.fail(Error::Inactive);
        }
        self.operate(current, false, |session, lease, expires| {
            session.timer_tick(&|| stopped(lease, expires))
        })
    }
    pub fn resume(&mut self, current: &MountedSurface) -> Result<Frame, Error> {
        self.validate(current, false)?;
        if current.binding.lifecycle() != Lifecycle::Active {
            return self.fail(Error::Inactive);
        }
        self.operate(current, false, |session, lease, expires| {
            session.resume(&|| stopped(lease, expires))
        })
    }
    pub fn dispose(mut self, current: &MountedSurface) -> Result<(), Error> {
        self.validate(current, false)?;
        self.session
            .take()
            .ok_or(Error::Stopped)?
            .dispose(&|| stopped(&self.lease, self.entry.expires_at_ms()))
            .map_err(execution)
    }
    /// Revocation/window closure does not run untrusted cleanup; reap only owned children without replaying remote requests.
    pub fn request_stop(&mut self) {
        self.lease.revoke();
        let _ = self.fail::<()>(Error::Stopped);
    }
    #[cfg(test)]
    pub(crate) fn probe(&self) -> crate::bounded_worker_process::Probe {
        self.session
            .as_ref()
            .unwrap()
            .child
            .as_ref()
            .unwrap()
            .probe()
            .unwrap()
    }
}
