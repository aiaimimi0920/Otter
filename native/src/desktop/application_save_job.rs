//! 保存候选 worker；UI Receipt 不进入线程，完成通知不代表允许提交。
use super::super::save_disk::{Destination, Disk};
use super::Data;
use crate::{desktop::loom_execution_signal::CompletionSignal, session_write::Candidate};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::JoinHandle,
};

pub struct Output {
    pub checkpoint_senders: Option<crate::projection_delivery::sender::binding::checkpoint::Index>,
    pub checkpoint_scope: Option<crate::session_projection_scope::Proof>,
    pub checkpoint_projections: Option<crate::projection_delivery::binding::checkpoint::Index>,
    pub library_changed: bool,
    pub candidate: Candidate,
    pub staged: Option<Disk>,
}
/// 回收候选会移除活动窗口，不能被仅更新库原文或磁盘保存的消费者领取。
enum Purpose {
    Save,
    Checkpoint,
    LibraryUpdate,
    Recycle(crate::workspace_id::UnitId),
}
#[path = "application_save_recycle_job.rs"]
mod recycle;
pub use recycle::Request as RecycleRequest;
pub struct Job {
    prune_policy: Option<crate::session_library::recycle::policy::Policy>,
    purpose: Purpose,
    cancel: Arc<AtomicBool>,
    ready: Arc<AtomicBool>,
    worker: Option<JoinHandle<Result<Output, String>>>,
}
impl Job {
    pub fn prune_policy(&self) -> Option<crate::session_library::recycle::policy::Policy> {
        self.prune_policy
    }
    pub fn is_library_update(&self) -> bool {
        matches!(self.purpose, Purpose::LibraryUpdate)
    }
    pub fn is_save(&self) -> bool {
        matches!(self.purpose, Purpose::Save)
    }
    pub fn is_checkpoint(&self) -> bool {
        matches!(self.purpose, Purpose::Checkpoint)
    }
    pub fn start_checkpoint(
        mut data: Data,
        destination: Destination,
        revision: u64,
        notify: Arc<dyn Fn() + Send + Sync>,
    ) -> std::io::Result<Self> {
        data.disk_revision = Some(revision);
        let mut job = Self::start(data, Some(destination), notify)?;
        job.purpose = Purpose::Checkpoint;
        Ok(job)
    }
    pub fn recycle_unit(&self) -> Option<&crate::workspace_id::UnitId> {
        match &self.purpose {
            Purpose::Recycle(unit) => Some(unit),
            _ => None,
        }
    }
    pub fn start_reference(
        data: Data,
        unit: crate::workspace_id::UnitId,
        remove: bool,
        entry_id: String,
        created_at: String,
        notify: Arc<dyn Fn() + Send + Sync>,
    ) -> std::io::Result<Self> {
        let mut job = Self::start_with(notify, move |cancel| {
            let original = data.original.clone();
            let revision = crate::session_document::decode(&original)
                .map_err(|error| format!("Invalid reference source: {error:?}"))?
                .revision;
            let snapshot = if remove {
                None
            } else {
                Some(
                    data.encode(cancel)
                        .map_err(|error| format!("Reference snapshot: {error:?}"))?,
                )
            };
            let bytes = crate::session_library::reference::prepare(
                &original,
                snapshot.as_ref().map(|candidate| candidate.bytes.as_ref()),
                &unit,
                &entry_id,
                &created_at,
                cancel,
            )?;
            Ok(Output {
                checkpoint_senders: None,
                checkpoint_scope: None,
                checkpoint_projections: None,
                library_changed: true,
                candidate: Candidate {
                    bytes,
                    expected_revision: revision,
                    revision,
                },
                staged: None,
            })
        })?;
        job.purpose = Purpose::LibraryUpdate;
        Ok(job)
    }
    pub fn start_library_remove(
        source: crate::session_source::Source,
        kind: crate::session_library::Kind,
        entry_id: String,
        notify: Arc<dyn Fn() + Send + Sync>,
    ) -> std::io::Result<Self> {
        let mut job = Self::start_with(notify, move |cancel| {
            let revision = crate::session_document::decode(&source)
                .map_err(|error| format!("Invalid frozen removal source: {error:?}"))?
                .revision;
            let bytes = crate::session_library::remove::prepare(&source, kind, &entry_id, cancel)?;
            Ok(Output {
                checkpoint_senders: None,
                checkpoint_scope: None,
                checkpoint_projections: None,
                library_changed: true,
                candidate: Candidate {
                    bytes,
                    expected_revision: revision,
                    revision,
                },
                staged: None,
            })
        })?;
        job.purpose = Purpose::LibraryUpdate;
        Ok(job)
    }
    pub fn start(
        data: Data,
        destination: Option<Destination>,
        notify: Arc<dyn Fn() + Send + Sync>,
    ) -> std::io::Result<Self> {
        Self::start_with(notify, move |cancel| {
            let candidate = data
                .encode(cancel)
                .map_err(|error| format!("Save encoding failed: {error:?}"))?;
            // 编码与磁盘暂存共用一个取消令牌/任务槽；取消后的 Staged 由 owner 清理。
            let staged = destination
                .map(|destination| destination.prepare(&candidate, cancel))
                .transpose()
                .map_err(|error| format!("Save staging failed: {error}"))?;
            Ok(Output {
                checkpoint_senders:
                    crate::projection_delivery::sender::binding::checkpoint::Index::capture(
                        &candidate.bytes.clone().into(),
                    ),
                checkpoint_scope: crate::session_projection_scope::Proof::capture(
                    &candidate.bytes.clone().into(),
                ),
                // 原 Save 候选即使稍后才选择目标，也可经 commit_new 真正落盘；库更新不走这里。
                checkpoint_projections:
                    crate::projection_delivery::binding::checkpoint::Index::capture(
                        &candidate.bytes.clone().into(),
                    ),
                candidate,
                staged,
                library_changed: true,
            })
        })
    }
    fn start_with(
        notify: Arc<dyn Fn() + Send + Sync>,
        work: impl FnOnce(&AtomicBool) -> Result<Output, String> + Send + 'static,
    ) -> std::io::Result<Self> {
        let cancel = Arc::new(AtomicBool::new(false));
        let ready = Arc::new(AtomicBool::new(false));
        let signal = cancel.clone();
        let completed = ready.clone();
        let worker = std::thread::Builder::new()
            .name("otter-session-save".into())
            .spawn(move || {
                let _completion = CompletionSignal::new(completed, notify);
                work(&signal)
            })?;
        Ok(Self {
            prune_policy: None,
            purpose: Purpose::Save,
            cancel,
            ready,
            worker: Some(worker),
        })
    }
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Release);
    }
    pub fn poll(&mut self) -> Option<Result<Output, String>> {
        if !self.is_ready() {
            return None;
        }
        let result = self
            .worker
            .take()?
            .join()
            .unwrap_or_else(|_| Err("Save worker panicked".into()));
        Some(if self.cancel.load(Ordering::Acquire) {
            Err("Save preparation cancelled".into())
        } else {
            result
        })
    }
    /// 仅观察完成状态，不消费结果或释放磁盘准备 owner。
    pub fn is_ready(&self) -> bool {
        self.worker
            .as_ref()
            .is_some_and(|worker| self.ready.load(Ordering::Acquire) || worker.is_finished())
    }
}
impl Drop for Job {
    fn drop(&mut self) {
        self.cancel();
        // WIC/字体调用不可强制打断；退出时等待其边界回收，禁止遗留无 owner 线程。
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session_file_write::Staged;
    fn candidate() -> Candidate {
        Candidate {
            bytes: vec![1].into(),
            expected_revision: 1,
            revision: 2,
        }
    }
    #[test]
    fn completion_is_delivered_once_and_late_cancel_discards_ready_candidate() {
        for cancelled in [false, true] {
            let (tx, rx) = std::sync::mpsc::channel();
            let mut job = Job::start_with(
                Arc::new(move || {
                    let _ = tx.send(());
                }),
                |_| {
                    Ok(Output {
                        checkpoint_senders: None,
                        checkpoint_scope: None,
                        checkpoint_projections: None,
                        library_changed: true,
                        candidate: candidate(),
                        staged: None,
                    })
                },
            )
            .unwrap();
            rx.recv_timeout(std::time::Duration::from_secs(5)).unwrap();
            if cancelled {
                job.cancel();
            }
            assert_eq!(job.poll().unwrap().is_err(), cancelled);
            assert!(job.poll().is_none());
        }
    }
    #[test]
    fn panic_notifies_and_drop_cancels_and_joins_worker() {
        let (tx, rx) = std::sync::mpsc::channel();
        let mut job = Job::start_with(
            Arc::new(move || {
                let _ = tx.send(());
            }),
            |_| panic!("save test panic"),
        )
        .unwrap();
        rx.recv_timeout(std::time::Duration::from_secs(5)).unwrap();
        assert_eq!(
            job.poll().unwrap().err().as_deref(),
            Some("Save worker panicked")
        );
        let (tx, rx) = std::sync::mpsc::channel();
        let job = Job::start_with(Arc::new(|| {}), move |cancel| {
            while !cancel.load(Ordering::Acquire) {
                std::thread::yield_now();
            }
            tx.send(()).unwrap();
            Err("cancelled".into())
        })
        .unwrap();
        drop(job);
        rx.recv_timeout(std::time::Duration::from_secs(5)).unwrap();
    }

    #[test]
    fn late_cancel_or_drop_reclaims_completed_staging_without_publishing() {
        let root = std::path::Path::new(
            "C:/Users/Public/nas_home/AI/GameEditor/linshi/otter-native-save-job-tests",
        );
        std::fs::create_dir_all(root).unwrap();
        for poll_cancelled in [false, true] {
            let directory = root.join(format!("{:?}", unsafe {
                windows::Win32::System::Com::CoCreateGuid().unwrap()
            }));
            std::fs::create_dir(&directory).unwrap();
            let destination = directory.join("session.json");
            let target = destination.clone();
            let (tx, rx) = std::sync::mpsc::channel();
            let mut job = Job::start_with(
                Arc::new(move || { let _ = tx.send(()); }),
                move |cancel| {
                    let candidate = Candidate {
                        bytes: br#"{"documentSchemaVersion":1,"documentRevision":1,"stickers":[],"links":[]}"#.to_vec().into(),
                        expected_revision: 0,
                        revision: 1,
                    };
                    let staged = Staged::prepare(&target, &candidate, cancel)
                        .map_err(|error| error.to_string())?;
                    Ok(Output { checkpoint_senders: None, checkpoint_scope: None, checkpoint_projections: None, candidate, staged: Some(Disk::New(staged)), library_changed: true })
                },
            ).unwrap();
            rx.recv_timeout(std::time::Duration::from_secs(5)).unwrap();
            assert!(!destination.exists());
            assert_eq!(std::fs::read_dir(&directory).unwrap().count(), 1);
            if poll_cancelled {
                job.cancel();
                assert!(job.poll().unwrap().is_err());
            }
            drop(job);
            assert!(!destination.exists());
            assert_eq!(std::fs::read_dir(&directory).unwrap().count(), 0);
            // 仅移除本测试在 linshi 创建的空目录，不递归清理。
            std::fs::remove_dir(directory).unwrap();
        }
    }
}
