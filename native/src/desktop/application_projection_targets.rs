//! Send-catalog consumers reuse Sessions/Lease; do not capture receive workspaces, alter receive polling, or create durable intents.
use crate::{
    desktop::{
        composition::relay::sessions::Sessions,
        device_authorization_ui::{targets as ui, View},
    },
    device_authorization_config::Config,
    device_authorization_owner::Owner,
    projection_delivery_runtime::{Error, Output},
};
pub(in crate::desktop::application) fn start(
    view: &mut View,
    owner: &mut Owner,
    remote: &mut Sessions,
    config: &mut impl FnMut(&str) -> Result<Config, crate::device_authorization_config::Error>,
) {
    if view.closed
        || view.busy
        || view.projection.busy
        || view.targets.busy
        || remote.projection_pending()
    {
        view.targets.status = "投送目标任务不可用".into();
        return;
    }
    // Preserve the previous catalog's presentation during background queries; busy prevents dispatch from it, while failures still invalidate it.
    view.targets.next = Some(std::time::Instant::now() + std::time::Duration::from_secs(5));
    if !view.targets.automatic {
        view.targets.directory = None;
    }
    let result = (|| {
        let origin = view.origin.as_deref().ok_or("尚未选择服务")?;
        let now =
            crate::desktop::surface_windows::now_ms().map_err(|_| "系统时间无效".to_owned())?;
        let lease = owner
            .lease(origin, now)
            .map_err(|e| format!("请先获取有效授权：{e:?}"))?;
        let config = config(origin).map_err(|e| format!("配置无效：{e:?}"))?;
        remote
            .query_projection_targets(config, lease)
            .map_err(|e| format!("目标目录未启动：{e}"))
    })();
    view.targets.busy = result.is_ok();
    view.targets.status = match result {
        Ok(()) => "正在查询投送目标…".into(),
        Err(error) => error,
    };
    if !view.targets.busy {
        view.targets.directory = None;
    }
}
impl super::Application {
    pub(in crate::desktop::application) fn refresh_projection_targets(
        &mut self,
        config: &mut impl FnMut(&str) -> Result<Config, crate::device_authorization_config::Error>,
    ) {
        let mut view = self.authorization.borrow_mut();
        if !view.targets.automatic || view.closed || self.exit_requested.get() {
            view.targets.next = None;
            return;
        }
        let now = std::time::Instant::now();
        if view.targets.next.is_some_and(|next| next > now) {
            return;
        }
        if view.request.is_some()
            || view.busy
            || view.projection.busy
            || view.targets.busy
            || self.modals.blocked()
            || self.remote_sessions.projection_pending()
            || self.projection_receive.busy()
            || self.projection_edit.busy()
        {
            // Missing one send/receive idle window should not require another full refresh period; the message pump still sleeps for a bounded interval.
            view.targets.next = Some(now + std::time::Duration::from_millis(500));
            return;
        }
        if !view.origin.as_deref().is_some_and(|origin| {
            crate::desktop::surface_windows::now_ms()
                .is_ok_and(|now| self.authorization_owner.token(origin, now).is_some())
        }) {
            view.targets.next = Some(now + std::time::Duration::from_secs(5));
            return;
        }
        start(
            &mut view,
            &mut self.authorization_owner,
            &mut self.remote_sessions,
            config,
        );
    }
}
pub(super) fn finish(view: &mut View, origin: &str, result: Result<Output, Error>) {
    if view.closed || view.origin.as_deref() != Some(origin) {
        return;
    }
    view.targets.busy = false;
    view.targets.next = Some(std::time::Instant::now() + std::time::Duration::from_secs(5));
    view.targets.directory = None;
    view.targets.status = match result {
        Ok(Output::Targets(directory)) if directory.origin() == origin => {
            let status = ui::status(&directory);
            view.targets.directory = Some(std::sync::Arc::new(directory));
            status
        }
        Err(Error::Stopped) => "目标目录已取消，请重新获取授权".into(),
        Err(error) => format!("目标目录查询失败：{error:?}"),
        _ => "目标目录响应不匹配".into(),
    };
}
