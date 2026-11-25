//! 按需的单次扩展执行 owner；使用独立认证连接，不借用目录订阅 socket，不自动重放。
use super::{
    capability_job::Source,
    extension_command_bridge::{Mailbox, Packet, Port},
    extension_command_flow::{self, Spec},
    loom_execution_signal::{reap_deadline, CompletionSignal, Notify},
};
use crate::loom_extension_session::{Error, Session};
use std::{
    sync::{atomic::AtomicBool, Arc, Mutex},
    thread::{self, JoinHandle},
};
pub(super) struct Job {
    port: Port,
    ready: Arc<AtomicBool>,
    worker: Option<JoinHandle<Result<(), String>>>,
}
struct Published(Arc<Mutex<Mailbox>>);
impl Drop for Published {
    fn drop(&mut self) {
        let mut mailbox = self.0.lock().unwrap_or_else(|e| e.into_inner());
        mailbox.packet = None;
        mailbox.interrupt = None;
    }
}
impl Job {
    pub(super) fn start(
        source: Source,
        spec: Spec,
        notify: Notify,
        cancel: Arc<AtomicBool>,
    ) -> Result<Self, String> {
        let port = Port {
            mailbox: Arc::new(Mutex::new(Mailbox::default())),
            cancel,
            notify: notify.clone(),
        };
        let worker_port = port.clone();
        let ready = Arc::new(AtomicBool::new(false));
        let completed = ready.clone();
        let worker = thread::Builder::new()
            .name("otter-extension-command".into())
            .spawn(move || {
                let _completion = CompletionSignal::new(completed, notify);
                // 正常结果和 panic 都先清理仍在邮箱中的请求，完成通知后不能再领取遗留 Apply。
                let _published = Published(worker_port.mailbox.clone());
                (|| {
                    worker_port.check().map_err(diagnostic)?;
                    let identity = match source {
                        Source::Fixed(identity) => identity,
                        Source::Discovery(config) => config
                            .load()
                            .map_err(|error| format!("extension discovery: {error:?}"))?,
                    };
                    let mut session =
                        Session::open(&identity, &worker_port.cancel).map_err(diagnostic)?;
                    {
                        let mut mailbox = worker_port
                            .mailbox
                            .lock()
                            .unwrap_or_else(|e| e.into_inner());
                        worker_port.check().map_err(diagnostic)?;
                        mailbox.interrupt = Some(session.interrupt().map_err(diagnostic)?);
                    }
                    extension_command_flow::run(&mut session, spec, &worker_port)
                        .map_err(diagnostic)
                })()
            })
            .map_err(|e| e.to_string())?;
        Ok(Self {
            port,
            ready,
            worker: Some(worker),
        })
    }
    pub(super) fn take_request(&self) -> Option<Packet> {
        self.port
            .mailbox
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .packet
            .take()
    }
    #[cfg(test)]
    pub(super) fn request_pending(&self) -> bool {
        self.port
            .mailbox
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .packet
            .is_some()
    }
    #[cfg(test)]
    pub(super) fn apply_pending(&self) -> bool {
        use super::extension_command_bridge::Request;
        self.port
            .mailbox
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .packet
            .as_ref()
            .is_some_and(|packet| matches!(&packet.request, Request::Apply { .. }))
    }
    pub(super) fn poll(&mut self) -> Option<Result<(), String>> {
        let worker = self.worker.as_ref()?;
        if !worker.is_finished() {
            return None;
        }
        Some(
            self.worker
                .take()
                .unwrap()
                .join()
                .unwrap_or_else(|_| Err("extension worker failed".into())),
        )
    }
    pub(super) fn deadline(&self) -> Option<std::time::Instant> {
        self.worker
            .as_ref()
            .and_then(|_| reap_deadline(&self.ready))
    }
    #[cfg(test)]
    pub(super) fn test_wait_finished(&self) {
        super::loom_execution_tasks::reap_fixture::wait_finished(self.worker.as_ref().unwrap());
    }
    pub(super) fn cancel(&self) {
        self.port.cancel();
    }
}
fn diagnostic(error: Error) -> String {
    format!("extension command: {error:?}")
}
impl Drop for Job {
    fn drop(&mut self) {
        self.cancel();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
#[cfg(test)]
#[path = "extension_command_reap_tests.rs"]
mod reap_tests;
