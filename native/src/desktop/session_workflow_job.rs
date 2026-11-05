//! 工作流准备 worker：无窗口/网络权限，结果不等于保存提交或发送授权。
use super::*;
use crate::desktop::{loom_execution_signal::CompletionSignal, session_asset_root::Root};
use std::{sync::Arc, thread::JoinHandle};
#[path = "session_workflow_archive_job.rs"]
pub mod archive;
#[path = "session_workflow_sync.rs"]
pub mod sync;

pub struct Input {
    pub saved: Vec<u8>,
    pub revision: u64,
    // 只能传显式准入的根目录 owner，不能从会话内的路径自行扩权。
    pub root: Option<Arc<Root>>,
    pub catalog: Vec<serde_json::Value>,
}
pub struct Ready {
    prepared: Prepared,
    source: crate::session_source::Source,
    metadata: workflow_snapshot::metadata::Plan,
    _source_lease: Option<crate::byte_budget::Lease>,
}
impl Ready {
    pub fn batch(
        &self,
        cancel: &AtomicBool,
    ) -> std::result::Result<workflow_snapshot::batch::Batch<'_>, workflow_snapshot::Error> {
        workflow_snapshot::batch::Batch::prepare(self.snapshot(), &self.source, cancel)
    }
    pub fn snapshot(&self) -> &Snapshot {
        self.prepared.snapshot()
    }
    pub fn metadata(&self) -> &workflow_snapshot::metadata::Plan {
        &self.metadata
    }
    pub fn matches_source(&self, saved: &[u8]) -> bool {
        self.metadata.matches(saved)
    }
    pub fn components(
        &self,
        cancel: &AtomicBool,
    ) -> std::result::Result<
        Vec<workflow_snapshot::components::Component<'_>>,
        workflow_snapshot::Error,
    > {
        workflow_snapshot::components::prepare(self.snapshot(), &self.source, cancel)
    }
}
fn prepare(input: Input, cancel: &AtomicBool) -> std::result::Result<Ready, String> {
    let _apartment = crate::desktop::Apartment::new().map_err(|e| e.to_string())?;
    let candidate =
        crate::desktop::session_candidate::prepare(input.saved, input.root.as_deref(), cancel)
            .map_err(|e| e.to_string())?;
    if input.revision == 0 || candidate.revision != input.revision {
        return Err("Workflow saved revision mismatch".into());
    }
    let candidate = VectorCandidate::prepare(candidate, cancel).map_err(|e| e.to_string())?;
    let prepared = candidate
        .prepare_workflow_snapshot_with_root(&input.catalog, input.root.as_deref(), cancel)
        .map_err(|e| e.to_string())?;
    let metadata = workflow_snapshot::metadata::prepare(
        prepared.snapshot(),
        candidate.candidate.original_json(),
        cancel,
    )
    .map_err(|e| format!("Workflow metadata preparation failed: {e:?}"))?;
    check(cancel).map_err(|e| e.to_string())?;
    // 移动原始字节，不为结果再复制含大图片的会话；恢复图像/编辑快照在此释放。
    Ok(Ready {
        prepared,
        source: candidate.candidate.original,
        metadata,
        _source_lease: None,
    })
}

pub struct Job {
    cancel: Arc<AtomicBool>,
    ready: Arc<AtomicBool>,
    worker: Option<JoinHandle<std::result::Result<Ready, String>>>,
}
impl Job {
    /// UI 只转交共享基线；有界字节复制及 revision 读取均在工作线程进行。
    pub fn start_shared(
        source: crate::session_source::Source,
        root: Option<Arc<Root>>,
        catalog: Vec<serde_json::Value>,
        notify: Arc<dyn Fn() + Send + Sync>,
    ) -> std::io::Result<Self> {
        Self::start_with(notify, move |cancel| {
            check(cancel).map_err(|e| e.to_string())?;
            if source.len() > crate::session_document::MAX_SESSION_BYTES {
                return Err("Workflow source exceeds session limit".into());
            }
            let mut lease =
                crate::pixels::reserve(source.len()).ok_or("Workflow source budget exhausted")?;
            let mut saved = Vec::new();
            saved
                .try_reserve_exact(source.len())
                .map_err(|_| "Cannot allocate workflow source")?;
            if !lease.grow_to(saved.capacity()) {
                return Err("Workflow source capacity exceeds budget".into());
            }
            saved.extend_from_slice(&source);
            let revision = crate::session_document::decode(&saved)
                .map_err(|_| "Invalid workflow source")?
                .revision;
            let mut ready = prepare(
                Input {
                    saved,
                    revision,
                    root,
                    catalog,
                },
                cancel,
            )?;
            ready._source_lease = Some(lease);
            Ok(ready)
        })
    }
    pub fn start(input: Input, notify: Arc<dyn Fn() + Send + Sync>) -> std::io::Result<Self> {
        Self::start_with(notify, move |cancel| prepare(input, cancel))
    }
    fn start_with(
        notify: Arc<dyn Fn() + Send + Sync>,
        work: impl FnOnce(&AtomicBool) -> std::result::Result<Ready, String> + Send + 'static,
    ) -> std::io::Result<Self> {
        let cancel = Arc::new(AtomicBool::new(false));
        let ready = Arc::new(AtomicBool::new(false));
        let signal = cancel.clone();
        let completed = ready.clone();
        let worker = std::thread::Builder::new()
            .name("otter-workflow-prepare".into())
            .spawn(move || {
                let _completion = CompletionSignal::new(completed, notify);
                work(&signal)
            })?;
        Ok(Self {
            cancel,
            ready,
            worker: Some(worker),
        })
    }
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Release);
    }
    pub fn is_ready(&self) -> bool {
        self.worker
            .as_ref()
            .is_some_and(|worker| self.ready.load(Ordering::Acquire) || worker.is_finished())
    }
    pub fn poll(&mut self) -> Option<std::result::Result<Ready, String>> {
        if !self.is_ready() {
            return None;
        }
        let result = self
            .worker
            .take()?
            .join()
            .unwrap_or_else(|_| Err("Workflow preparation worker panicked".into()));
        Some(if self.cancel.load(Ordering::Acquire) {
            Err("Workflow preparation cancelled".into())
        } else {
            result
        })
    }
}
impl Drop for Job {
    fn drop(&mut self) {
        self.cancel();
        // WIC 原生调用不可强制中断；owner 等待取消边界回收，不遗留无主线程。
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

#[cfg(test)]
#[path = "session_workflow_job_tests.rs"]
mod tests;
