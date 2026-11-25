//! Explicit sharing/acceptance fixes the current workspace and original menu source; background work receives only immutable values, never HWNDs.
use super::*;
use crate::desktop::device_authorization_ui::account::Request;
impl Application {
    pub(super) unsafe fn start_account_request(&mut self, request: Request) -> Result<()> {
        if !self.account_idle()
            || self.account_projection.pending.is_some()
            || self.account_projection.saved.is_some()
        {
            return Err(stale());
        }
        if let Request::Stop(binding) = request {
            return self.start_account_sync(binding, true);
        }
        let participation = self
            .authorization
            .borrow()
            .consent
            .participation()
            .ok_or_else(stale)?;
        let (request, recovery) = match request {
            Request::Recover(intent) => (
                Request::Import {
                    envelope: intent.envelope.clone(),
                    cancel: Arc::new(AtomicBool::new(false)),
                },
                Some(intent),
            ),
            request => (request, None),
        };
        let origin = self
            .authorization
            .borrow()
            .origin
            .clone()
            .ok_or_else(stale)?;
        let root = self
            .workspace_checkpoint
            .as_ref()
            .ok_or_else(stale)?
            .profile_root()
            .to_owned();
        let lease = self
            .authorization_owner
            .lease(&origin, crate::desktop::surface_windows::now_ms()?)
            .map_err(|_| stale())?;
        let discovery = crate::loom_discovery::Config::from_env();
        let notify = self.execution_wake.callback();
        let (guard, source, cancel, job) = match request {
            Request::Create(selection) => {
                if selection.origin != origin
                    || selection.cancel.load(Ordering::Acquire)
                    || !self.documents.iter().any(|d| {
                        d.state.unit_id == selection.unit
                            && d.is_alive()
                            && selection.target.matches(&d.state.canvas.borrow())
                    })
                {
                    return Err(stale());
                }
                let (mut scope, snapshot) =
                    projection_sender::scope::Scope::capture(self, &selection.unit)?;
                scope.cancel = Some(selection.cancel.clone());
                let source = scope.source.clone();
                let input = work::SourceInput {
                    source: scope.binding_source.clone(),
                    unit: scope.unit.clone(),
                    scope: scope.scope,
                    snapshot,
                };
                let target = origin.clone();
                let job = job::Job::start(lease, notify, move |lease| {
                    work::create(lease, &discovery, &target, input)
                })?;
                (
                    Guard::Source(Box::new(scope)),
                    source,
                    Some(selection.cancel),
                    job,
                )
            }
            Request::Import { envelope, cancel } => {
                if cancel.load(Ordering::Acquire)
                    || (recovery.is_none() && self.authorization.borrow().consent.stamp().is_none())
                {
                    return Err(stale());
                }
                let destination = if recovery.is_some() {
                    account_projection::Destination::capture_existing(self)?
                } else {
                    account_projection::Destination::capture(self)?
                };
                let source = destination.source().clone();
                let input = work::ReceiveInput {
                    source: source.clone(),
                    space: destination.space,
                    scope: destination.scope,
                    units: self.workspace.units().to_vec(),
                    root,
                    envelope,
                    recovery,
                };
                let target = origin.clone();
                let job = job::Job::start(lease, notify, move |lease| {
                    work::receive(lease, &discovery, &target, input)
                })?;
                (
                    Guard::Receive(Box::new(destination)),
                    source,
                    Some(cancel),
                    job,
                )
            }
            Request::Stop(_) | Request::Recover(_) => unreachable!(),
        };
        self.account_projection.pending = Some(Pending {
            job,
            origin,
            source,
            workspace: self.workspace.identity_stamp(),
            guard,
            cancel,
            participation: Some(participation),
        });
        self.authorization.borrow_mut().account.received = None;
        self.account_status(crate::ui_text::text("app-account-processing"));
        Ok(())
    }
}
