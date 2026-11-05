//! 首次挂载集合：取消仅禁止交付场景，不丢失已确认绑定或未知创建的恢复上下文。
use super::*;
use crate::surface_attach_runtime::{Error, Job, Outcome, Request};

impl Sessions {
    pub fn attaching(&self) -> bool {
        !self.attach_jobs.is_empty()
    }
    /// 交付方容量不足时原样放回，不能通过 pop 丢失未知创建或确认绑定的证据。
    pub fn deliver_attach(
        &mut self,
        deliver: impl FnOnce(Outcome) -> Result<(), Box<Outcome>>,
    ) -> bool {
        let Some(outcome) = self.attach_outcomes.pop() else {
            return false;
        };
        match deliver(outcome) {
            Ok(()) => true,
            Err(outcome) => {
                self.attach_outcomes.push(*outcome);
                false
            }
        }
    }
    pub fn start_attach(
        &mut self,
        config: Config,
        lease: Lease,
        request: Arc<Request>,
    ) -> io::Result<()> {
        if self.closed {
            return Err(io::ErrorKind::BrokenPipe.into());
        }
        if self.occupied() >= MAX_SESSIONS {
            return Err(io::ErrorKind::WouldBlock.into());
        }
        self.attach_jobs.try_reserve(1).map_err(io::Error::other)?;
        // 回收前预留结果容量；未取走结果同样占总配额，不允许无界积累。
        self.attach_outcomes
            .try_reserve(self.attach_jobs.len() + 1)
            .map_err(io::Error::other)?;
        self.attach_jobs.push(Job::start(config, lease, request)?);
        Ok(())
    }
    pub(super) fn cancel_attach_requests(&mut self) {
        for job in &self.attach_jobs {
            job.request_stop();
        }
        for outcome in &mut self.attach_outcomes {
            outcome.result = Err(Error::Stopped);
        }
    }
    pub(super) fn poll_attaches(&mut self) -> Vec<(String, super::surfaces::Error)> {
        let mut errors = Vec::new();
        let mut index = 0;
        while index < self.attach_jobs.len() {
            if let Some(outcome) = self.attach_jobs[index].try_join() {
                self.attach_jobs.swap_remove(index);
                if let Err(error) = &outcome.result {
                    if *error != Error::Stopped {
                        errors.push((
                            outcome.origin.clone(),
                            super::surfaces::Error::Attach(*error),
                        ));
                    }
                }
                self.attach_outcomes.push(outcome);
            } else {
                index += 1;
            }
        }
        errors
    }
    /// 只移交恢复证据；调用方须重新验证授权，不能自动重放未知创建。
    pub fn take_attach_outcome(&mut self) -> Option<Outcome> {
        self.attach_outcomes.pop()
    }
}
