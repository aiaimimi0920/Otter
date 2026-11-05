//! 保存准备结果继续持有 UI Receipt；临时文件写完后、最终提交前再次核对。
use super::*;
use crate::{session_file_write::Staged, session_write::Candidate};
use std::{path::PathBuf, sync::atomic::AtomicBool};

pub(in crate::desktop) struct Prepared {
    pub(super) checkpoint_senders:
        Option<crate::projection_delivery::sender::binding::checkpoint::Index>,
    pub(super) checkpoint_scope: Option<crate::session_projection_scope::Proof>,
    pub(super) checkpoint_projections:
        Option<crate::projection_delivery::binding::checkpoint::Index>,
    pub(super) candidate: Candidate,
    pub(super) staged: Option<save_disk::Disk>,
    pub(super) receipt: save_snapshot::Receipt,
}
impl Prepared {
    unsafe fn current(&self, app: &Application, staged: &Staged) -> bool {
        staged.is_for(&self.candidate) && self.receipt.matches(app)
    }
    /// UI 线程不得在校验与 rename 之间泵消息；成功后同步交换基线，失败不改旧记录。
    pub(super) unsafe fn commit_new(
        self,
        app: &mut Application,
        staged: Staged,
        cancel: &AtomicBool,
    ) -> Result<PathBuf> {
        self.commit_disk(app, save_disk::Disk::New(staged), cancel)
            .map(|published| published.destination)
    }
    pub(super) unsafe fn commit_ready(
        mut self,
        app: &mut Application,
        cancel: &AtomicBool,
    ) -> Result<crate::session_file_replace::Published> {
        let disk = self
            .staged
            .take()
            .ok_or_else(|| Error::new(E_FAIL, "Missing prepared session file"))?;
        self.commit_disk(app, disk, cancel)
    }
    unsafe fn commit_disk(
        self,
        app: &mut Application,
        disk: save_disk::Disk,
        cancel: &AtomicBool,
    ) -> Result<crate::session_file_replace::Published> {
        if !disk.is_for(&self.candidate)
            || !self.receipt.matches(app)
            || self
                .checkpoint_senders
                .as_ref()
                .is_some_and(|index| !index.is_for(&self.candidate.bytes.clone().into()))
            || self.checkpoint_scope.as_ref().is_none_or(|proof| {
                !proof.is_for(&self.candidate.bytes.clone().into())
                    || app.restored_session.as_ref().unwrap().projection_scope
                        != Some(proof.scope())
            })
            || self
                .checkpoint_projections
                .as_ref()
                .is_some_and(|index| !index.is_for(&self.candidate.bytes.clone().into()))
        {
            return Err(Error::new(
                E_ABORT,
                "Save candidate or workspace changed before commit",
            ));
        }
        let promote_host = if let Some(host) = app
            .restored_session
            .as_ref()
            .and_then(|record| record.transient_host.as_ref())
        {
            crate::session_document::decode(&self.candidate.bytes)
                .map_err(|_| Error::new(E_FAIL, "Invalid candidate baseline"))?
                .workspace
                .contains(host)
        } else {
            false
        };
        let saved_state = save_state::Stamp::capture_snapshot(app, &self.receipt)?;
        let published = disk.commit(cancel).map_err(|failure| {
            let mut message = format!("Cannot commit session: {}", failure.error);
            if let Some(path) = failure.recovery {
                message.push_str(&format!("\n原会话保留于：{}", path.display()));
            }
            if let Some(error) = failure.restore_error {
                message.push_str(&format!("\nCannot restore original destination: {error}"));
            }
            Error::new(E_FAIL, message)
        })?;
        app.projection_receive
            .committed(self.checkpoint_projections.as_ref(), &app.workspace);
        app.projection_sender
            .committed(self.checkpoint_senders.as_ref(), &app.workspace);
        let record = app
            .restored_session
            .as_mut()
            .expect("receipt verified retained session");
        if record.saved_path.as_ref() != Some(&published.destination) {
            app.workspace_autosave.disable();
        }
        record.original = self.candidate.bytes.into();
        // 仅沿已提交候选推进只读关联身份；手动文件不授默认Checkpoint的协作ack资格。
        record.restored_projections = self.checkpoint_projections;
        record.disk_original = None;
        record.saved_path = Some(published.destination.clone());
        if promote_host {
            record.transient_host = None;
        }
        app.saved_state = Some(saved_state);
        Ok(published)
    }
}

/// 隐藏回归只在 linshi 发布合成工作区，不改写用户会话。
pub(super) fn smoke_destination() -> Result<PathBuf> {
    let root = std::path::Path::new(
        "C:/Users/Public/nas_home/AI/GameEditor/linshi/otter-native-save-publish",
    );
    std::fs::create_dir_all(root).map_err(|error| Error::new(E_FAIL, error.to_string()))?;
    let path = root.join(format!("{:?}.json", unsafe {
        windows::Win32::System::Com::CoCreateGuid()?
    }));
    Ok(path)
}

