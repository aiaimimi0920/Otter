//! 同一任务槽内区分远端租约与本地 Shader；共享退休/回收调度，不伪造远端身份或清理确认。
use super::{loom_execution_job as remote, shader_execution_job as shader};

pub enum Completion {
    Remote(Result<remote::Completion, remote::Error>),
    Shader(Result<shader::Completion, shader::Error>),
}
impl Completion {
    pub(super) fn cleanup_pending(&self) -> bool {
        matches!(self, Self::Remote(Ok(completion)) if completion.cleanup_pending())
    }
    pub(super) fn cleanup_unconfirmed(&self) -> bool {
        self.cleanup_pending()
            || matches!(
                self,
                Self::Remote(Err(_)) | Self::Shader(Err(shader::Error::WorkerFailed))
            )
    }
    #[cfg(test)]
    pub(crate) fn into_remote(self) -> Option<Result<remote::Completion, remote::Error>> {
        match self {
            Self::Remote(result) => Some(result),
            Self::Shader(_) => None,
        }
    }
}
pub(super) enum Work {
    Remote(remote::Job),
    Shader(shader::Job),
}
impl Work {
    pub(super) fn cancel(&self) {
        match self {
            Self::Remote(job) => job.cancel(),
            Self::Shader(job) => job.cancel(),
        }
    }
    pub(super) fn poll(&mut self) -> Option<Completion> {
        match self {
            Self::Remote(job) => job.poll().map(Completion::Remote),
            Self::Shader(job) => job.poll().map(Completion::Shader),
        }
    }
    pub(super) fn shutdown(&mut self) -> Option<Completion> {
        match self {
            Self::Remote(job) => job.shutdown().map(Completion::Remote),
            Self::Shader(job) => job.shutdown().map(Completion::Shader),
        }
    }
    pub(super) fn take_updates(
        &self,
    ) -> (Option<f64>, Option<super::loom_execution_session::Update>) {
        match self {
            Self::Remote(job) => job.take_updates(),
            Self::Shader(_) => (None, None),
        }
    }
}
