//! Admit authenticated mount results at application settlement points; retain evidence on failure and keep windows hidden until lifecycle activation.
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
        .map(|_| ())
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
    ) -> Result<bool> {
        use crate::surface_live_binding::Lifecycle;
        let now = crate::desktop::surface_windows::now_ms()?;
        // Ordinary settlement and exit pumps share this entry point; closed windows still return owners and preserve action evidence.
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
            // refresh may close HWNDs and synchronously reenter; do not hold View borrows across window calls.
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
        // Release confirmed slots before delivering queued mounts; stored outcomes have no wake handle.
        let retirement_blocked = self.surface_windows.retire_confirmed();
        self.admit_surface_results()?;
        for window in self.surface_windows.iter_mut() {
            // refresh may close HWNDs on JS terminal state; settle before choosing active/inactive to avoid losing the last wakeup.
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
        // Do not collect one-shot owners at capacity; retry after in-flight tasks settle, using the same entry point on exit.
        for failure in self.surface_windows.failures_mut() {
            if !self.remote_sessions.has_surface_worker_slot() {
                break;
            }
            let Some(owner) = failure.begin_inactive() else {
                continue;
            };
            if let Err(error) = start_transition_with(
                &mut self.authorization_owner,
                &mut self.remote_sessions,
                owner,
                Lifecycle::Inactive,
                now,
                &mut *config,
            ) {
                failure.failed_start(error);
            }
        }
        Ok(retirement_blocked || self.surface_windows.retire_confirmed())
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
                        // Besides window revocation, JS has replies and thread terminal state; retain both for collection after closure.
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

/// Visible windows and attachments without windows share request boundaries; new leases must belong to the original attachment's device.
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
