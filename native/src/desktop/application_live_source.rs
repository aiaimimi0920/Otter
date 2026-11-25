//! Application fence for explicit Live publication; old workspaces/sources/authorization cannot accept late Ready, and failures preserve local capture.
use super::*;
#[path = "application_live_direct_source.rs"]
mod direct;
use crate::{
    desktop::composition::live::window::publication::{Phase, Request},
    device_authorization_config::Config,
    live_relay_source_runtime::{Cleanup, Exit, Status},
};
impl Application {
    pub(in crate::desktop::application) fn poll_live_sources_with(
        &mut self,
        config: &mut impl FnMut(
            &str,
        ) -> std::result::Result<
            Config,
            crate::device_authorization_config::Error,
        >,
    ) {
        let origin = self.authorization.borrow().origin.clone();
        self.remote_sessions
            .cancel_sources(|expected, unit, identity| {
                !self.exit_requested.get()
                    && origin.as_deref() == Some(expected)
                    && self.live_sessions.iter().any(|source| {
                        source.unit_id() == unit
                            && self.workspace.contains(unit)
                            && source.belongs_to(&self.workspace)
                            && source.is_alive()
                            && std::sync::Arc::ptr_eq(source.input_identity_ref(), identity)
                    })
            });
        for event in self.remote_sessions.poll_sources() {
            let phase = if event.outcome.cleanup == Cleanup::Unconfirmed {
                Phase::Unconfirmed
            } else if matches!(
                event.outcome.exit,
                Exit::Stopped | Exit::SourceClosed | Exit::RemoteClosed
            ) {
                Phase::Idle
            } else {
                Phase::Failed
            };
            if let Some(source) = self.live_sessions.iter().find(|source| {
                source.unit_id() == &event.unit
                    && std::sync::Arc::ptr_eq(source.input_identity_ref(), &event.identity)
            }) {
                source.publication_phase(phase);
            }
            self.authorization.borrow_mut().live.status = format!(
                "Live 发布已结束：{:?} / {:?}",
                event.outcome.exit, event.outcome.cleanup
            );
        }
        let live_count = self.live_sessions.len();
        for index in 0..live_count {
            let source = &mut self.live_sessions[index];
            let status = self
                .remote_sessions
                .source_status(source.input_identity_ref());
            if let Some(status) = status {
                source.publication_phase(match status {
                    Status::Stopping => Phase::Stopping,
                    Status::Recovering => Phase::Recovering,
                    Status::Publishing => Phase::Publishing,
                    Status::Starting => Phase::Starting,
                });
            }
            let Some(publish) = source.take_publication_request() else {
                continue;
            };
            if matches!(publish, Request::Stop) {
                if self
                    .remote_sessions
                    .stop_source(source.input_identity_ref())
                {
                    source.publication_phase(Phase::Stopping);
                }
                continue;
            }
            let result = (|| -> std::result::Result<bool, String> {
                if self.exit_requested.get()
                    || self.modals.blocked()
                    || self.restore_job.is_some()
                    || !source.is_alive()
                    || !source.belongs_to(&self.workspace)
                {
                    return Err("应用或 Live 源正在切换，未发布".into());
                }
                if status.is_some() {
                    return Err("此 Live 已有发布任务".into());
                }
                if !super::live::fits_wait_budget(live_count, self.surface_windows.wait_budget()) {
                    return Err("窗口等待容量已满".into());
                }
                let origin = origin.as_deref().ok_or("请先连接并授权 Loom")?;
                let now = crate::desktop::surface_windows::now_ms().map_err(|e| e.to_string())?;
                let lease = self
                    .authorization_owner
                    .lease(origin, now)
                    .map_err(|e| format!("请先获取有效授权：{e:?}"))?;
                let config = config(origin).map_err(|e| format!("配置无效：{e:?}"))?;
                let recipient = if matches!(publish, Request::PublishDirect) {
                    let directory = self
                        .authorization
                        .borrow()
                        .targets
                        .directory
                        .clone()
                        .filter(|directory| directory.origin() == origin)
                        .ok_or("请先刷新投送目标")?;
                    let hwnd = source.publication_window().ok_or("Live 源已关闭")?;
                    let selected = {
                        let flag = std::rc::Rc::new(std::cell::Cell::new(false));
                        self.modals.register(&flag);
                        let _guard = crate::desktop::file_dialog::Guard::begin(&flag)
                            .expect("new modal flag");
                        direct::choose(hwnd, &lease, &directory)?
                    };
                    let Some(recipient) = selected else {
                        return Ok(false);
                    };
                    let view = self.authorization.borrow();
                    if view.closed
                        || view.busy
                        || view.targets.busy
                        || view.request.is_some()
                        || view.origin.as_deref() != Some(origin)
                        || view
                            .targets
                            .directory
                            .as_ref()
                            .is_none_or(|current| !std::sync::Arc::ptr_eq(current, &directory))
                        || self.exit_requested.get()
                        || self.modals.blocked()
                        || lease.is_revoked()
                        || source.publication_window() != Some(hwnd)
                    {
                        return Err("Live 来源、目标目录或当前设备授权已变化".into());
                    }
                    Some(recipient)
                } else {
                    None
                };
                let (mut publication, consumer) = source.source_publication()?;
                if let Some(recipient) = recipient {
                    publication
                        .allow_direct_viewer(recipient)
                        .map_err(|e| format!("投射目标无效：{e:?}"))?;
                }
                let observation = if matches!(publish, Request::PublishObservations) {
                    let target = source.observation_target()?;
                    publication
                        .allow_observation()
                        .map_err(|e| format!("观察发布无效：{e:?}"))?;
                    let observation_lease = self
                        .authorization_owner
                        .lease(origin, now)
                        .map_err(|e| format!("观察授权不可用：{e:?}"))?;
                    Some(crate::live_relay_source_runtime::Observation::new(
                        observation_lease,
                        target,
                    ))
                } else {
                    None
                };
                let interaction = if matches!(publish, Request::PublishInteractive) {
                    if !source.remote_input_allowed() {
                        return Err("来源不支持远程控制，或原控制尚未回收".into());
                    }
                    publication
                        .allow_interaction()
                        .map_err(|e| format!("控制发布无效：{e:?}"))?;
                    let control_lease = self
                        .authorization_owner
                        .lease(origin, now)
                        .map_err(|e| format!("控制授权不可用：{e:?}"))?;
                    let bridge = source.reserve_remote_input()?;
                    Some(crate::live_relay_source_runtime::Interaction::new(
                        control_lease,
                        bridge,
                    ))
                } else {
                    None
                };
                let result = self
                    .remote_sessions
                    .start_source(
                        config,
                        lease,
                        source.unit_id().clone(),
                        source.input_identity(),
                        crate::desktop::composition::relay::sessions::SourceCapture {
                            publication,
                            consumer,
                            interaction: interaction.clone(),
                            observation,
                        },
                    )
                    .map_err(|e| e.to_string());
                if result.is_err() {
                    if let Some(interaction) = interaction {
                        interaction.cancel();
                    }
                }
                result.map(|()| true)
            })();
            source.publication_phase(match &result {
                Ok(true) => Phase::Starting,
                Ok(false) => Phase::Idle,
                Err(_) => Phase::Failed,
            });
            self.authorization.borrow_mut().live.status = match result {
                Ok(true) => "正在发布 Live…".into(),
                Ok(false) => String::new(),
                Err(error) => error,
            };
        }
    }
}
