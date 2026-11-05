//! 发送目录消费者复用 Sessions/Lease；不捕获接收工作区、不改接收轮询或创建持久意图。
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
    // 每次请求先丢旧快照；失败不能留下看似新鲜的目标或组。
    view.targets.directory = None;
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
}
pub(super) fn finish(view: &mut View, origin: &str, result: Result<Output, Error>) {
    if view.closed || view.origin.as_deref() != Some(origin) {
        return;
    }
    view.targets.busy = false;
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
