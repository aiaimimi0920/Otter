//! 磁盘提示到只读恢复查询的认证边界；查询不会重放创建或自动 remount。
use super::*;
use crate::{
    desktop::composition::relay::sessions::Sessions,
    device_authorization_owner::Owner,
    surface_attach_runtime::Request,
    surface_host_capabilities::{Input, Report, Runtime},
    surface_recovery_record::Record,
};
use std::sync::Arc;

fn request(record: &Record, origin: &str, device: &str) -> std::result::Result<Request, String> {
    if record.origin() != origin || record.device() != device {
        return Err("恢复记录与当前服务或设备不一致".into());
    }
    let art = record.art().ok_or("记录缺少 Art，不能查询恢复候选")?;
    let report = Report::new(
        vec![Runtime::Declarative],
        vec![],
        vec![],
        vec![],
        Input::default(),
    )
    .map_err(|error| format!("恢复查询配置无效：{error:?}"))?;
    // 仅作为 GET 查询的 Art/node 上下文；此请求绝不交给 start_attach。
    Request::new(
        art.into(),
        record.node().into(),
        report,
        crate::live_relay_http::Persistence::Temporary,
    )
    .map_err(|error| error.to_string())
}
pub(super) fn query(
    record: &Record,
    selected: Option<&str>,
    auth: &mut Owner,
    sessions: &mut Sessions,
) -> std::result::Result<(), String> {
    if selected != Some(record.origin()) || sessions.querying_recovery() || sessions.attaching() {
        return Err("服务已变化或恢复查询仍在进行".into());
    }
    let now = crate::desktop::surface_windows::now_ms().map_err(|error| error.to_string())?;
    let lease = auth
        .lease(record.origin(), now)
        .map_err(|error| format!("请先获取有效授权：{error:?}"))?;
    let request = request(record, record.origin(), lease.device_id())?;
    let config = crate::device_authorization_config::Config::from_env(record.origin())
        .map_err(|error| format!("配置无效：{error:?}"))?;
    sessions
        .query_surface_recovery(config, lease, Arc::new(request))
        .map_err(|error| format!("恢复查询未启动：{error}"))
}
impl Application {
    pub(super) fn poll_surface_recovery_ui(&mut self) {
        let mut view = self.authorization.borrow_mut();
        if let Some(result) = self.remote_sessions.take_recovery_file() {
            if !view.closed {
                match result {
                    Ok(records) => {
                        view.recovery_status = format!("已读取 {} 条恢复记录", records.len());
                        view.recovery = records.into_iter().map(Arc::new).collect();
                    }
                    Err(error) => {
                        view.recovery.clear();
                        view.recovery_status = format!("恢复文件无效：{error}");
                    }
                }
            }
        }
        while let Some(outcome) = self.remote_sessions.take_recovery_outcome() {
            let current = !view.closed
                && view.origin.as_deref() == Some(&outcome.origin)
                && crate::desktop::surface_windows::now_ms()
                    .ok()
                    .is_some_and(|now| {
                        self.authorization_owner
                            .lease(&outcome.origin, now)
                            .is_ok_and(|lease| lease.device_id() == outcome.device)
                    });
            if !current {
                view.recovery_candidate = None;
                if !view.closed {
                    view.recovery_status = "恢复查询结果已失效，请重新授权后查询".into();
                }
                continue;
            }
            view.recovery_candidate = None;
            match outcome.result {
                Ok(Some(candidate)) => {
                    view.recovery_status = "找到恢复候选，尚未重新挂载".into();
                    view.recovery_candidate = Some(Arc::new(candidate));
                }
                Ok(None) => view.recovery_status = "未找到候选；不能据此确认原创建未生效".into(),
                Err(error) => view.recovery_status = format!("恢复查询未完成：{error:?}"),
            }
        }
        view.recovery_busy = self.remote_sessions.reading_recovery_file()
            || self.remote_sessions.querying_recovery();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn record() -> Record {
        crate::surface_recovery_record::decode(br#"{"version":1,"records":[{"origin":"https://localhost:1234","device":"viewer:test","node":"node:test","art":"art:test","binding":null,"uncertain":true,"inFlight":false}]}"#).unwrap().remove(0)
    }
    #[test]
    fn disk_hint_only_builds_query_context_for_matching_service_and_device() {
        let record = record();
        assert!(request(&record, "https://localhost:5678", "viewer:test").is_err());
        assert!(request(&record, record.origin(), "another-device").is_err());
        let request = request(&record, record.origin(), record.device()).unwrap();
        assert_eq!(request.art(), "art:test");
        assert_eq!(request.node(), "node:test");
        assert!(record.uncertain());
    }
    #[test]
    fn untrusted_disk_record_cannot_start_network_without_current_authorization() {
        let record = record();
        let mut sessions = Sessions::default();
        assert!(query(
            &record,
            Some(record.origin()),
            &mut Owner::default(),
            &mut sessions
        )
        .is_err());
        assert!(!sessions.querying_recovery());
        assert!(sessions.finished());
    }
}
