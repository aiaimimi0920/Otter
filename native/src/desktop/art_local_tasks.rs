//! Bounded local Art download tasks: the original registry owns generations/leases; threads being cleaned up continue occupying slots.
//! The UI transfers only configuration snapshots; DNS/TLS/source-signature verification runs in workers, without automatic trust establishment or window creation.
use super::loom_execution_signal::{reap_deadline, CompletionSignal, Notify};
use crate::{
    art_execution_grant::wire::Binding,
    art_issuer_trust::Authority,
    art_local_requests::{Completion, Error, Pending, Registry},
    device_authorization_config::Config,
    device_authorization_lease::Lease,
    graph_requests::Request as Token,
    live_relay_http::art_prepare::{Prepared, Request},
    workspace_id::UnitId,
};
use std::{
    sync::{atomic::AtomicBool, Arc},
    thread::JoinHandle,
};

const MAX_JOBS: usize = 2;
struct Job {
    unit: UnitId,
    token: Token,
    lease: Arc<Lease>,
    ready: Arc<AtomicBool>,
    worker: JoinHandle<Completion>,
}
pub struct Report {
    pub unit: UnitId,
    pub result: Result<Prepared, Error>,
}
pub struct Tasks {
    registry: Registry,
    jobs: Vec<Job>,
    notify: Notify,
}
impl Tasks {
    /// notify must be brief and nonblocking, never synchronously waiting for the UI; production callers use Application Wake's posting callback.
    pub fn new(workspace: String, notify: Arc<dyn Fn() + Send + Sync>) -> Self {
        Self {
            registry: Registry::new(workspace),
            jobs: Vec::new(),
            notify,
        }
    }
    /// Finished but uncollected threads still occupy slots; retain at most two bounded source results.
    pub fn busy(&self) -> bool {
        !self.jobs.is_empty()
    }
    pub fn has_capacity(&self) -> bool {
        self.jobs.len() < MAX_JOBS
    }
    pub(super) fn deadline(&self) -> Option<std::time::Instant> {
        self.jobs
            .iter()
            .filter_map(|job| reap_deadline(&job.ready))
            .min()
    }
    /// Device description through source-signature verification share one worker and the original Lease; the UI does not fetch the description synchronously first.
    pub fn start_description(
        &mut self,
        unit: UnitId,
        request: crate::art_local_requests::description::Request,
        lease: Arc<Lease>,
        authority: &mut Authority,
        config: Config,
    ) -> Result<Token, Error> {
        if config.origin() != request.origin {
            return Err(Error::Configuration(
                crate::device_authorization_config::Error::Origin,
            ));
        }
        let worker_lease = lease.clone();
        self.start_description_with(unit, request, lease, authority, move |pending| {
            let cache = crate::art_source_cache::Store::new(config.identity_root());
            if worker_lease.is_local() {
                return Ok(pending.fetch_offline(config.identity_root()));
            }
            let client = worker_lease
                .prepare_client(&config)
                .map_err(Error::Configuration)?;
            Ok(pending.fetch_cached(&client, &cache))
        })
    }
    pub(crate) fn start_description_with(
        &mut self,
        unit: UnitId,
        request: crate::art_local_requests::description::Request,
        lease: Arc<Lease>,
        authority: &mut Authority,
        run: impl FnOnce(
                crate::art_local_requests::description::PendingDescription,
            ) -> Result<Completion, Error>
            + Send
            + 'static,
    ) -> Result<Token, Error> {
        self.reserve_job()?;
        let pending = self
            .registry
            .begin_description(unit, request, lease.clone(), authority)?;
        let (unit, token) = pending.identity();
        self.spawn(unit, token.clone(), lease, move || run(pending))?;
        Ok(token)
    }
    pub fn start(
        &mut self,
        unit: UnitId,
        request: Request,
        lease: Arc<Lease>,
        authority: &mut Authority,
        config: Config,
    ) -> Result<Binding, Error> {
        if config.origin() != request.binding.origin {
            return Err(Error::Configuration(
                crate::device_authorization_config::Error::Origin,
            ));
        }
        let worker_lease = lease.clone();
        self.start_with(unit, request, lease, authority, move |pending| {
            let client = worker_lease
                .prepare_client(&config)
                .map_err(Error::Configuration)?;
            Ok(pending.fetch(&client))
        })
    }
    #[cfg(test)]
    pub(crate) fn start_client(
        &mut self,
        unit: UnitId,
        request: Request,
        lease: Arc<Lease>,
        authority: &mut Authority,
        client: crate::device_session_client::Client,
    ) -> Result<Binding, Error> {
        self.start_with(unit, request, lease, authority, move |pending| {
            Ok(pending.fetch(&client))
        })
    }
    fn start_with(
        &mut self,
        unit: UnitId,
        request: Request,
        lease: Arc<Lease>,
        authority: &mut Authority,
        run: impl FnOnce(Pending) -> Result<Completion, Error> + Send + 'static,
    ) -> Result<Binding, Error> {
        self.reserve_job()?;
        let pending = self
            .registry
            .begin(unit, request, lease.clone(), authority)?;
        let binding = pending.binding().clone();
        let (unit, token) = pending.identity();
        self.spawn(unit, token, lease, move || run(pending))?;
        Ok(binding)
    }
    fn reserve_job(&mut self) -> Result<(), Error> {
        // Reject capacity before changing the Registry generation, without disturbing existing instances.
        if !self.has_capacity() {
            return Err(Error::Capacity);
        }
        self.jobs.try_reserve(1).map_err(|_| Error::Capacity)
    }
    fn spawn(
        &mut self,
        unit: UnitId,
        token: Token,
        lease: Arc<Lease>,
        run: impl FnOnce() -> Result<Completion, Error> + Send + 'static,
    ) -> Result<(), Error> {
        let failure_unit = unit.clone();
        let failure_token = token.clone();
        let ready = Arc::new(AtomicBool::new(false));
        let completed = ready.clone();
        let notify = self.notify.clone();
        let worker = std::thread::Builder::new()
            .name("otter-art-prepare".into())
            .spawn(move || {
                // Declare signal first so run and its captured resources drop before notification; panics also publish completion.
                let _signal = CompletionSignal::new(completed, notify);
                let run = run;
                match run() {
                    Ok(completion) => completion,
                    Err(error) => Completion::failed(failure_unit, failure_token, error),
                }
            });
        match worker {
            Ok(worker) => self.jobs.push(Job {
                unit,
                token,
                lease,
                ready,
                worker,
            }),
            Err(_) => {
                self.registry.cancel(&unit);
                return Err(Error::Worker);
            }
        }
        Ok(())
    }
    /// Join only exited workers; if notification precedes exit, the deadline continues reclamation.
    /// Discard late successes and failures alike; neither may change the current node's error state or revoke a new instance.
    pub fn take_settled(&mut self) -> Option<Report> {
        while let Some(index) = self.jobs.iter().position(|job| job.worker.is_finished()) {
            let job = self.jobs.remove(index);
            let completion = job.worker.join().unwrap_or_else(|_| {
                Completion::failed(job.unit.clone(), job.token.clone(), Error::Worker)
            });
            let result = self.registry.accept(completion);
            if matches!(result, Err(Error::Stale)) {
                continue;
            }
            return Some(Report {
                unit: job.unit,
                result,
            });
        }
        None
    }
    pub fn cancel(&mut self, unit: &UnitId) {
        self.registry.cancel(unit);
        // Explicitly revoke replaced workers' leases too, not just the latest registry entry.
        for job in self.jobs.iter().filter(|job| &job.unit == unit) {
            job.lease.revoke();
        }
    }
    pub fn mark_running(&mut self, unit: &UnitId, token: &Token) {
        self.registry.mark_running(unit, token);
    }
    pub fn retain_units(&mut self, keep: impl Fn(&UnitId) -> bool) {
        self.registry.retain_units(&keep);
        for job in &self.jobs {
            if !keep(&job.unit) {
                job.lease.revoke();
            }
        }
    }
    /// Workspace swaps synchronously revoke old chains but never join live threads on the UI; cleanup tasks retain their original capacity.
    pub fn reset_workspace(&mut self, workspace: String) {
        self.registry = Registry::new(workspace);
        for job in &self.jobs {
            job.lease.revoke();
        }
    }
    /// On exit, revoke all chains before joining; never detach, release occupied slots early, or collect old-workspace results.
    pub fn shutdown(&mut self) {
        self.registry.clear();
        for job in &self.jobs {
            job.lease.revoke();
        }
        for job in self.jobs.drain(..) {
            let _ = job.worker.join();
        }
    }
}
impl Drop for Tasks {
    fn drop(&mut self) {
        self.shutdown();
    }
}
#[cfg(test)]
#[path = "art_local_description_tasks_tests.rs"]
mod description_tests;
#[cfg(test)]
#[path = "art_local_reap_tests.rs"]
mod reap_tests;
#[cfg(test)]
#[path = "art_local_tasks_tests.rs"]
mod tests;
