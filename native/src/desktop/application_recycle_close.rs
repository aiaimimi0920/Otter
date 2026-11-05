//! 回收关闭的 UI 提交 owner；先准备最后宿主并复核凭证，再转交热键和销毁目标。
use super::*;
#[path = "application_recycle_close_smoke.rs"]
pub(in crate::desktop::application) mod smoke;

impl Application {
    pub(in crate::desktop::application) unsafe fn poll_recycle(
        &mut self,
        visible: bool,
    ) -> Result<()> {
        self.poll_recycle_with(visible, |hwnd| DestroyWindow(hwnd))
    }
    unsafe fn poll_recycle_with(
        &mut self,
        visible: bool,
        close: impl FnOnce(HWND) -> Result<()>,
    ) -> Result<()> {
        self.poll_recycle_using(visible, close, |doc, enabled| {
            crate::desktop::global_capture::set(doc.window.0, &doc.state, enabled)
        })
    }
    unsafe fn poll_recycle_using(
        &mut self,
        visible: bool,
        close: impl FnOnce(HWND) -> Result<()>,
        mut change: impl FnMut(&DocumentWindow, bool) -> Result<()>,
    ) -> Result<()> {
        let Some(unit) = self
            .save_job
            .as_ref()
            .and_then(|(_, job)| job.recycle_unit())
            .cloned()
        else {
            return Ok(());
        };
        let Some(result) = self.save_job.as_mut().and_then(|(_, job)| job.poll()) else {
            return Ok(());
        };
        let (receipt, _) = self.save_job.take().unwrap();
        // process_live 早于此回收点；让保留的下一条 Live 意图无需外部输入即可续进。
        PostMessageW(None, WM_NULL, WPARAM(0), LPARAM(0))?;
        let output = result.map_err(|message| Error::new(E_ABORT, message))?;
        if !receipt.matches(self)
            || output.staged.is_some()
            || output.candidate.revision != output.candidate.expected_revision
        {
            return Err(Error::new(E_ABORT, "Recycle close cancelled or stale"));
        }
        if self
            .live_sessions
            .iter()
            .any(|session| session.unit_id() == &unit)
        {
            return self.commit_live_recycle(&unit, &receipt, output.candidate.bytes.into());
        }
        let index = self
            .documents
            .iter()
            .position(|doc| {
                doc.state.unit_id == unit
                    && doc.is_alive()
                    && doc.state.live_request.borrow().is_none()
            })
            .ok_or_else(|| Error::new(E_ABORT, "Recycle target closed"))?;
        // 隐藏宿主尚未加入工作区，因此准备失败或凭证失效仅析构局部 owner。
        let mut host = if self.documents.len() == 1 {
            Some(self.prepare_window(State::new(None, false)?, false)?)
        } else {
            None
        };
        let next_workspace = host
            .as_ref()
            .map(|doc| {
                self.workspace
                    .replacement_unit(&unit, doc.state.unit_id.clone())
            })
            .transpose()
            .map_err(|error| {
                Error::new(E_FAIL, format!("Cannot replace recycle host: {error:?}"))
            })?;
        let source: crate::session_source::Source = output.candidate.bytes.into();
        self.capture_windows
            .borrow_mut()
            .try_reserve(2)
            .map_err(|_| Error::new(E_OUTOFMEMORY, "Cannot reserve recycle capture registry"))?;
        if !receipt.matches(self) {
            return Err(Error::new(
                E_ABORT,
                "Workspace changed before recycle close",
            ));
        }
        let target = &self.documents[index];
        let next = host
            .as_ref()
            .or_else(|| self.documents.iter().find(|doc| doc.state.unit_id != unit))
            .ok_or_else(|| Error::new(E_ABORT, "Missing recycle successor"))?;
        let transferred = target.state.global_capture.get();
        restore_batch::transfer_hotkey(
            std::slice::from_ref(target),
            std::slice::from_ref(next),
            &mut change,
        )?;
        // 同线程 DestroyWindow 同步完成；失败且目标仍存活时回滚热键，不发布回收候选。
        let closed = close(target.window.0);
        if target.is_alive() {
            let rollback = if transferred {
                restore_batch::transfer_hotkey(
                    std::slice::from_ref(next),
                    std::slice::from_ref(target),
                    &mut change,
                )
            } else {
                Ok(())
            };
            return Err(Error::new(
                E_FAIL,
                format!("Recycle window close failed: {closed:?}; hotkey rollback: {rollback:?}"),
            ));
        }
        // 目标已销毁：不再执行可失败的准备。即使系统返回错误也必须保留其冻结记录。
        let record = self.restored_session.as_mut().unwrap();
        record.edit_original(source);
        record.annotation_ids.retain(|(id, _)| id != &unit);
        if let Some(host) = host.take() {
            record.transient_host = Some(host.state.unit_id.clone());
            let old = std::mem::replace(&mut self.documents[index], host);
            drop(old);
            self.workspace = next_workspace.unwrap();
            // 最后宿主是原位 owner 替换，collect_closed 无法据数量变化发现旧代际条目。
            self.executions
                .retain_units(|id| self.workspace.contains(id));
            self.execution_jobs
                .retain_units(|id| self.workspace.contains(id));
            self.local_arts
                .retire(&self.documents, &self.workspace, &self.live_sessions);
            self.sync_capture_windows();
        } else {
            if record.transient_host.as_ref() == Some(&unit) {
                record.transient_host = None;
            }
            self.collect_closed();
        }
        self.saved_state = None;
        self.sync_groups();
        self.sync_frozen_menu();
        if visible && self.documents.len() == 1 {
            let doc = &self.documents[0];
            if self
                .restored_session
                .as_ref()
                .unwrap()
                .transient_host
                .as_ref()
                == Some(&doc.state.unit_id)
            {
                let _ = ShowWindow(doc.window.0, SW_SHOWNOACTIVATE);
                let _ = ShowWindow(doc.panel.0, SW_SHOWNOACTIVATE);
            }
        }
        Ok(())
    }
}
