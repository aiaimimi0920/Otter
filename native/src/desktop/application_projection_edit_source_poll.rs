//! 初始化响应先原子发布标注/canonical候选，真正默认Checkpoint后ack；取消/迟到保原stage与pending。
use super::*;
impl Application {
    pub(in crate::desktop::application::projection_edit) unsafe fn poll_projection_source(
        &mut self,
        phase: Box<Phase>,
        config: &mut impl FnMut(
            &str,
        ) -> std::result::Result<
            Config,
            crate::device_authorization_config::Error,
        >,
    ) {
        match *phase {
            Phase::Sending(mut pending) => {
                let valid = !pending.cancelled
                    && pending.guard.matches(self, None)
                    && pending.source.as_ref().is_some_and(|(scope, base)| {
                        scope.matches_basis(self)
                            && self
                                .documents
                                .iter()
                                .find(|doc| doc.state.unit_id == pending.guard.unit)
                                .is_some_and(|doc| base.matches(&doc.state.canvas.borrow().editor))
                    });
                if !valid {
                    pending.cancelled = true;
                    pending.job.request_stop();
                }
                let Some(result) = pending.job.try_join() else {
                    self.projection_edit.0 =
                        super::super::Phase::Source(Box::new(Phase::Sending(pending)));
                    return;
                };
                let unit = pending.guard.unit.clone();
                let scheduled = pending.guard.scheduled.clone();
                let result = match result {
                    Ok(Output::Edited(ready)) if valid => {
                        self.publish_projection_source(pending, ready, config)
                    }
                    Err(error) => Err(format!(
                        "源端协作未确认，原stage/请求和本地编辑保留：{error:?}"
                    )),
                    _ => Err("源端协作响应已过期；保留当前内容和原请求".into()),
                };
                if let Err(error) = result {
                    self.settle_projection_source(&unit, scheduled.as_deref(), false);
                    self.edit_status(&error);
                }
            }
            Phase::Checkpoint(saved) => {
                let candidate = (!saved.durable)
                    .then(|| saved.ready.source.as_ref()?.candidate.as_ref())
                    .flatten();
                if !saved.guard.matches(self, candidate) {
                    self.settle_projection_source(
                        &saved.guard.unit,
                        saved.guard.scheduled.as_deref(),
                        false,
                    );
                    self.edit_status("源端协作保存已中止；内容和原stage/请求保留");
                    return;
                }
                if !saved.durable {
                    self.projection_edit.0 =
                        super::super::Phase::Source(Box::new(Phase::Checkpoint(saved)));
                    return;
                }
                let result = (|| {
                    let origin = saved.guard.context.origin();
                    let prepared = config(origin).map_err(|_| stale())?;
                    let lease = self
                        .authorization_owner
                        .lease(origin, now()?)
                        .map_err(|_| stale())?;
                    if !lease.same_grant(&saved.guard.current)
                        || saved
                            .ready
                            .source
                            .as_ref()
                            .and_then(|source| source.candidate.as_ref())
                            .is_some_and(|candidate| {
                                !self.projection_sender_inclusion(candidate.binding())
                            })
                    {
                        return Err(stale());
                    }
                    Job::start_edit_ack(prepared, lease, saved.ready).map_err(|_| stale())
                })();
                match result {
                    Ok(job) => {
                        self.projection_edit.0 =
                            super::super::Phase::Source(Box::new(Phase::Acknowledging(Pending {
                                guard: saved.guard,
                                job,
                                source: None,
                                cancelled: false,
                            })));
                        self.edit_status("源端协作已写入检查点，正在确认原请求…");
                    }
                    Err(error) => {
                        self.settle_projection_source(
                            &saved.guard.unit,
                            saved.guard.scheduled.as_deref(),
                            false,
                        );
                        self.edit_status(&format!(
                            "源端协作内容已保存；原请求确认待显式重试：{error}"
                        ));
                    }
                }
            }
            Phase::Acknowledging(mut pending) => {
                let valid = !pending.cancelled && pending.guard.matches(self, None);
                if !valid {
                    pending.cancelled = true;
                    pending.job.request_stop();
                }
                let Some(result) = pending.job.try_join() else {
                    self.projection_edit.0 =
                        super::super::Phase::Source(Box::new(Phase::Acknowledging(pending)));
                    return;
                };
                if valid && matches!(result, Ok(Output::EditAcknowledged)) {
                    let guard = pending.guard;
                    drop(pending.job);
                    self.continue_projection_source(guard, config);
                } else {
                    self.settle_projection_source(
                        &pending.guard.unit,
                        pending.guard.scheduled.as_deref(),
                        false,
                    );
                    let reason = match result {
                        Err(error) => format!("{error:?}"),
                        _ => "响应过期、已取消或类型不匹配".into(),
                    };
                    self.edit_status(&format!("源端协作原请求确认未完成；请显式重试：{reason}"));
                }
            }
        }
    }
    unsafe fn publish_projection_source(
        &mut self,
        mut pending: Pending,
        ready: Box<Ready>,
        config: &mut impl FnMut(
            &str,
        ) -> std::result::Result<
            Config,
            crate::device_authorization_config::Error,
        >,
    ) -> std::result::Result<(), String> {
        let source = ready.source.as_ref().ok_or("缺少源端协作结果")?;
        let targets =
            self.projection_source_targets(&pending.guard.unit, pending.guard.context.origin());
        if !targets.iter().any(|target| target.binding == source.target) {
            return Err("源端目标已变化".into());
        }
        if let Some(candidate) = &source.candidate {
            if !candidate
                .expected()
                .same_storage(&pending.source.as_ref().unwrap().0.binding_source)
                || !candidate.index().is_for(candidate.source())
                || self.restored_session.is_none()
            {
                return Err("源端canonical候选已变化".into());
            }
        }
        let route = targets
            .iter()
            .find(|target| target.binding.envelope().projection_id() == source.frozen.anchor())
            .map(|target| target.binding.edit_route());
        let doc = self
            .documents
            .iter()
            .find(|doc| doc.state.unit_id == pending.guard.unit)
            .ok_or("源端窗口已关闭")?;
        let mut canvas = doc.state.canvas.borrow_mut();
        let mut slot = doc.state.projection_edit.borrow_mut();
        let unchanged = ready.ticket.is_none()
            && source.candidate.is_none()
            && slot
                .as_ref()
                .is_some_and(|owner| owner.capture(&canvas.editor).document() == &ready.document);
        pending
            .guard
            .targets
            .retain(|id| id != source.target.envelope().projection_id());
        pending.guard.target = Some(source.target.clone());
        if unchanged {
            drop(slot);
            drop(canvas);
            drop(ready);
            drop(pending.job);
            drop(pending.source);
            self.continue_projection_source(pending.guard, config);
            return Ok(());
        }
        let route = route
            .or_else(|| {
                slot.as_ref()
                    .map(|owner| owner.capture(&canvas.editor).route())
            })
            .ok_or("源端anchor不可用")?;
        let owner = crate::desktop::projection_edit_workspace::source::prepare(
            slot.as_ref(),
            &source.frozen,
            source.layout,
            pending.guard.context.clone(),
            route,
            ready.document.clone(),
            &canvas.editor,
        )
        .and_then(|candidate| candidate.apply(&mut canvas.editor, None))
        .map_err(|error| format!("源端协作候选未发布：{error}"))?;
        let snapshot = owner.capture(&canvas.editor);
        pending.guard.stamp = Some(owner.stamp());
        *slot = Some(owner);
        drop(slot);
        drop(canvas);
        if let Some(candidate) = &source.candidate {
            self.restored_session
                .as_mut()
                .unwrap()
                .edit_original(candidate.source().clone());
        }
        self.saved_state = None;
        self.display_cache.borrow_mut().take();
        *self.display_composition.borrow_mut() = Default::default();
        let _ = windows::Win32::Graphics::Gdi::InvalidateRect(Some(doc.window.0), None, false);
        self.projection_edit.0 = super::super::Phase::Source(Box::new(Phase::Checkpoint(Saved {
            guard: pending.guard,
            ready,
            snapshot,
            durable: false,
        })));
        self.edit_status("源端协作已初始化，等待工作区检查点…");
        Ok(())
    }
    unsafe fn continue_projection_source(
        &mut self,
        guard: Guard,
        config: &mut impl FnMut(
            &str,
        ) -> std::result::Result<
            Config,
            crate::device_authorization_config::Error,
        >,
    ) {
        self.settle_projection_source(
            &guard.unit,
            guard
                .target
                .as_ref()
                .map(|target| target.envelope().projection_id())
                .or(guard.scheduled.as_deref()),
            true,
        );
        if !guard.targets.is_empty() && guard.explicit {
            self.start_projection_source(guard.unit, Some(guard.targets), true, config);
        } else {
            self.edit_status("源端协作已核对；新投送目标继续使用同一冻结底图");
            self.automatic_projection_edit(guard.unit, config);
        }
    }
}
