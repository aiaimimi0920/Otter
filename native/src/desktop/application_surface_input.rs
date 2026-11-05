//! 按授权/owner/未结算证据同步输入资格，再消费 HWND 候选；消息回调不联网。
use super::*;
impl Application {
    pub(in crate::desktop::application) fn surface_input_origin(
        &self,
        visible: bool,
    ) -> Option<String> {
        if !visible || self.modals.blocked() || self.exit_requested.get() {
            return None;
        }
        let view = self.authorization.borrow();
        if view.closed || view.busy || view.request.is_some() {
            None
        } else {
            view.origin.clone()
        }
    }
    pub(in crate::desktop::application) fn dispatch_surface_clicks_with(
        &mut self,
        visible: bool,
        config: &mut impl FnMut(
            &str,
        ) -> std::result::Result<
            crate::device_authorization_config::Config,
            crate::device_authorization_config::Error,
        >,
    ) {
        let origin = self.surface_input_origin(visible);
        let now = crate::desktop::surface_windows::now_ms().unwrap_or(u64::MAX);
        for window in self.surface_windows.iter_mut() {
            let editing = origin.as_deref() == Some(window.binding().origin());
            let requested = editing
                && self
                    .remote_sessions
                    .surface_input_ready_with(window.binding(), |commit| {
                        self.documents
                            .iter()
                            .find(|document| {
                                document.state.unit_id.as_str() == window.binding().node()
                            })
                            .is_some_and(|document| {
                                document
                                    .state
                                    .execution_results
                                    .borrow()
                                    .has_surface_result(window.binding(), commit.revision())
                            })
                    });
            if let Err(error) = window.sync_input(editing, requested, now) {
                self.authorization.borrow_mut().status = format!("输入状态更新失败：{error}");
            }
        }
        let Some(origin) = origin else { return };
        let mut status = None;
        for window in self.surface_windows.iter_mut() {
            if !window.has_pending_click()
                || window.binding().origin() != origin
                || !window.is_shown()
                || !self
                    .remote_sessions
                    .surface_input_ready_with(window.binding(), |commit| {
                        self.documents
                            .iter()
                            .find(|document| {
                                document.state.unit_id.as_str() == window.binding().node()
                            })
                            .is_some_and(|document| {
                                document
                                    .state
                                    .execution_results
                                    .borrow()
                                    .has_surface_result(window.binding(), commit.revision())
                            })
                    })
                || !window
                    .lifecycle()
                    .is_some_and(crate::surface_lifecycle_owner::Owner::confirmed_active)
            {
                continue;
            }
            let prepared = (|| {
                let now =
                    crate::desktop::surface_windows::now_ms().map_err(std::io::Error::other)?;
                let lease = self
                    .authorization_owner
                    .lease(&origin, now)
                    .map_err(|error| std::io::Error::other(format!("{error:?}")))?;
                if lease.device_id() != window.binding().device() {
                    return Err(std::io::Error::other("Surface device changed"));
                }
                let config =
                    config(&origin).map_err(|error| std::io::Error::other(format!("{error:?}")))?;
                Ok((config, lease, now))
            })();
            let (config, lease, now) = match prepared {
                Ok(value) => value,
                Err(error) => {
                    status = Some(format!("输入未准备：{error}"));
                    continue;
                }
            };
            let result = self
                .remote_sessions
                .dispatch_surface_batch(config, lease, |limit| {
                    if !window.prepared_input_fits(limit) {
                        return Err(std::io::ErrorKind::WouldBlock.into());
                    }
                    let Some(events) = window
                        .take_prepared_click(now)
                        .map_err(std::io::Error::other)?
                    else {
                        return Ok(None);
                    };
                    if events.is_empty() {
                        return Ok(None);
                    }
                    if events.len() > limit {
                        window.cancel_prepared_input(events);
                        return Err(std::io::ErrorKind::WouldBlock.into());
                    }
                    let view = self.authorization.borrow();
                    let changed = self.exit_requested.get()
                        || view.closed
                        || view.busy
                        || view.request.is_some()
                        || view.origin.as_deref() != Some(origin.as_str());
                    drop(view);
                    if changed {
                        window.cancel_prepared_input(events);
                        return Ok(None);
                    }
                    let Some(owner) = window
                        .begin_batch(&events, now)
                        .map_err(std::io::Error::other)?
                    else {
                        return Ok(None);
                    };
                    Ok(Some(crate::surface_event_batch_runtime::Input {
                        owner,
                        revision: window.snapshot_revision(),
                        events,
                    }))
                });
            match result {
                Ok(true) => status = Some("正在提交 Surface 输入…".into()),
                Ok(false) => {}
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(error) => status = Some(format!("输入未启动：{error}")),
            }
        }
        if let Some(status) = status {
            self.authorization.borrow_mut().status = status;
        }
    }
}
