//! 授权 owner 仅由应用循环访问；菜单共享状态只提供有界请求和无凭据的结果。
use super::*;
use crate::{
    desktop::device_authorization_ui::Request,
    device_authorization_config::{Action, Config},
    device_authorization_owner::Event,
    device_authorization_runtime::Task,
};
impl Application {
    pub(super) fn process_authorization(&mut self) {
        self.process_authorization_with_projection(Config::from_env);
    }
    pub(super) fn process_authorization_with_projection(
        &mut self,
        mut config: impl FnMut(
            &str,
        )
            -> std::result::Result<Config, crate::device_authorization_config::Error>,
    ) {
        self.projection_profile.poll();
        self.live_viewer_requests(&mut config);
        unsafe {
            self.projection_edit_requests(&mut config);
        }
        self.process_projection_policy();
        self.projection_profile.sync(
            &mut self.authorization.borrow_mut().projection.recovery,
            |ticket| self.projection_checkpoint_inclusion(ticket).is_some(),
        );
        let resume = {
            let mut view = self.authorization.borrow_mut();
            if matches!(view.request, Some(Request::ResumeSurface(_))) {
                match view.request.take() {
                    Some(Request::ResumeSurface(id)) => Some(id),
                    _ => unreachable!(),
                }
            } else {
                None
            }
        };
        if let Some(id) = resume {
            self.resume_surface_batch(&id);
        }
        // 对话框不能跨消息循环持有 View 的 RefCell 借用。
        let confirmation = {
            let mut view = self.authorization.borrow_mut();
            if matches!(view.request, Some(Request::ConfirmSurface(_))) {
                match view.request.take() {
                    Some(Request::ConfirmSurface(id)) => Some(id),
                    _ => unreachable!(),
                }
            } else {
                None
            }
        };
        if let Some(id) = confirmation {
            if !self.confirm_surface_batch(&id) {
                self.confirm_surface_event(&id);
            }
        }
        let display = {
            let mut view = self.authorization.borrow_mut();
            if matches!(
                view.request,
                Some(Request::Projection(
                    crate::desktop::device_authorization_ui::projection::Request::Display(_)
                ))
            ) {
                match view.request.take() {
                    Some(Request::Projection(
                        crate::desktop::device_authorization_ui::projection::Request::Display(
                            ticket,
                        ),
                    )) => Some(ticket),
                    _ => unreachable!(),
                }
            } else {
                None
            }
        };
        if let Some(ticket) = display {
            unsafe {
                self.start_projection_display_recovery(ticket, &mut config);
            }
        }
        // Scope可为旧工作区分配一次持久身份；不能跨这一步保留View的RefCell借用。
        let receive_kind = {
            let view = self.authorization.borrow();
            match &view.request {
                Some(Request::Projection(
                    crate::desktop::device_authorization_ui::projection::Request::Reattach(entry),
                )) if !self
                    .workspace
                    .units()
                    .iter()
                    .any(|unit| unit.as_str() == entry.receiver_unit_id()) =>
                {
                    Some(true)
                }
                Some(Request::Projection(
                    crate::desktop::device_authorization_ui::projection::Request::Reattach(_),
                )) => None,
                Some(Request::Projection(_)) => Some(false),
                _ => None,
            }
        };
        let receive_scope = unsafe {
            match receive_kind {
                Some(true) => super::projection::receive::Scope::capture_existing(self).ok(),
                Some(false) => super::projection::receive::Scope::capture(self).ok(),
                None => None,
            }
        };
        let sender_request = {
            let mut view = self.authorization.borrow_mut();
            if matches!(
                view.request,
                Some(
                    Request::SendProjection(_)
                        | Request::SendProjectionGroup(_)
                        | Request::StopProjectionGroup(_)
                        | Request::CancelProjectionSend
                        | Request::StopProjectionSend(_)
                        | Request::RetryProjectionSend(_)
                )
            ) {
                view.request.take()
            } else {
                None
            }
        };
        match sender_request {
            Some(Request::SendProjection(selection)) => unsafe {
                self.start_projection_sender(selection, &mut config);
            },
            Some(Request::SendProjectionGroup(selection)) => unsafe {
                self.start_projection_group(selection, &mut config);
            },
            Some(Request::StopProjectionGroup(selection)) => unsafe {
                self.start_projection_stop_group(selection);
            },
            Some(Request::CancelProjectionSend) => self.stop_projection_sender(),
            Some(Request::StopProjectionSend(selected)) => unsafe {
                self.start_projection_sender_stop(*selected)
            },
            Some(Request::RetryProjectionSend(unit)) => unsafe {
                self.retry_projection_sender(unit, &mut config);
            },
            _ => {}
        }
        let invalidate_sender = {
            let view = self.authorization.borrow();
            matches!(view.request, Some(Request::Cancel | Request::Start(_)))
                || matches!(&view.request, Some(Request::Select(origin)) if view.origin.as_ref() != Some(origin))
        };
        if invalidate_sender {
            self.stop_projection_sender();
        }
        // OpenArt 消费前释放 View 借用；本地链使用显式 HTTPS origin，不走旧远端 attach。
        let local_request = {
            let mut view = self.authorization.borrow_mut();
            if !view.closed && matches!(view.request, Some(Request::OpenArt { .. })) {
                match view.request.take() {
                    Some(Request::OpenArt { art, node }) => Some((view.origin.clone(), art, node)),
                    _ => unreachable!(),
                }
            } else {
                None
            }
        };
        if let Some((origin, art, node)) = local_request {
            let result = origin
                .ok_or_else(|| "尚未选择服务".to_owned())
                .and_then(|origin| {
                    let configuration =
                        config(&origin).map_err(|_| "Art 服务配置无效".to_owned())?;
                    self.open_local_art(&origin, art, &node, configuration)
                });
            let mut view = self.authorization.borrow_mut();
            view.mount_busy = self.remote_sessions.attaching() || self.local_arts.busy();
            view.status = match result {
                Ok(()) => "正在本地打开 Art…".into(),
                Err(error) => error,
            };
        }
        let retire_local = {
            let view = self.authorization.borrow();
            matches!(&view.request, Some(Request::Cancel | Request::Start(_)))
                || matches!(&view.request,Some(Request::Select(origin)) if view.origin.as_ref()!=Some(origin))
        };
        // 原本离线租约不属于Device会话；显式授权/清除/换服务必须在无view借用时停止本地HWND。
        if retire_local {
            self.local_arts.reset();
            self.remote_sessions.cancel_viewers();
        }
        let mut view = self.authorization.borrow_mut();
        // 先消费撤销/切换，再收割完成结果；旧请求即使已经完成也不能覆盖新选择。
        if let Some(request) = view.request.take().filter(|_| !view.closed) {
            match request {
                Request::WatchLive(_) | Request::StopLive(_) => {
                    unreachable!("viewer requests consumed before borrowing view")
                }
                Request::SendProjection(_)
                | Request::SendProjectionGroup(_)
                | Request::StopProjectionGroup(_)
                | Request::CancelProjectionSend
                | Request::StopProjectionSend(_)
                | Request::RetryProjectionSend(_) => {
                    unreachable!("sender requests consumed before borrowing view")
                }
                Request::SubmitProjectionEdit(_)
                | Request::ResolveProjectionEdit(_, _)
                | Request::SetProjectionEditMode(_, _)
                | Request::InitializeProjectionEdit(_)
                | Request::CancelProjectionEdit => {
                    unreachable!("edit requests consumed before borrowing view")
                }
                Request::ProjectionTargets => {
                    if self.modals.blocked()
                        || self.exit_requested.get()
                        || self.projection_receive.busy()
                        || self.projection_edit.busy()
                    {
                        view.targets.status = "目标目录未查询：应用正在处理其他操作".into();
                    } else {
                        super::projection::targets::start(
                            &mut view,
                            &mut self.authorization_owner,
                            &mut self.remote_sessions,
                            &mut config,
                        );
                    }
                }
                Request::Projection(request) => {
                    if self.modals.blocked()
                        || self.exit_requested.get()
                        || self.restore_job.is_some()
                        || self.projection_receive.busy()
                        || self.projection_edit.busy()
                    {
                        view.projection.status = "投送操作未执行：工作区正在切换或退出".into();
                    } else if matches!(
                        request,
                        crate::desktop::device_authorization_ui::projection::Request::Accept(_)
                        | crate::desktop::device_authorization_ui::projection::Request::Reattach(_)
                    ) && receive_scope.is_none()
                    {
                        view.projection.status = "接收未启动：当前工作区不能安全保存".into();
                    } else if super::projection::start(
                        &mut view,
                        request,
                        &mut self.authorization_owner,
                        &mut self.remote_sessions,
                        &self.projection_profile,
                        &mut config,
                    ) {
                        self.projection_receive.waiting(receive_scope);
                    }
                }
                Request::ConfirmSurface(_) | Request::ResumeSurface(_) => {
                    unreachable!("confirmation consumed before borrowing view")
                }
                Request::ReceivePolicy(_) | Request::ReceivePause(_) => {
                    unreachable!("policy consumed before borrowing view")
                }
                Request::RestoreSurface(candidate) => {
                    let result = super::surface_remount::start(
                        &mut view,
                        candidate,
                        &mut self.authorization_owner,
                        &mut self.remote_sessions,
                        &self.surface_windows,
                        self.live_sessions.len(),
                    );
                    view.mount_busy = self.remote_sessions.attaching();
                    view.restoring = result.is_ok();
                    view.recovery_status = match result {
                        Ok(()) => "正在重新挂载候选…".into(),
                        Err(error) => error,
                    };
                }
                Request::LoadRecovery(path) => {
                    view.recovery.clear();
                    view.recovery_candidate = None;
                    let result = self.remote_sessions.read_recovery_file(path);
                    view.recovery_busy = result.is_ok();
                    view.recovery_status = match result {
                        Ok(()) => "正在读取恢复记录…".into(),
                        Err(error) => format!("恢复文件未读取：{error}"),
                    };
                }
                Request::QueryRecovery(record) => {
                    view.recovery_candidate = None;
                    let result = super::surface_recovery::query(
                        &record,
                        view.origin.as_deref(),
                        &mut self.authorization_owner,
                        &mut self.remote_sessions,
                    );
                    view.recovery_busy = result.is_ok();
                    view.recovery_status = match result {
                        Ok(()) => "正在查询恢复候选…".into(),
                        Err(error) => error,
                    };
                }
                Request::OpenArt { .. } => {
                    unreachable!("local request consumed before borrowing view")
                }
                Request::Select(origin) => {
                    if view.origin.as_ref() != Some(&origin) {
                        super::projection::reset(
                            &mut self.remote_sessions,
                            &mut view,
                            &mut self.projection_receive,
                            &mut self.projection_cleanup,
                        );
                        view.recovery_candidate = None;
                        view.recovery_status.clear();
                        self.remote_sessions.cancel_discovery();
                        self.remote_sessions.cancel_surface_requests();
                        view.catalog = None;
                        view.catalog_busy = false;
                        if let Some(old) = &view.origin {
                            let _ = self.authorization_owner.invalidate(old);
                        }
                        view.origin = Some(origin);
                        view.busy = false;
                        view.status = "已选择服务，尚未获取授权会话".into();
                    }
                }
                Request::Cancel => {
                    super::projection::reset(
                        &mut self.remote_sessions,
                        &mut view,
                        &mut self.projection_receive,
                        &mut self.projection_cleanup,
                    );
                    view.recovery_candidate = None;
                    view.recovery_status.clear();
                    self.remote_sessions.cancel_discovery();
                    self.remote_sessions.cancel_surface_requests();
                    view.catalog = None;
                    view.catalog_busy = false;
                    if let Some(origin) = &view.origin {
                        let _ = self.authorization_owner.invalidate(origin);
                    }
                    view.busy = false;
                    view.status = "本地会话已清除；服务端设备批准状态不变".into();
                    // 无本地Art信任的旧会话不触及profile；配置捕获无IO，减权沿原有界后台owner提交。
                    if let Some(origin) = view
                        .origin
                        .as_deref()
                        .filter(|o| self.art_issuers.is_trusted(o))
                    {
                        let result = config(origin)
                            .map_err(|_| "离线授权清除未启动：配置无效".to_owned())
                            .and_then(|config| {
                                self.authorization_owner
                                    .start(Task::ClearOffline { config })
                                    .map_err(|e| format!("离线授权清除未启动：{e:?}"))
                            });
                        match result {
                            Ok(()) => {
                                view.busy = true;
                                view.status = "本地会话已停止；正在清除离线授权…".into();
                            }
                            Err(error) => view.status = error,
                        }
                    }
                }
                Request::Discover => {
                    view.catalog = None;
                    let result = view
                        .origin
                        .as_deref()
                        .ok_or_else(|| "尚未选择服务".to_owned())
                        .and_then(|origin| {
                            let now = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .map_err(|_| "系统时间无效".to_owned())?;
                            let now = u64::try_from(now.as_millis())
                                .map_err(|_| "系统时间超出范围".to_owned())?;
                            let lease = self
                                .authorization_owner
                                .lease(origin, now)
                                .map_err(|e| format!("请先获取有效授权：{e:?}"))?;
                            let config = config(origin).map_err(|e| format!("配置无效：{e:?}"))?;
                            self.remote_sessions
                                .discover(config, lease)
                                .map_err(|e| format!("目录任务未启动：{e}"))
                        });
                    view.catalog_busy = result.is_ok();
                    view.status = match result {
                        Ok(()) => "正在获取远程会话目录…".into(),
                        Err(error) => error,
                    };
                }
                Request::Start(action) => {
                    super::projection::reset(
                        &mut self.remote_sessions,
                        &mut view,
                        &mut self.projection_receive,
                        &mut self.projection_cleanup,
                    );
                    view.recovery_candidate = None;
                    view.recovery_status.clear();
                    self.remote_sessions.cancel_discovery();
                    self.remote_sessions.cancel_surface_requests();
                    view.catalog = None;
                    view.catalog_busy = false;
                    if let Some(origin) = &view.origin {
                        let result = Config::from_env(origin)
                            .map_err(|e| format!("配置无效：{e:?}"))
                            .and_then(|config| {
                                self.authorization_owner
                                    .start(Task::Configured { config, action })
                                    .map_err(|e| format!("任务未启动：{e:?}"))
                            });
                        match result {
                            Ok(()) => {
                                view.busy = true;
                                view.status = match action {
                                    Action::RequestPairing => "正在提交配对…",
                                    Action::AwaitApproval => "等待管理员批准（最长 90 秒）…",
                                    Action::Renew => "正在获取授权会话…",
                                }
                                .into();
                            }
                            Err(error) => {
                                view.busy = false;
                                view.status = error;
                            }
                        }
                    }
                }
            }
        }
        for event in self.authorization_owner.poll() {
            let (origin, status) = match event {
                Event::OfflineCleared { origin } => (
                    origin,
                    "本地会话及离线授权已清除；服务端设备批准状态不变".into(),
                ),
                Event::Pairing { origin, .. } => {
                    (origin, "配对已提交；请等待管理员批准或获取会话".into())
                }
                Event::Authorized { origin, .. } => {
                    (origin, "最近一次授权成功；使用时仍须校验有效期".into())
                }
                Event::Failed { origin, error } => (origin, format!("授权请求失败：{error:?}")),
            };
            if !view.closed && view.origin.as_ref() == Some(&origin) {
                view.busy = false;
                view.status = status;
            }
        }
        drop(view);
        self.poll_projection_inbox();
        unsafe {
            self.poll_projection_sender(&mut config);
        }
        self.poll_projection_receive(&mut config);
        unsafe {
            self.poll_projection_edit(&mut config);
        }
        self.process_projection_cleanup(&mut config);
        unsafe {
            self.process_projection_automatic(&mut config, std::time::Instant::now());
            self.process_projection_sync(&mut config, std::time::Instant::now());
        }
        self.projection_profile.sync(
            &mut self.authorization.borrow_mut().projection.recovery,
            |ticket| self.projection_checkpoint_inclusion(ticket).is_some(),
        );
    }
}

#[cfg(test)]
#[path = "application_authorization_tests.rs"]
mod tests;
