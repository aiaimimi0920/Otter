//! 归档的 UI 提交边界：后台只暂存，复核 Receipt 后才能替换磁盘和保存基线。
#[path = "application_workflow_archive_smoke.rs"]
pub(in crate::desktop::application) mod smoke;
use super::*;
use crate::desktop::session_candidate::{WorkflowArchiveJob, WorkflowArchiveStaged};
pub(in crate::desktop::application) struct Pending {
    receipt: Option<Receipt>,
    job: WorkflowArchiveJob,
}
pub(in crate::desktop::application) struct Prepared {
    receipt: Receipt,
    staged: WorkflowArchiveStaged,
}
impl Candidate {
    pub(in crate::desktop::application) unsafe fn start_archive(
        self,
        app: &Application,
        root: Arc<Root>,
        timestamp_ms: u64,
    ) -> Result<Pending> {
        if !self.matches(app) {
            return Err(Error::new(
                E_ABORT,
                "Workflow archive candidate became stale",
            ));
        }
        let destination = app
            .restored_session
            .as_ref()
            .and_then(|record| record.saved_path.clone())
            .ok_or_else(|| {
                Error::new(E_ABORT, "Workflow archive requires committed destination")
            })?;
        let job = WorkflowArchiveJob::start(
            self.ready,
            root,
            destination,
            timestamp_ms,
            app.execution_wake.callback(),
        )
        .map_err(|_| Error::new(E_FAIL, "Cannot start workflow archive worker"))?;
        Ok(Pending {
            receipt: Some(self.receipt),
            job,
        })
    }
}
impl Pending {
    pub(in crate::desktop::application) unsafe fn poll(
        &mut self,
        app: &Application,
    ) -> Option<Result<Prepared>> {
        if !self.receipt.as_ref()?.matches(app) {
            self.job.cancel();
        }
        let result = self.job.poll()?;
        let receipt = self.receipt.take()?;
        Some(
            result
                .map(|staged| Prepared { receipt, staged })
                .map_err(|message| Error::new(E_ABORT, message)),
        )
    }
}
impl Prepared {
    pub(in crate::desktop::application) unsafe fn commit(
        self,
        app: &mut Application,
    ) -> Result<crate::session_file_replace::Published> {
        if !self.receipt.matches(app) {
            return Err(Error::new(
                E_ABORT,
                "Workflow archive changed before commit",
            ));
        }
        let stamp = save_state::Stamp::capture(app)?;
        // 检查与磁盘提交间不泵消息。失败不交换基线；恢复路径必须保留在错误中。
        let committed = self
            .staged
            .commit(&std::sync::atomic::AtomicBool::new(false))
            .map_err(|failure| {
                Error::new(
                    E_FAIL,
                    format!(
                        "Workflow archive commit failed: {}; recovery={:?}; restore={:?}",
                        failure.error, failure.recovery, failure.restore_error
                    ),
                )
            })?;
        let record = app
            .restored_session
            .as_mut()
            .expect("verified workflow archive baseline");
        record.original = committed.prepared.candidate().bytes.clone().into();
        record.disk_original = None;
        record.saved_path = Some(committed.publication.destination.clone());
        app.saved_state = Some(stamp);
        // 新 revision 必须重新准备工作流快照，不能发送归档前 Ready 的旧 revision。
        Ok(committed.publication)
    }
}
