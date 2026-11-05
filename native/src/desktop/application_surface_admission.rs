//! 应用结算点接纳已认证挂载结果；失败保留证据，窗口保持隐藏等待生命周期激活。
use super::*;
#[path = "application_surface_input.rs"]
mod input;
#[path = "application_surface_outputs.rs"]
mod outputs;
impl Application {
    pub(super) fn activate_surface_windows(&mut self, visible: bool) -> Result<()> {
        self.activate_surface_windows_with(
            visible,
            &mut crate::device_authorization_config::Config::from_env,
        )
    }
    pub(super) fn activate_surface_windows_with(
        &mut self,
        visible: bool,
        config: &mut impl FnMut(
            &str,
        ) -> std::result::Result<
            crate::device_authorization_config::Config,
            crate::device_authorization_config::Error,
        >,
    ) -> Result<()> {
        use crate::surface_live_binding::Lifecycle;
        let now = crate::desktop::surface_windows::now_ms()?;
        // 普通结算和退出泵共用此入口；关闭窗口仍须返还 owner 并保存动作证据。
        self.remote_sessions
            .settle_surface_events(|outcome| self.surface_windows.accept_event(outcome, now))
            .map_err(|error| Error::new(E_FAIL, error.to_string()))?;
        self.remote_sessions
            .settle_surface_batches(|outcome| self.surface_windows.accept_batch(outcome, now))
            .map_err(|error| Error::new(E_FAIL, error.to_string()))?;
        self.remote_sessions
            .settle_surface_presentations(|binding, revision| {
                self.surface_windows
                    .iter_mut()
                    .find(|window| window.binding() == binding)
                    .ok_or(crate::desktop::surface_authorized_window::PresentationError::Inactive)?
                    .finish_javascript_presentation(binding, revision, now)
            });
        if visible {
            self.apply_surface_outputs(now);
        }
        {
            // refresh 可能关闭 HWND 并同步重入，不能跨窗口调用持有 View 借用。
            let origin = self.authorization.borrow().origin.clone();
            let selected = origin
                .as_deref()
                .and_then(|origin| {
                    self.remote_sessions
                        .pending_surface_confirmation_for(origin, now, |receipt| {
                            self.surface_windows.iter_mut().any(|window| {
                                window.binding() == receipt.event.binding()
                                    && window.confirmation_for(receipt, now).is_ok()
                            })
                        })
                        .or_else(|| {
                            self.remote_sessions.pending_batch_confirmation_for(
                                origin,
                                now,
                                |receipt, index| {
                                    self.surface_windows.iter_mut().any(|window| {
                                        window.binding() == receipt.events[index].binding()
                                            && window
                                                .batch_confirmation_for(receipt, index, now)
                                                .is_ok()
                                    })
                                },
                            )
                        })
                })
                .map(str::to_owned);
            let continuation = origin
                .as_deref()
                .and_then(|origin| {
                    self.remote_sessions
                        .pending_batch_resume_for(origin, |receipt| {
                            self.surface_windows.iter_mut().any(|window| {
                                receipt
                                    .events
                                    .first()
                                    .is_some_and(|event| window.binding() == event.binding())
                                    && window.batch_continuation_for(receipt, now).is_ok()
                            })
                        })
                })
                .map(str::to_owned);
            let mut view = self.authorization.borrow_mut();
            view.continuation = if view.origin == origin && !view.closed {
                continuation
            } else {
                None
            };
            view.confirmation = if view.origin == origin && !view.closed {
                selected
            } else {
                None
            };
        }
        while self.remote_sessions.deliver_surface(|outcome| {
            self.surface_windows
                .accept_activation(outcome, now, visible)
        }) {}
        for window in self.surface_windows.iter_mut() {
            // refresh 可因 JS 终态关闭 HWND；必须先结算再选择 active/inactive，避免丢失最后唤醒。
            window.refresh(now)?;
            if visible
                && window.is_alive()
                && !window.is_shown()
                && window.activation_error().is_none()
                && window
                    .lifecycle()
                    .is_some_and(|owner| owner.confirmed_active())
            {
                if let Err(error) = window.show(now) {
                    window.record_activation_error(error.to_string());
                }
            }
            let transition = if window.is_alive() {
                window
                    .begin_activation(now)?
                    .map(|owner| (owner, Lifecycle::Active))
            } else {
                window
                    .begin_deactivation(now)?
                    .map(|owner| (owner, Lifecycle::Inactive))
            };
            let Some((owner, state)) = transition else {
                continue;
            };
            let result = start_transition_with(
                &mut self.authorization_owner,
                &mut self.remote_sessions,
                owner,
                state,
                now,
                &mut *config,
            );
            if let Err(error) = result {
                window.failed_start(error);
            }
        }
        self.dispatch_surface_clicks_with(visible, config);
        Ok(())
    }
    pub(super) fn admit_surface_results(&mut self) -> Result<()> {
        let now = crate::desktop::surface_windows::now_ms()?;
        let view = self.authorization.borrow();
        let selected = view.origin.clone();
        let closed = view.closed;
        drop(view);
        loop {
            let handles = self.surface_windows.wait_budget();
            let delivered = self.remote_sessions.deliver_attach(|outcome| {
                self.surface_windows.admit(
                    outcome,
                    |outcome| {
                        // JS 除窗口撤权外还有回复与线程终态；关闭后两者仍须保留收割。
                        let extra = if outcome
                            .request
                            .supports(crate::surface_host_capabilities::Runtime::Javascript)
                        {
                            3
                        } else {
                            1
                        };
                        if !super::live::fits_wait_budget(self.live_sessions.len(), handles + extra)
                        {
                            return Err("窗口等待容量已满，保留已确认挂载结果".into());
                        }
                        if closed || selected.as_deref() != Some(outcome.origin.as_str()) {
                            return Err("服务已切换或授权界面已关闭，保留挂载结果".into());
                        }
                        self.authorization_owner
                            .lease(&outcome.origin, now)
                            .map_err(|error| format!("当前授权不可用：{error:?}"))
                    },
                    now,
                )
            });
            if !delivered {
                break;
            }
        }
        Ok(())
    }
}

