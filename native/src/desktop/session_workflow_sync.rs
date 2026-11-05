//! 同步 worker 与 UI 的单槽逐条准入握手；后台不读取任何窗口或 Rc 状态。
use super::Ready;
use crate::{
    byte_budget::Budget, desktop::loom_execution_signal::CompletionSignal,
    loom_bridge_identity::Identity, session_document::workflow_snapshot::batch::dispatch,
};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, SyncSender},
        Arc,
    },
    thread::JoinHandle,
    time::{Duration, Instant},
};

pub struct Admission {
    request_id: String,
    workflow_id: String,
    reply: SyncSender<bool>,
}
impl Admission {
    pub fn request_id(&self) -> &str {
        &self.request_id
    }
    pub fn workflow_id(&self) -> &str {
        &self.workflow_id
    }
    /// 消费一次性应答；过期/丢弃的请求不能给下一条消息授权。
    pub fn resolve(self, allowed: bool) {
        let _ = self.reply.try_send(allowed);
    }
}
pub struct Delivery {
    pub ready: Ready,
    pub report: dispatch::Report,
}
pub struct Job {
    cancel: Arc<AtomicBool>,
    completed: Arc<AtomicBool>,
    admissions: Receiver<Admission>,
    worker: Option<JoinHandle<Delivery>>,
}
impl Job {
    pub fn start(
        ready: Ready,
        identity: Identity,
        budget: Arc<Budget>,
        limit: usize,
        notify: Arc<dyn Fn() + Send + Sync>,
    ) -> std::io::Result<Self> {
        let cancel = Arc::new(AtomicBool::new(false));
        let completed = Arc::new(AtomicBool::new(false));
        let (tx, admissions) = mpsc::sync_channel(1);
        let signal = cancel.clone();
        let done = completed.clone();
        let worker = std::thread::Builder::new()
            .name("otter-workflow-sync".into())
            .spawn(move || {
                let _completion = CompletionSignal::new(done, notify.clone());
                let report = match ready.batch(&signal) {
                    Ok(mut batch) => {
                        dispatch::run(&mut batch, &identity, &budget, limit, &signal, |request| {
                            admit(&tx, request.id(), request.workflow_id(), &signal, &notify)
                        })
                    }
                    Err(error) => dispatch::Report {
                        acknowledged: 0,
                        failure: Some(dispatch::Failure::Prepare(error)),
                        may_have_reached_remote: false,
                    },
                };
                Delivery { ready, report }
            })?;
        Ok(Self {
            cancel,
            completed,
            admissions,
            worker: Some(worker),
        })
    }
    pub fn take_admission(&self) -> Option<Admission> {
        self.admissions.try_recv().ok()
    }
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Release);
    }
    /// 不把取消后的部分成功报告覆盖为笼统 cancelled，远端已确认状态不能丢失。
    pub fn poll(&mut self) -> Option<Result<Delivery, &'static str>> {
        if !self
            .worker
            .as_ref()
            .is_some_and(|w| self.completed.load(Ordering::Acquire) || w.is_finished())
        {
            return None;
        }
        Some(
            self.worker
                .take()?
                .join()
                .map_err(|_| "Workflow sync worker panicked"),
        )
    }
}
impl Drop for Job {
    fn drop(&mut self) {
        self.cancel();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
fn admit(
    tx: &SyncSender<Admission>,
    request_id: &str,
    workflow_id: &str,
    cancel: &AtomicBool,
    notify: &Arc<dyn Fn() + Send + Sync>,
) -> bool {
    if cancel.load(Ordering::Acquire) {
        return false;
    }
    let (reply, rx) = mpsc::sync_channel(1);
    if tx
        .try_send(Admission {
            request_id: request_id.into(),
            workflow_id: workflow_id.into(),
            reply,
        })
        .is_err()
    {
        return false;
    }
    // 邮箱先发布再唤醒；通知 panic 与 UI 无应答均按拒绝处理，不永久挂起 worker。
    if std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| notify())).is_err() {
        return false;
    }
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if cancel.load(Ordering::Acquire) {
            return false;
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return false;
        }
        match rx.recv_timeout(remaining.min(Duration::from_millis(100))) {
            Ok(allowed) => return allowed && !cancel.load(Ordering::Acquire),
            Err(mpsc::RecvTimeoutError::Timeout) => (),
            Err(mpsc::RecvTimeoutError::Disconnected) => return false,
        }
    }
}

#[cfg(test)]
#[path = "session_workflow_sync_tests.rs"]
mod tests;
