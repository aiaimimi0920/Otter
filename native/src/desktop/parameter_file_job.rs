//! Application single-file workers; UI owners retain configuration evidence, while threads receive paths/cancellation flags only.
use super::{
    loom_execution_signal::{CompletionSignal, Notify},
    parameter_file_input::Request,
};
use crate::{workspace::TopologyStamp, workspace_id::UnitId};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::JoinHandle,
};

pub(super) enum Loaded {
    Parameter(String),
    Sticker(Arc<super::sticker_input::Asset>),
}
pub(super) struct Job {
    pub unit: UnitId,
    pub request: Request,
    pub topology: TopologyStamp,
    cancel: Arc<AtomicBool>,
    ready: Arc<AtomicBool>,
    worker: Option<JoinHandle<Result<Loaded, String>>>,
}
impl Job {
    pub fn start(
        unit: UnitId,
        request: Request,
        topology: TopologyStamp,
        notify: Notify,
    ) -> std::io::Result<Self> {
        let cancel = Arc::new(AtomicBool::new(false));
        let ready = Arc::new(AtomicBool::new(false));
        let signal = cancel.clone();
        let completed = ready.clone();
        let path = request.path.clone();
        let target = request.target.clone();
        let worker = std::thread::Builder::new()
            .name("otter-parameter-file".into())
            .spawn(move || {
                let _completion = CompletionSignal::new(completed, notify);
                match target {
                    super::parameter_file_input::Target::Parameter(_) => {
                        crate::parameter_file::load(&path, &signal)
                            .map(Loaded::Parameter)
                            .map_err(|e| e.to_string())
                    }
                    super::parameter_file_input::Target::Sticker => {
                        super::sticker_input::Asset::load(&path, &signal)
                            .map(Loaded::Sticker)
                            .map_err(|e| e.to_string())
                    }
                }
            })?;
        Ok(Self {
            unit,
            request,
            topology,
            cancel,
            ready,
            worker: Some(worker),
        })
    }
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Release);
    }
    pub fn cancelled(&self) -> bool {
        self.cancel.load(Ordering::Acquire)
    }
    pub(super) fn deadline(&self) -> Option<std::time::Instant> {
        self.worker.as_ref()?;
        super::loom_execution_signal::reap_deadline(&self.ready)
    }
    pub fn poll(&mut self) -> Option<Result<Loaded, String>> {
        if !self.worker.as_ref()?.is_finished() {
            return None;
        }
        let result = self
            .worker
            .take()?
            .join()
            .map_err(|_| "文件导入任务异常退出".to_string())
            .and_then(|result| result.map_err(|error| error.to_string()));
        Some(if self.cancel.load(Ordering::Acquire) {
            Err("文件导入已取消".into())
        } else {
            result
        })
    }
}
impl Drop for Job {
    fn drop(&mut self) {
        self.cancel();
        // Blocking filesystem calls cannot be forcibly interrupted; wait for reclamation on exit without unowned threads.
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::desktop::{
        configuration_stamp::Stamp,
        loom_execution_tasks::reap_fixture::{wait_finished, Gate, WAIT},
        parameter_file_input::Target,
    };

    #[test]
    fn cancelled_file_owner_is_not_joined_during_completion_notification() {
        let gate = Gate::new(|| {});
        let state = crate::desktop::State::new(None, false).unwrap();
        // An empty path cannot resolve to an importable file.
        let request = Request {
            path: Default::default(),
            target: Target::Parameter("upload".into()),
            stamp: Stamp::capture(&state),
            view: None,
        };
        let mut job = Job::start(
            state.unit_id.clone(),
            request,
            crate::workspace::Workspace::default().topology_stamp(),
            gate.notify.clone(),
        )
        .unwrap();
        gate.noticed.recv_timeout(WAIT).unwrap();
        job.cancel();
        assert!(job.deadline().is_some());
        assert!(job.poll().is_none());
        gate.release.send(()).unwrap();
        wait_finished(job.worker.as_ref().unwrap());
        assert_eq!(job.poll().unwrap().err().as_deref(), Some("文件导入已取消"));
        assert!(job.poll().is_none());
        assert!(job.deadline().is_none());
    }
}
