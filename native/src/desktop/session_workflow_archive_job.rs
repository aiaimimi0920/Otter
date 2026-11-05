//! 归档后台阶段：资源发布、会话组装与精确基线暂存；提交权限留给 UI Receipt owner。
use super::{CompletionSignal, Ready};
use crate::{
    desktop::session_asset_root::{
        archive::{self, Prepared},
        Root,
    },
    session_file_replace::{Failure, Published, Replacement},
};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::JoinHandle,
};

pub struct Staged {
    // 先清理会话暂存/释放基线锁，再归还资源句柄和目录 owner。
    replacement: Replacement,
    prepared: Prepared,
    root: Arc<Root>,
}
pub struct Committed {
    pub prepared: Prepared,
    pub publication: Published,
    pub root: Arc<Root>,
}
impl Staged {
    pub fn candidate(&self) -> &crate::session_write::Candidate {
        self.prepared.candidate()
    }
    /// 调用方须在 UI 验证 Receipt 后、不泵消息地执行，并只在成功后交换保存基线。
    /// 返回恢复文件信息不能被丢弃；返回失败不删除已发布的图片资源。
    pub fn commit(self, cancel: &AtomicBool) -> Result<Committed, Failure> {
        let publication = self.replacement.commit(cancel)?;
        Ok(Committed {
            prepared: self.prepared,
            publication,
            root: self.root,
        })
    }
}
pub struct Job {
    cancel: Arc<AtomicBool>,
    completed: Arc<AtomicBool>,
    worker: Option<JoinHandle<Result<Staged, String>>>,
}
/// 归档不发送图片快照；源字节租约必须继续存活，不能随烘焙快照一起提前归还。
struct ArchiveInput {
    source: crate::session_source::Source,
    metadata: crate::session_document::workflow_snapshot::metadata::Plan,
    _source_lease: Option<crate::byte_budget::Lease>,
}
impl ArchiveInput {
    fn from_ready(ready: Ready) -> Self {
        let Ready {
            prepared,
            source,
            metadata,
            _source_lease,
        } = ready;
        // 在 worker 内释放，避免 UI 为失效的大快照承担析构；新 revision 将重新准备。
        drop(prepared);
        Self {
            source,
            metadata,
            _source_lease,
        }
    }
}
impl Job {
    pub fn start(
        ready: Ready,
        root: Arc<Root>,
        destination: PathBuf,
        timestamp_ms: u64,
        notify: Arc<dyn Fn() + Send + Sync>,
    ) -> std::io::Result<Self> {
        let cancel = Arc::new(AtomicBool::new(false));
        let completed = Arc::new(AtomicBool::new(false));
        let signal = cancel.clone();
        let done = completed.clone();
        let worker = std::thread::Builder::new()
            .name("otter-workflow-archive".into())
            .spawn(move || {
                let _completion = CompletionSignal::new(done, notify);
                if signal.load(Ordering::Acquire) {
                    return Err("Workflow archive cancelled".into());
                }
                let input = ArchiveInput::from_ready(ready);
                let batch =
                    archive::prepare(&root, &input.metadata, &input.source, timestamp_ms, &signal)
                        .map_err(|_| "Workflow resource archive failed")?;
                let prepared = batch
                    .into_candidate(&input.source, &signal)
                    .map_err(|_| "Workflow archive session assembly failed")?;
                let replacement = Replacement::prepare(
                    &destination,
                    &input.source,
                    prepared.candidate(),
                    &signal,
                )
                .map_err(|_| "Workflow archive baseline or staging failed")?;
                Ok(Staged {
                    replacement,
                    prepared,
                    root,
                })
            })?;
        Ok(Self {
            cancel,
            completed,
            worker: Some(worker),
        })
    }
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Release);
    }
    pub fn poll(&mut self) -> Option<Result<Staged, String>> {
        if !self
            .worker
            .as_ref()
            .is_some_and(|w| self.completed.load(Ordering::Acquire) || w.is_finished())
        {
            return None;
        }
        let result = self
            .worker
            .take()?
            .join()
            .unwrap_or_else(|_| Err("Workflow archive worker panicked".into()));
        Some(if self.cancel.load(Ordering::Acquire) {
            Err("Workflow archive cancelled".into())
        } else {
            result
        })
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

#[cfg(test)]
#[path = "session_workflow_archive_job_tests.rs"]
mod tests;
