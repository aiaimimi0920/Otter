//! 图片 GET 与现有应用远程任务共用容量和等待；不是附件事务，不创建恢复证据。
use super::*;
use crate::surface_resource_runtime::request::{inline::Inline, Request};
use crate::{
    live_relay_http::ResourceLease,
    surface_live_binding::Binding,
    surface_resource_runtime::{Error as ResourceError, Job as ResourceJob, Ready},
};

impl Sessions {
    /// 截止到期主动释放 Ready；已取消但未回收的线程不再贡献过去的 deadline，避免忙循环。
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
        // 同一绑定/场景/资源不并发下载；不同资源也受共享四槽和会话容量限制。
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
    /// 场景替换/窗口关闭用绑定和场景版本取消；线程结束前仍留在集合等待回收。
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
    /// 接纳方必须用 Ready::image 与当前 mount 复核。暂时无法接纳时返回完整 owner。
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