pub(super) unsafe fn verify(app: &mut Application, mut prepared: Prepared) -> Result<()> {
    let cancel = AtomicBool::new(false);
    let path = smoke_destination()?;
    verify_failed_commits(app, &prepared.candidate, &path)?;
    let disk = prepared
        .staged
        .take()
        .ok_or_else(|| Error::new(E_FAIL, "Save worker omitted staged file"))?;
    let save_disk::Disk::New(staged) = disk else {
        return Err(Error::new(E_FAIL, "New save returned replacement"));
    };
    if !prepared.current(app, &staged) {
        return Err(Error::new(E_FAIL, "Fresh staged candidate rejected"));
    }
    let opacity = app.documents[0].state.canvas.borrow().normal_opacity;
    app.documents[0].state.canvas.borrow_mut().normal_opacity =
        if opacity == 0.5 { 0.6 } else { 0.5 };
    let stale_rejected = !prepared.current(app, &staged);
    app.documents[0].state.canvas.borrow_mut().normal_opacity = opacity;
    if !stale_rejected {
        return Err(Error::new(
            E_FAIL,
            "Staged save ignored late workspace change",
        ));
    }
    let expected = prepared.candidate.bytes.clone();
    let revision = prepared.candidate.revision;
    let (old_receipt, _) = app.capture_session_snapshot()?;
    let committed = prepared.commit_new(app, staged, &cancel)?;
    let result = std::fs::read(&committed);
    // 本路径由本次 GUID 创建且只在 linshi；不遍历或删除其他文件。
    if result.map_err(|error| Error::new(E_FAIL, error.to_string()))? != expected.as_ref() {
        return Err(Error::new(
            E_FAIL,
            "Published session differs from staged candidate",
        ));
    }
    let record = app.restored_session.as_ref().unwrap();
    let decoded = crate::session_document::decode(&record.original)
        .map_err(|_| Error::new(E_FAIL, "Cannot decode saved coordinate metadata"))?;
    if !decoded
        .coordinate_space
        .zip(record.space)
        .is_some_and(|(actual, expected)| {
            actual.origin == expected.origin && actual.scale == expected.scale
        })
    {
        return Err(Error::new(
            E_FAIL,
            "Saved coordinate metadata differs from snapshot",
        ));
    }
    if old_receipt.matches(app)
        || !record
            .original
            .same_storage(&crate::session_source::Source::from(expected))
        || record.saved_path.as_ref() != Some(&committed)
    {
        return Err(Error::new(
            E_FAIL,
            "Published session did not adopt exact baseline",
        ));
    }
    let (_, next) = app.capture_session_snapshot()?;
    let next = next
        .encode(&cancel)
        .map_err(|_| Error::new(E_FAIL, "Cannot encode next saved revision"))?;
    if next.expected_revision != revision || next.revision != revision + 1 {
        return Err(Error::new(
            E_FAIL,
            "Next save did not advance committed baseline",
        ));
    }
    restore_batch::workflow::smoke::verify(app)?;
    let replaced = save_replace_smoke::verify(app, &committed);
    std::fs::remove_file(&committed).map_err(|error| Error::new(E_FAIL, error.to_string()))?;
    replaced?;
    println!("native-save-publish-smoke: candidate-bound=true late-stale-rejected=true new-file-roundtrip=true failed-baseline-preserved=true");
    Ok(())
}

/// 实际走提交失败分支，而不只测试 current；目标仅为上方生成的 linshi GUID 路径。
unsafe fn verify_failed_commits(
    app: &mut Application,
    candidate: &Candidate,
    path: &std::path::Path,
) -> Result<()> {
    let io_error = |error: std::io::Error| Error::new(E_FAIL, error.to_string());
    for mode in 0..3 {
        let (receipt, _) = app.capture_session_snapshot()?;
        let (unchanged, _) = app.capture_session_snapshot()?;
        let record = app.restored_session.as_ref().unwrap();
        let old_path = record.saved_path.clone();
        let old_host = record.transient_host.clone();
        let prepared = Prepared {
            checkpoint_senders: None,
            checkpoint_scope: crate::session_projection_scope::Proof::capture(
                &candidate.bytes.clone().into(),
            ),
            checkpoint_projections: None,
            staged: None,
            candidate: Candidate {
                bytes: candidate.bytes.clone(),
                expected_revision: candidate.expected_revision,
                revision: candidate.revision,
            },
            receipt,
        };
        let cancel = AtomicBool::new(false);
        let staged = Staged::prepare(path, &prepared.candidate, &cancel).map_err(io_error)?;
        let opacity = app.documents[0].state.canvas.borrow().normal_opacity;
        if mode == 0 {
            cancel.store(true, std::sync::atomic::Ordering::Release);
        } else if mode == 1 {
            app.documents[0].state.canvas.borrow_mut().normal_opacity =
                if opacity == 0.5 { 0.6 } else { 0.5 };
        } else {
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(path)
                .map_err(io_error)?;
            std::io::Write::write_all(&mut file, b"existing session sentinel").map_err(io_error)?;
        }
        let result = prepared.commit_new(app, staged, &cancel);
        app.documents[0].state.canvas.borrow_mut().normal_opacity = opacity;
        let target_preserved = if mode == 2 {
            let bytes = std::fs::read(path).map_err(io_error)?;
            std::fs::remove_file(path).map_err(io_error)?;
            bytes == b"existing session sentinel"
        } else {
            !path.try_exists().map_err(io_error)?
        };
        let record = app.restored_session.as_ref().unwrap();
        if result.is_ok()
            || !target_preserved
            || !unchanged.matches(app)
            || record.saved_path != old_path
            || record.transient_host != old_host
        {
            return Err(Error::new(E_FAIL, "Failed save changed target or baseline"));
        }
    }
    Ok(())
}
