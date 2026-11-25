//! Image GET shares capacity/waits with existing application remote tasks; it is not an attachment transaction and creates no recovery evidence.
use super::*;
use crate::surface_resource_runtime::request::{inline::Inline, Request};
use crate::{
    live_relay_http::ResourceLease,
    surface_live_binding::Binding,
    surface_resource_runtime::{Error as ResourceError, Job as ResourceJob, Ready},
};

impl Sessions {
    /// Release Ready proactively at expiry; cancelled but unreclaimed threads no longer contribute past deadlines that would cause busy loops.
    pub fn expire_surface_resources(&mut self, now: u64) {
        for job in &self.resource_jobs {
            if job.deadline_ms().is_some_and(|expiry| expiry <= now) {
                job.request_stop();
            }
        }
        self.resource_ready.retain(|ready| {
            if ready.expires_at_ms() <= now {
                ready.request_stop();
                false
            } else {
                true
            }
        });
    }
    pub fn resource_deadline(&self, now: u64) -> Option<std::time::Instant> {
        self.resource_jobs
            .iter()
            .filter_map(ResourceJob::deadline_ms)
            .chain(self.resource_ready.iter().map(Ready::expires_at_ms))
            .min()
            .and_then(|expiry| {
                std::time::Instant::now()
                    .checked_add(std::time::Duration::from_millis(expiry.saturating_sub(now)))
            })
    }
    pub fn start_surface_resource(
        &mut self,
        config: Config,
        lease: Lease,
        request: ResourceLease,
    ) -> io::Result<()> {
        self.start_resource_with(Request::Resource(request), |request| {
            let Request::Resource(request) = request else {
                unreachable!()
            };
            ResourceJob::start(config, lease, request)
        })
    }
    #[cfg(test)]
    pub(crate) fn start_resource_prepared(
        &mut self,
        client: crate::device_session_client::Client,
        lease: Lease,
        request: ResourceLease,
    ) -> io::Result<()> {
        self.start_resource_with(Request::Resource(request), |request| {
            let Request::Resource(request) = request else {
                unreachable!()
            };
            ResourceJob::start_prepared(client, lease, request)
        })
    }
    pub fn start_inline_image(&mut self, lease: Lease, request: Inline) -> io::Result<()> {
        self.start_resource_with(Request::Inline(request), |request| {
            let Request::Inline(request) = request else {
                unreachable!()
            };
            ResourceJob::start_inline(lease, request)
        })
    }
    fn start_resource_with(
        &mut self,
        request: Request,
        spawn: impl FnOnce(Request) -> io::Result<ResourceJob>,
    ) -> io::Result<()> {
        if self.closed {
            return Err(io::ErrorKind::BrokenPipe.into());
        }
        if self.occupied() >= MAX_SESSIONS {
            return Err(io::ErrorKind::WouldBlock.into());
        }
        // Do not download the same binding/scene/resource concurrently; distinct resources still share four-slot and session capacity limits.
        let duplicate = self.resource_jobs.iter().any(|job| {
            let target = job.target();
            &target.binding == request.binding()
                && target.revision == request.revision()
                && target.key == request.key()
        }) || self.resource_ready.iter().any(|ready| {
            ready.binding() == request.binding()
                && ready.revision() == request.revision()
                && ready.key() == request.key()
        });
        if duplicate {
            return Err(io::ErrorKind::AlreadyExists.into());
        }
        self.resource_jobs
            .try_reserve(1)
            .map_err(io::Error::other)?;
        self.resource_ready
            .try_reserve(self.resource_jobs.len() + 1)
            .map_err(io::Error::other)?;
        self.resource_jobs.push(spawn(request)?);
        Ok(())
    }
    /// Cancel by binding/scene revision on scene replacement/window closure; retain threads in the collection for reclamation until exit.
    pub fn cancel_surface_resources(&mut self, mut matches: impl FnMut(&Binding, u64) -> bool) {
        for job in &self.resource_jobs {
            if matches(&job.target().binding, job.target().revision) {
                job.request_stop();
            }
        }
        self.resource_ready.retain(|ready| {
            if matches(ready.binding(), ready.revision()) {
                ready.request_stop();
                false
            } else {
                true
            }
        });
    }
    pub(super) fn poll_surface_resources(&mut self) -> Vec<(String, super::surfaces::Error)> {
        let mut errors = Vec::new();
        let mut index = 0;
        while index < self.resource_jobs.len() {
            if let Some(result) = self.resource_jobs[index].try_join() {
                let job = self.resource_jobs.swap_remove(index);
                match result {
                    Ok(ready) if !self.closed => self.resource_ready.push(ready),
                    Ok(ready) => ready.request_stop(),
                    Err(ResourceError::Stopped) => (),
                    Err(error) => errors.push((
                        job.target().binding.origin().to_owned(),
                        super::surfaces::Error::Resource(error),
                    )),
                }
            } else {
                index += 1;
            }
        }
        errors
    }
    /// Receivers must revalidate Ready::image against the current mount. Return the complete owner when temporarily unable to admit.
    pub fn deliver_surface_resource(
        &mut self,
        mut deliver: impl FnMut(Ready) -> Result<(), Box<Ready>>,
    ) -> bool {
        for index in (0..self.resource_ready.len()).rev() {
            let ready = self.resource_ready.remove(index);
            match deliver(ready) {
                Ok(()) => return true,
                Err(ready) => self.resource_ready.insert(index, *ready),
            }
        }
        false
    }
}
