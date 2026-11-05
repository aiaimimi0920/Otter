//! 应用层单文件 worker；配置凭证留在 UI owner，线程只接收路径与取消标志。
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
    pub fn poll(&mut self) -> Option<Result<Loaded, String>> {
        if !self.ready.load(Ordering::Acquire) && !self.worker.as_ref()?.is_finished() {
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
        // 文件系统阻塞调用不能强制中断；退出时等待回收，不留下无 owner 线程。
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
