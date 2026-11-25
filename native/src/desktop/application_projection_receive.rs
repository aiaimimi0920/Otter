//! Sole receiver consumer: original request fence -> background candidate -> hidden append -> exact checkpoint -> visible presentation/receipt.
use super::*;
use crate::{
    device_authorization_lease::Lease,
    projection_delivery_runtime::{Hold, Ready},
    session_source::Source,
};
#[path = "application_projection_receive_scope.rs"]
mod scope;
use scope::stale;
pub(in crate::desktop::application) use scope::Scope;
#[path = "application_projection_receive_display.rs"]
mod display;
#[path = "application_projection_receive_job.rs"]
mod job;
#[path = "application_projection_receive_lifecycle.rs"]
mod lifecycle;
#[path = "application_projection_present.rs"]
pub(in crate::desktop::application) mod present;
#[path = "application_projection_receive_recovery.rs"]
mod recovery;
#[path = "application_projection_sync.rs"]
mod sync;

#[derive(Default)]
pub(in crate::desktop::application) struct Owner(Phase, sync::Schedule);
#[derive(Default)]
enum Phase {
    #[default]
    Idle,
    Waiting(Option<Box<Scope>>),
    Preparing(Box<Pending>),
    Recovering(Box<recovery::Pending>),
    Syncing(Box<sync::Pending>),
    Checkpoint(Box<Saved>),
    Reporting(Box<Reporting>),
}
struct Pending {
    scope: Scope,
    origin: String,
    current: Lease,
    job: job::Job,
}
struct Reporting {
    source: Source,
    current: Lease,
    visual: present::Visual,
    job: crate::projection_delivery_runtime::Job,
    recovery: Option<crate::projection_delivery::binding::Binding>,
}
struct Saved {
    cancel: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    source: Source,
    held: Hold,
    expected: crate::projection_delivery::binding::Binding,
    durable: bool,
    current: Lease,
    visual: present::Visual,
    workspace: crate::workspace::IdentityStamp,
    recovery: bool,
}
impl Owner {
    #[cfg(test)]
    pub(in crate::desktop::application) fn preparation_finished(&self) -> bool {
        matches!(&self.0, Phase::Preparing(pending) if pending.job.finished())
    }
    pub(in crate::desktop::application) fn busy(&self) -> bool {
        !matches!(self.0, Phase::Idle)
    }
    pub(in crate::desktop::application) fn preparing(&self) -> bool {
        // Continuous read does not modify UI and may coexist with user save/restore preparation; commit still checks original source/Document fences.
        matches!(
            self.0,
            Phase::Preparing(_) | Phase::Recovering(_) | Phase::Reporting(_)
        )
    }
    pub(in crate::desktop::application) fn waiting(&mut self, scope: Option<Scope>) {
        self.0 = Phase::Waiting(scope.map(Box::new));
    }
    pub(super) fn finish_metadata(&mut self) {
        if matches!(self.0, Phase::Waiting(_)) {
            self.0 = Phase::Idle;
            self.1.defer_foreground(std::time::Instant::now());
        }
    }
    pub(in crate::desktop::application) fn deadline(&self, now: u64) -> Option<std::time::Instant> {
        let reap = match &self.0 {
            Phase::Preparing(pending) => pending.job.reap_deadline(),
            _ => None,
        };
        self.phase_deadline(now)
            .into_iter()
            .chain(reap)
            .chain(self.1.wake)
            .min()
    }
    fn phase_deadline(&self, now: u64) -> Option<std::time::Instant> {
        let expiry = match &self.0 {
            Phase::Preparing(pending) if !pending.job.stopped() => pending.job.expires_at_ms(),
            Phase::Recovering(pending) => pending.job.expires_at_ms()?,
            Phase::Syncing(pending) => pending.job.expires_at_ms()?,
            Phase::Checkpoint(saved) => saved.held.expires_at_ms(),
            Phase::Reporting(reporting) => {
                let held = reporting.job.held()?;
                if held.lease().is_revoked() {
                    return None; // After cancellation, wait only on the original thread handle to avoid zero-timeout busy loops from expired deadlines.
                }
                held.expires_at_ms()
            }
            _ => return None,
        };
        std::time::Instant::now()
            .checked_add(std::time::Duration::from_millis(expiry.saturating_sub(now)))
    }
    pub(in crate::desktop::application) fn wait_handle(
        &self,
    ) -> Option<std::os::windows::io::BorrowedHandle<'_>> {
        match &self.0 {
            Phase::Reporting(reporting) => reporting.job.wait_handle(),
            Phase::Recovering(pending) => pending.job.wait_handle(),
            Phase::Syncing(pending) => pending.job.wait_handle(),
            _ => None,
        }
    }
}
impl Application {
    fn receive_lease(&mut self, origin: &str) -> Result<Lease> {
        let view = self.authorization.borrow();
        if view.closed || view.origin.as_deref() != Some(origin) || self.exit_requested.get() {
            return Err(stale());
        }
        self.authorization_owner
            .lease(origin, job::now()?)
            .map_err(|_| stale())
    }
    fn receive_hold_valid(&self, held: &Hold, current: &Lease) -> bool {
        let view = self.authorization.borrow();
        // The retained receive lease shares Grant with the original request; cache replacement permanently revokes both synchronously, avoiding repeated hot-path Lease allocation.
        !view.closed
            && view.origin.as_deref() == Some(held.origin())
            && !self.exit_requested.get()
            && self.projection_profile.recorded(held.ticket())
            && job::now().is_ok_and(|now| held.validate(current, now).is_ok())
    }
    pub(in crate::desktop::application) fn receive_projection_ready(&mut self, ready: Ready) {
        let cleanup = ready.cleanup_ticket();
        if !matches!(self.projection_receive.0, Phase::Waiting(_)) {
            drop(ready);
            self.receive_cleanup(cleanup);
            self.receive_status(Err(stale()));
            return; // Do not overwrite or synchronously join another in-flight owner.
        }
        let phase = std::mem::take(&mut self.projection_receive.0);
        let result = (|| {
            let Phase::Waiting(Some(scope)) = phase else {
                return Err(stale());
            };
            if !unsafe { scope.matches(self) } {
                return Err(stale());
            }
            let origin = ready.origin().to_owned();
            let lease = self.receive_lease(&origin)?;
            // The same Store/Record from the original profile is required; UUIDs or equal disk content are not substitutes.
            if !self.projection_profile.recorded(ready.ticket()) {
                return Err(stale());
            }
            let job = job::Job::start(
                ready,
                &lease,
                &scope,
                self.workspace.units().to_vec(),
                self.execution_wake.callback(),
            )?;
            self.projection_receive.0 = Phase::Preparing(Box::new(Pending {
                scope: *scope,
                origin,
                current: lease,
                job,
            }));
            Ok("正在准备接收投送…")
        })();
        if result.is_err() {
            self.receive_cleanup(cleanup);
        }
        self.receive_status(result);
    }
    pub(in crate::desktop::application) fn poll_projection_receive(
        &mut self,
        config: &mut impl FnMut(
            &str,
        ) -> std::result::Result<
            crate::device_authorization_config::Config,
            crate::device_authorization_config::Error,
        >,
    ) {
        // Recheck after taking state so this task's preparing flag cannot block its own Guard/append/checkpoint.
        let phase = std::mem::take(&mut self.projection_receive.0);
        let result = match phase {
            Phase::Preparing(mut pending) => {
                if !pending.job.valid(&pending.origin) || !unsafe { pending.scope.matches(self) } {
                    pending.job.stop();
                }
                let Some(result) = pending.job.poll() else {
                    self.projection_receive.0 = Phase::Preparing(pending);
                    return;
                };
                // Snapshot only after reaping; successful poll already moved Hold, so the ticket cannot be taken from Job alone.
                let cleanup = match &result {
                    Ok(prepared) => prepared.held.cleanup_ticket(),
                    Err(_) => pending.job.cleanup_ticket(),
                };
                let result = match result {
                    Ok(prepared) if !pending.job.stopped() => unsafe {
                        self.publish_projection_receive(pending.scope, prepared, pending.current)
                    },
                    _ => Err(stale()),
                };
                drop(pending.job); // Even when stopped discards a successful result, release the original Hold before transferring.
                if result.is_err() {
                    self.receive_cleanup(cleanup);
                }
                result
            }
            Phase::Checkpoint(mut saved) => unsafe {
                if self.projection_checkpoint_inclusion(saved.held.ticket())
                    == Some(&saved.expected)
                {
                    saved.stabilize();
                }
                let cleanup = saved.held.cleanup_ticket();
                let result = self.display_projection_saved(saved, config);
                if result.is_err() {
                    self.receive_cleanup(cleanup);
                }
                result
            },
            Phase::Reporting(reporting) => unsafe { self.poll_projection_report(reporting) },
            Phase::Recovering(pending) => unsafe { self.poll_projection_recovery(pending, config) },
            other => {
                self.projection_receive.0 = other;
                return;
            }
        };
        self.receive_status(result);
    }
    unsafe fn publish_projection_receive(
        &mut self,
        scope: Scope,
        prepared: job::Prepared,
        current: Lease,
    ) -> Result<&'static str> {
        use crate::desktop::session_binding::Bound;
        use std::sync::atomic::AtomicBool;
        if !scope.matches(self) || !self.receive_hold_valid(&prepared.held, &current) {
            return Err(stale());
        }
        let cancel = AtomicBool::new(false);
        let catalog = self.catalog.borrow().loaded.clone();
        let states = Bound::from_prepared(prepared.restore, catalog.clone())
            .into_parts_for(catalog.as_ref(), &cancel)
            .map_err(|_| stale())?
            .prepare_states(&cancel)?;
        let windows = self.prepare_restored_batch(states, &cancel, |placement| {
            scope.space.physical(placement.rect).map_err(|_| stale())
        })?;
        if !scope.matches(self) || !self.receive_hold_valid(&prepared.held, &current) {
            return Err(stale());
        }
        let append = self.prepare_append(windows, scope.source, prepared.retained, scope.space)?;
        self.publish_append(append, false)?;
        // Shared publication still exclusively owns Windows; after takeover, obtain presentation credentials from Application's original Document owner.
        let document = self
            .documents
            .iter()
            .find(|doc| doc.state.unit_id.as_str() == prepared.held.ticket().unit())
            .ok_or_else(stale)?;
        let visual = present::Visual::capture(document)?;
        self.projection_receive.0 = Phase::Checkpoint(Box::new(Saved {
            cancel: scope.cancel,
            source: self.restored_session.as_ref().unwrap().original.clone(),
            held: prepared.held,
            expected: prepared.expected,
            durable: false,
            current,
            visual,
            workspace: self.workspace.identity_stamp(),
            recovery: false,
        }));
        Ok("正在保存接收投送…")
    }
    fn receive_status(&mut self, result: Result<&str>) {
        #[cfg(test)]
        if let Err(error) = &result {
            eprintln!("projection-receive-failure: {error}");
        }
        self.projection_receive
            .1
            .defer_foreground(std::time::Instant::now());
        let mut view = self.authorization.borrow_mut();
        // Syncing occupies only the serial slot, not foreground UI busy state; rejecting explicit Display must not invalidate it.
        view.projection.busy = self.remote_sessions.projection_pending()
            || (self.projection_receive.busy()
                && !matches!(self.projection_receive.0, Phase::Syncing(_)));
        let status = result.unwrap_or("接收未完成；本地内容和未结意图保留");
        if view.projection.status != status {
            view.projection.status.clear();
            view.projection.status.push_str(status);
        }
    }
}