/// 显示窗口与未建窗附件共用请求边界；新租约必须属于原附件设备。
pub(super) fn start_transition(
    authorization: &mut crate::device_authorization_owner::Owner,
    remote: &mut crate::desktop::composition::relay::sessions::Sessions,
    owner: crate::surface_lifecycle_owner::Owner,
    state: crate::surface_live_binding::Lifecycle,
    now: u64,
) -> std::result::Result<(), crate::surface_lifecycle_runtime::StartError> {
    start_transition_with(
        authorization,
        remote,
        owner,
        state,
        now,
        crate::device_authorization_config::Config::from_env,
    )
}
fn start_transition_with(
    authorization: &mut crate::device_authorization_owner::Owner,
    remote: &mut crate::desktop::composition::relay::sessions::Sessions,
    owner: crate::surface_lifecycle_owner::Owner,
    state: crate::surface_live_binding::Lifecycle,
    now: u64,
    config: impl FnOnce(
        &str,
    ) -> std::result::Result<
        crate::device_authorization_config::Config,
        crate::device_authorization_config::Error,
    >,
) -> std::result::Result<(), crate::surface_lifecycle_runtime::StartError> {
    use crate::surface_lifecycle_runtime::{Action, StartError};
    let origin = owner.binding().origin();
    let prepared = authorization
        .lease(origin, now)
        .map_err(|error| format!("当前授权不可用：{error:?}"))
        .and_then(|lease| {
            if lease.device_id() != owner.binding().device() {
                return Err("当前设备与附件绑定不一致".into());
            }
            config(origin)
                .map(|config| (config, lease))
                .map_err(|error| format!("配置无效：{error:?}"))
        });
    match prepared {
        Ok((config, lease)) => {
            remote.start_surface(config, lease, owner, Action::Transition(state))
        }
        Err(error) => Err(StartError {
            owner: Box::new(owner),
            error: std::io::Error::other(error),
        }),
    }
}
