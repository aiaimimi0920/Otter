//! Local-preview seam protected by new execution permits; reuse real JS children, not old Surface resource leases as permission.
//! Blocking APIs are background-owner-only. Currently connects local state/previews; upper layers still own service dispatch, committed results, and UI wiring.
//! Host lifecycle invalidation must enter cancelled; grant Binding lacks window/attachment lifecycle and cannot replace its owner.
use super::{Frame, Session};
use crate::{
    art_execution_grant::{self as grant, wire, Expected, Issuer, Permit},
    device_authorization_lease::Lease,
    surface_live_binding::Binding,
};
use std::{
    cell::RefCell,
    io,
    sync::{Arc, Mutex},
};
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-surface")))]
#[path = "surface_javascript_signed_result_tests.rs"]
mod result_tests;
#[path = "surface_javascript_signed_runtime.rs"]
pub mod runtime;
#[path = "surface_javascript_signed_view.rs"]
mod view;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Grant(grant::Error),
    Lease,
    Binding,
    Unsupported,
    Stopped,
    Execution(io::ErrorKind),
}
/// Both identities come separately from the current host request; never fabricate Expected by working backward from signed responses.
pub struct Input<'a> {
    pub envelope: &'a [u8],
    pub issuer: Issuer<'a>,
    pub expected: Expected<'a>,
    pub source: &'a [u8],
    pub view: Binding,
    pub snapshot: serde_json::Value,
    pub resources: serde_json::Value,
    pub images: Vec<super::ImageResource>,
    pub viewport: Option<crate::surface_viewport::Viewport>,
}
pub struct Local {
    permit: Arc<Mutex<Permit>>,
    lease: Arc<Lease>,
    session: Option<Session>,
}
pub(crate) enum Interaction {
    Input(crate::surface_javascript_wire::LocalInput),
    Click(String),
}
fn now() -> Result<u64, Error> {
    super::authorized::now().map_err(|_| Error::Stopped)
}
fn check(
    permit: &Mutex<Permit>,
    lease: &Lease,
    current: &wire::Binding,
    online: bool,
    cancelled: &impl Fn() -> bool,
) -> Result<(), Error> {
    let result = (|| {
        let time = now()?;
        if cancelled() {
            return Err(Error::Stopped);
        }
        lease
            .validate(&current.origin, time)
            .map_err(|_| Error::Lease)?;
        if lease.execution_stopped() {
            return Err(Error::Stopped);
        }
        if lease.device_id() != current.device_id {
            return Err(Error::Binding);
        }
        permit
            .lock()
            .map_err(|_| Error::Stopped)?
            .authorize(current, time, online, lease.is_revoked())
            .map_err(Error::Grant)?;
        Ok(())
    })();
    if result.is_err() {
        permit.lock().unwrap_or_else(|e| e.into_inner()).stop();
    }
    result
}
impl Local {
    /// Does not establish issuer trust, download source, or create another JS engine; verify signatures before allocating children.
    pub fn mount(
        input: Input<'_>,
        lease: Arc<Lease>,
        online: bool,
        cancelled: &impl Fn() -> bool,
    ) -> Result<(Self, Frame), Error> {
        Self::mount_inner(input, lease, online, cancelled, None, None)
    }
    /// Downloaded results retain the initial-admission clock; revalidate input bytes, since Permit cannot replace source integrity.
    pub(crate) fn mount_prepared(
        input: Input<'_>,
        lease: Arc<Lease>,
        online: bool,
        cancelled: &impl Fn() -> bool,
        permit: Arc<Mutex<Permit>>,
        dispatcher: Option<Box<dyn super::services::Dispatcher>>,
    ) -> Result<(Self, Frame), Error> {
        Self::mount_inner(
            input,
            lease,
            online,
            cancelled,
            Some((permit, false)),
            dispatcher,
        )
    }
    pub(crate) fn mount_resumed(
        input: Input<'_>,
        lease: Arc<Lease>,
        cancelled: &impl Fn() -> bool,
        permit: Arc<Mutex<Permit>>,
        dispatcher: Option<Box<dyn super::services::Dispatcher>>,
    ) -> Result<(Self, Frame), Error> {
        if !lease.is_local() {
            return Err(Error::Lease);
        }
        Self::mount_inner(
            input,
            lease,
            false,
            cancelled,
            Some((permit, true)),
            dispatcher,
        )
    }
    fn mount_inner(
        input: Input<'_>,
        lease: Arc<Lease>,
        online: bool,
        cancelled: &impl Fn() -> bool,
        prior: Option<(Arc<Mutex<Permit>>, bool)>,
        dispatcher: Option<Box<dyn super::services::Dispatcher>>,
    ) -> Result<(Self, Frame), Error> {
        let current = input.expected.binding;
        if input.view.origin() != current.origin
            || input.view.device() != current.device_id
            || input.view.instance() != current.instance_id
            || input.view.generation() != current.generation
            || input.view.lifecycle() != crate::surface_live_binding::Lifecycle::Active
        {
            return Err(Error::Binding);
        }
        let resumed = prior.as_ref().is_some_and(|(_, resumed)| *resumed);
        let accept = if resumed {
            Permit::accept_resumed
        } else {
            Permit::accept
        };
        let accepted = accept(
            input.envelope,
            input.issuer,
            input.expected,
            input.source,
            now()?,
        )
        .map_err(Error::Grant)?;
        let permit = prior
            .map(|(permit, _)| permit)
            .unwrap_or_else(|| Arc::new(Mutex::new(accepted)));
        // Only downloaded owners may inject the original permit dispatcher; old emits remain neither services nor committed results.
        let scope = permit
            .lock()
            .map_err(|_| Error::Stopped)?
            .authorize(current, now()?, online, false)
            .map_err(Error::Grant)?
            .permissions
            .clone();
        if !scope.contains(&wire::Permission::LocalState)
            || !scope.contains(&wire::Permission::Preview)
            || (scope.contains(&wire::Permission::LoomService)
                && (dispatcher.is_none() || (!online && !resumed)))
        {
            return Err(Error::Unsupported);
        }
        check(&permit, &lease, current, online, cancelled)?;
        let failure = RefCell::new(None);
        let stopped = || match check(&permit, &lease, current, online, cancelled) {
            Ok(()) => false,
            Err(error) => {
                *failure.borrow_mut() = Some(error);
                true
            }
        };
        let source = std::str::from_utf8(input.source)
            .map_err(|_| Error::Binding)?
            .to_owned();
        let result = Session::mount_serviced(
            super::services::Mount {
                binding: input.view,
                source,
                snapshot: input.snapshot,
                resources: input.resources,
                images: input.images,
                viewport: input.viewport,
                dispatcher,
                allow_results: scope.contains(&wire::Permission::WorkspaceResult),
            },
            &stopped,
        );
        let (session, frame) =
            result.map_err(|e| failure.into_inner().unwrap_or(Error::Execution(e.kind())))?;
        let mut local = Self {
            permit,
            lease,
            session: Some(session),
        };
        local.refresh(current, online, cancelled)?;
        local.frame(frame).map(|frame| (local, frame))
    }
    /// Idle owners must call on authorization-deadline/revocation wakeups; this seam starts no additional timer thread.
    pub fn refresh(
        &mut self,
        current: &wire::Binding,
        online: bool,
        cancelled: &impl Fn() -> bool,
    ) -> Result<(), Error> {
        if self.session.is_none() {
            return Err(Error::Stopped);
        }
        if let Err(error) = check(&self.permit, &self.lease, current, online, cancelled) {
            self.stop();
            return Err(error);
        }
        Ok(())
    }
    fn frame(&mut self, frame: Frame) -> Result<Frame, Error> {
        if !frame.events.is_empty() {
            self.stop();
            return Err(Error::Unsupported);
        }
        Ok(frame)
    }
    fn operate(
        &mut self,
        current: &wire::Binding,
        online: bool,
        cancelled: &impl Fn() -> bool,
        execute: impl FnOnce(&mut Session, &dyn Fn() -> bool) -> io::Result<Frame>,
    ) -> Result<Frame, Error> {
        self.refresh(current, online, cancelled)?;
        let failure = RefCell::new(None);
        let result = execute(
            self.session.as_mut().ok_or(Error::Stopped)?,
            &|| match check(&self.permit, &self.lease, current, online, cancelled) {
                Ok(()) => false,
                Err(error) => {
                    *failure.borrow_mut() = Some(error);
                    true
                }
            },
        );
        let frame = match result {
            Ok(frame) => frame,
            Err(error) => {
                self.stop();
                return Err(failure
                    .into_inner()
                    .unwrap_or(Error::Execution(error.kind())));
            }
        };
        self.refresh(current, online, cancelled)?;
        self.frame(frame)
    }
    pub fn click(
        &mut self,
        current: &wire::Binding,
        online: bool,
        target: &str,
        cancelled: &impl Fn() -> bool,
    ) -> Result<Frame, Error> {
        self.operate(current, online, cancelled, |session, stop| {
            session.click(target, &stop)
        })
    }
    pub fn suspend(
        &mut self,
        current: &wire::Binding,
        online: bool,
        cancelled: &impl Fn() -> bool,
    ) -> Result<Frame, Error> {
        self.operate(current, online, cancelled, |session, stop| {
            session.suspend(&stop)
        })
    }
    pub fn resume(
        &mut self,
        current: &wire::Binding,
        online: bool,
        cancelled: &impl Fn() -> bool,
    ) -> Result<Frame, Error> {
        self.operate(current, online, cancelled, |session, stop| {
            session.resume(&stop)
        })
    }
    pub(crate) fn scheduled(
        &mut self,
        current: &wire::Binding,
        online: bool,
        viewport: crate::surface_viewport::Viewport,
        animation: Option<bool>,
        cancelled: &impl Fn() -> bool,
    ) -> Result<Frame, Error> {
        self.operate(current, online, cancelled, |session, stop| {
            session.set_viewport(viewport)?;
            match animation {
                Some(true) => session.animation_frame(&stop),
                Some(false) => session.timer_tick(&stop),
                None => session.resume(&stop),
            }
        })
    }
    pub(crate) fn input(
        &mut self,
        current: &wire::Binding,
        online: bool,
        input: Interaction,
        viewport: crate::surface_viewport::Viewport,
        cancelled: &impl Fn() -> bool,
    ) -> Result<Frame, Error> {
        self.operate(current, online, cancelled, |session, stop| {
            session.set_viewport(viewport)?;
            match input {
                Interaction::Input(input) => session.local_input(input, &stop),
                Interaction::Click(target) => session.click(&target, &stop),
            }
        })
    }
    /// Automatic callbacks execute only through the original serial child; every blocking seam checks both this claim and signed authorization.
    pub(crate) fn live_trigger(
        &mut self,
        current: &wire::Binding,
        online: bool,
        ticket: crate::art_live_trigger::ticket::Ticket,
        viewport: crate::surface_viewport::Viewport,
    ) -> Result<Frame, Error> {
        self.operate(
            current,
            online,
            &|| !ticket.current(current),
            |session, stop| {
                session.set_viewport(viewport)?;
                session.live_trigger(ticket.event().clone(), &stop)
            },
        )
    }
    /// Never execute untrusted dispose callbacks; reap only owned children without revoking other owners' device leases.
    pub fn stop(&mut self) {
        self.permit.lock().unwrap_or_else(|e| e.into_inner()).stop();
        self.session.take();
    }
}
#[cfg(test)]
#[path = "surface_javascript_signed_tests.rs"]
mod tests;
