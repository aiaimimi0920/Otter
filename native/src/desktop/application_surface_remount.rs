//! 显式候选的一次性恢复入口；不把磁盘提示或重复确认升级成新的创建请求。
use crate::{
    desktop::{
        composition::relay::sessions::Sessions, device_authorization_ui::View,
        surface_windows::Windows,
    },
    device_authorization_owner::Owner,
    live_relay_http::RecoveryCandidate,
};
use std::sync::Arc;

pub(super) fn start(
    view: &mut View,
    candidate: Arc<RecoveryCandidate>,
    authorization: &mut Owner,
    sessions: &mut Sessions,
    windows: &Windows,
    live_count: usize,
) -> Result<(), String> {
    if view.closed
        || view.busy
        || view.recovery_busy
        || view.mount_busy
        || view.origin.as_deref() != Some(candidate.origin())
        || !view
            .recovery_candidate
            .as_ref()
            .is_some_and(|current| Arc::ptr_eq(current, &candidate))
    {
        return Err("恢复候选已变化，请重新查询".into());
    }
    // 消费本次明确选择；任何失败都要求重新查询，不保留可被双击重放的候选。
    view.recovery_candidate = None;
    if sessions.attaching() || !windows.can_mount(candidate.origin(), candidate.node()) {
        return Err("节点已有附件或待处理记录，不能重复挂载".into());
    }
    if !super::live::fits_wait_budget(live_count, windows.wait_budget() + 1) {
        return Err("窗口等待容量已满，请关闭部分 Live 后重新查询".into());
    }
    let now = crate::desktop::surface_windows::now_ms().map_err(|error| error.to_string())?;
    let lease = authorization
        .lease(candidate.origin(), now)
        .map_err(|error| format!("请重新授权并查询：{error:?}"))?;
    if lease.device_id() != candidate.device() {
        return Err("当前设备与候选不一致，请重新查询".into());
    }
    let config = crate::device_authorization_config::Config::from_env(candidate.origin())
        .map_err(|error| format!("配置无效：{error:?}"))?;
    let request =
        crate::surface_attach_runtime::Request::recover(candidate, super::surface_start::report()?);
    sessions
        .start_attach(config, lease, Arc::new(request))
        .map_err(|error| format!("恢复未启动，请重新查询：{error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn view(candidate: &Arc<RecoveryCandidate>) -> View {
        View {
            origin: Some(candidate.origin().into()),
            recovery_candidate: Some(candidate.clone()),
            ..Default::default()
        }
    }
    #[test]
    fn unauthed_confirmation_is_consumed_once_without_network_or_new_art() {
        let candidate = Arc::new(RecoveryCandidate::fixture());
        let mut view = view(&candidate);
        let mut sessions = Sessions::default();
        assert!(start(
            &mut view,
            candidate.clone(),
            &mut Owner::default(),
            &mut sessions,
            &Windows::default(),
            0
        )
        .is_err());
        assert!(view.recovery_candidate.is_none());
        assert!(!sessions.attaching());
        assert!(start(
            &mut view,
            candidate,
            &mut Owner::default(),
            &mut sessions,
            &Windows::default(),
            0
        )
        .is_err());
        assert!(sessions.finished());
    }
    #[test]
    fn stale_equal_candidate_cannot_consume_new_selection_and_capacity_fails_before_network() {
        let current = Arc::new(RecoveryCandidate::fixture());
        let mut view = view(&current);
        let stale = Arc::new(RecoveryCandidate::fixture());
        let mut sessions = Sessions::default();
        assert!(start(
            &mut view,
            stale,
            &mut Owner::default(),
            &mut sessions,
            &Windows::default(),
            0
        )
        .is_err());
        assert!(Arc::ptr_eq(
            view.recovery_candidate.as_ref().unwrap(),
            &current
        ));
        let error = start(
            &mut view,
            current,
            &mut Owner::default(),
            &mut sessions,
            &Windows::default(),
            16,
        )
        .unwrap_err();
        assert!(error.contains("等待容量"));
        assert!(view.recovery_candidate.is_none());
        assert!(sessions.finished());
    }
}
