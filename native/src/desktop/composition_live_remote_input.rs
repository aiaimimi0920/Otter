//! 远程控制复用Session唯一输入worker；后台单槽请求须等真实执行/释放结果，保护生命期不变。
use super::*;
use crate::live_relay_source_runtime::input::{Action, Bridge, Error as InputError, Request};
use std::sync::Arc;
pub(super) struct Remote {
    pub bridge: Arc<Bridge>,
    pending: Option<Request>,
}
impl Session {
    #[cfg(test)]
    pub(in crate::desktop) fn test_cancel_remote_input(&self) {
        self.cancel_remote_input();
    }
    #[cfg(test)]
    pub(in crate::desktop) fn test_input_target(
        &mut self,
        target: crate::live_window_target::Target,
    ) {
        self.target = Some(target);
        self.window.as_ref().unwrap().allow_input().unwrap();
    }
    pub(in crate::desktop) fn reserve_remote_input(&mut self) -> Result<Arc<Bridge>, String> {
        if !self.is_alive()
            || self.target.is_none()
            || self.input.is_some()
            || self.remote_input.is_some()
            || self.window.as_ref().is_none_or(|w| w.group_input_blocked())
        {
            return Err("来源不支持远程控制，或已有控制正在执行/回收".into());
        }
        let bridge = Bridge::new();
        self.remote_input = Some(Remote {
            bridge: bridge.clone(),
            pending: None,
        });
        if let Some(window) = &self.window {
            window.block_local_input(true);
        }
        Ok(bridge)
    }
    pub(in crate::desktop) fn remote_input_allowed(&self) -> bool {
        self.target.is_some() && self.remote_input.is_none() && self.input.is_none()
    }
    #[cfg(test)]
    pub(in crate::desktop) fn remote_input_idle(&self) -> bool {
        self.remote_input.is_none() && self.input.is_none()
    }
    pub(super) fn cancel_remote_input(&self) {
        if let Some(remote) = &self.remote_input {
            remote.bridge.cancel();
        }
    }
    pub(super) fn poll_remote_input(&mut self) {
        let Some(mut remote) = self.remote_input.take() else {
            return;
        };
        if !self.is_alive() || self.window.as_ref().is_none_or(|w| w.group_input_blocked()) {
            remote.bridge.cancel();
        }
        if remote.bridge.stopping() {
            self.disable_input();
        }
        if let Some(request) = remote.pending.take() {
            let result = match request.action {
                Action::Release if self.input.is_none() => Some(
                    if self.input_cleanup_unknown
                        || self
                            .input_cleanup
                            .is_some_and(|r| r.failed > 0 || r.skipped > 0)
                    {
                        Err(InputError::Execution)
                    } else {
                        Ok(())
                    },
                ),
                Action::Release => None,
                _ if self.input_error.is_some()
                    || self.input.is_none()
                    || remote.bridge.stopping() =>
                {
                    Some(Err(InputError::Execution))
                }
                _ if self.input_enabled
                    && self.input.as_ref().is_some_and(|w| w.outstanding() == 0) =>
                {
                    Some(Ok(()))
                }
                _ => None,
            };
            if let Some(result) = result {
                if result.is_err() {
                    remote.bridge.cancel();
                }
                request.finish(result);
            } else {
                remote.pending = Some(request);
            }
        }
        if !remote.bridge.stopping() && remote.pending.is_none() {
            if let Some(request) = remote.bridge.take() {
                let result = match request.action {
                    Action::Enable => self.enable_input(),
                    Action::Input(event) => {
                        self.send_input(self.last_input_sequence.saturating_add(1), event)
                    }
                    Action::Release => {
                        self.disable_input();
                        Ok(())
                    }
                };
                if result.is_err() {
                    request.finish(Err(InputError::Execution));
                    remote.bridge.cancel();
                } else {
                    remote.pending = Some(request);
                }
            }
        }
        remote.bridge.bind_input(
            self.input
                .as_ref()
                .and_then(crate::live_input_worker::Worker::stop_signal),
        );
        if remote.bridge.stopping() && self.input.is_none() {
            if let Some(window) = &self.window {
                window.block_local_input(false);
            }
        } else {
            self.remote_input = Some(remote);
        }
    }
}
