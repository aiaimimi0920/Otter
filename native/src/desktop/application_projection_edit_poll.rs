//! HTTP成功先发布原Editor候选，再等待精确canonical包含；ack和下一32项始终串行。
use super::*;
impl Application {
    pub(in crate::desktop::application) unsafe fn poll_projection_edit(
        &mut self,
        config: &mut impl FnMut(
            &str,
        ) -> std::result::Result<
            Config,
            crate::device_authorization_config::Error,
        >,
    ) {
        let phase = std::mem::take(&mut self.projection_edit.0);
        match phase {
            Phase::Sending(mut pending) => {
                let valid = !pending.cancelled && pending.fence.matches(self);
                if !valid {
                    pending.job.request_stop();
                    pending.cancelled = true;
                }
                let Some(result) = pending.job.try_join() else {
                    self.projection_edit.0 = Phase::Sending(pending);
                    return;
                };
                let result = match result {
                    Ok(Output::Edited(ready)) if valid => {
                        self.publish_projection_edit(pending.fence, ready, pending.remaining)
                    }
                    Err(error) => Err(format!("协作提交未确认，保留本地修改和原请求：{error:?}")),
                    Ok(_) => Err("协作提交已过期；保留本地修改和原请求".into()),
                };
                if let Err(error) = result {
                    self.edit_status(&error);
                }
            }
            Phase::Checkpoint(saved) => {
                if !saved.fence.matches(self) {
                    self.edit_status("协作保存目标已变化；内容及原请求保留");
                    return;
                }
                if !saved.durable {
                    self.projection_edit.0 = Phase::Checkpoint(saved);
                    return;
                }
                let Saved {
                    fence,
                    ready,
                    remaining,
                    ..
                } = *saved;
                let result = (|| {
                    let origin = fence.destination.origin();
                    let configuration = config(origin).map_err(|_| stale())?;
                    let lease = self
                        .authorization_owner
                        .lease(origin, now()?)
                        .map_err(|_| stale())?;
                    if !fence.current.same_grant(&lease) {
                        return Err(stale());
                    }
                    Job::start_edit_ack(configuration, lease, ready).map_err(|_| stale())
                })();
                match result {
                    Ok(job) => {
                        self.projection_edit.0 = Phase::Acknowledging(Box::new(Pending {
                            fence,
                            job,
                            cancelled: false,
                            remaining,
                        }));
                        self.edit_status("协作结果已写入检查点，正在确认原请求…");
                    }
                    Err(error) => {
                        self.edit_status(&format!("协作结果已保存；日志确认待重试：{error}"))
                    }
                }
            }
            Phase::Acknowledging(mut pending) => {
                let valid = !pending.cancelled && pending.fence.matches(self);
                if !valid {
                    pending.job.request_stop();
                    pending.cancelled = true;
                }
                let Some(result) = pending.job.try_join() else {
                    self.projection_edit.0 = Phase::Acknowledging(pending);
                    return;
                };
                if matches!(result, Ok(Output::EditAcknowledged)) && valid {
                    self.continue_projection_edit(pending, config);
                } else {
                    self.edit_status("协作结果保留；原请求确认未完成，请显式重试");
                }
            }
            Phase::Idle => {}
            Phase::Source(phase) => self.poll_projection_source(phase, config),
        }
    }
    unsafe fn publish_projection_edit(
        &mut self,
        mut fence: Fence,
        ready: Box<Ready>,
        remaining: u8,
    ) -> std::result::Result<(), String> {
        let doc = self
            .documents
            .iter()
            .find(|doc| doc.state.unit_id == fence.unit)
            .ok_or("协作窗口已关闭")?;
        let mut canvas = doc.state.canvas.borrow_mut();
        let mut slot = doc.state.projection_edit.borrow_mut();
        let owner = slot
            .as_ref()
            .ok_or("协作会话已关闭")?
            .apply_ready(&ready, &mut canvas.editor)
            .map_err(|error| format!("协作响应未发布；原图及请求保留：{error}"))?;
        let snapshot = owner.capture(&canvas.editor);
        fence.stamp = owner.stamp();
        *slot = Some(owner);
        drop(slot);
        drop(canvas);
        self.projection_edit.0 = Phase::Checkpoint(Box::new(Saved {
            fence,
            ready,
            snapshot,
            durable: false,
            remaining,
        }));
        self.saved_state = None;
        self.display_cache.borrow_mut().take();
        *self.display_composition.borrow_mut() = Default::default();
        let _ = windows::Win32::Graphics::Gdi::InvalidateRect(Some(doc.window.0), None, false);
        self.edit_status("协作响应已接收，等待工作区检查点…");
        Ok(())
    }
    unsafe fn continue_projection_edit(
        &mut self,
        pending: Box<Pending>,
        config: &mut impl FnMut(
            &str,
        ) -> std::result::Result<
            Config,
            crate::device_authorization_config::Error,
        >,
    ) {
        self.projection_edit.1.confirmed(&pending.fence.unit);
        let next = self
            .documents
            .iter()
            .find(|doc| doc.state.unit_id == pending.fence.unit)
            .and_then(|doc| {
                let slot = doc.state.projection_edit.borrow();
                let owner = slot.as_ref()?;
                let snapshot = owner.capture(&doc.state.canvas.borrow().editor);
                let current = snapshot.editor_view().ok()?;
                let observed = crate::projection_edit::view::View::from_document(
                    snapshot.document(),
                    current.size(),
                )
                .ok()?;
                if snapshot.conflict().is_none()
                    && (snapshot.role() == crate::projection_edit::workspace::Role::Source
                        || snapshot.document().mode() == crate::projection_edit::Mode::TwoWay)
                    && !observed.changes_to(&current).ok()?.is_empty()
                {
                    Some(Selection {
                        unit: pending.fence.unit.clone(),
                        stamp: owner.stamp(),
                    })
                } else {
                    None
                }
            });
        if let Some(selection) = next {
            if pending.remaining > 1 {
                // 完成后先释放原Slot/Lease/句柄，再准入下一批；每次显式操作最多8批，持续编辑不会无限循环。
                let remaining = pending.remaining - 1;
                drop(pending);
                self.start_projection_edit(selection, config, remaining, Action::Submit);
                return;
            }
            self.edit_status("本批协作修改已保存；仍有修改待继续提交");
        } else {
            let conflict = self
                .documents
                .iter()
                .find(|doc| doc.state.unit_id == pending.fence.unit)
                .and_then(|doc| {
                    doc.state
                        .projection_edit
                        .borrow()
                        .as_ref()
                        .and_then(|owner| {
                            owner.capture(&doc.state.canvas.borrow().editor).conflict()
                        })
                });
            self.edit_status(if conflict.is_some() {
                "协作冲突已保存；请选择保留本地修改或采用远端标注"
            } else {
                "协作会话已更新并写入检查点"
            });
        }
    }
}
