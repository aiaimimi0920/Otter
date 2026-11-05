//! 用户明确选择 Art 后启动一次临时挂载；能力来自宿主，不从 Art 请求反向宣称。
#[cfg(test)]
pub(super) fn start(
    origin: &str,
    art: String,
    node: String,
    authorization: &mut crate::device_authorization_owner::Owner,
    sessions: &mut crate::desktop::composition::relay::sessions::Sessions,
    windows: &crate::desktop::surface_windows::Windows,
) -> Result<(), String> {
    start_with(
        origin,
        art,
        node,
        authorization,
        sessions,
        windows,
        crate::device_authorization_config::Config::from_env,
    )
}
/// Same admission and owner transfer as the UI entry; tests supply per-origin TLS configuration.
#[cfg(test)]
pub(super) fn start_with(
    origin: &str,
    art: String,
    node: String,
    authorization: &mut crate::device_authorization_owner::Owner,
    sessions: &mut crate::desktop::composition::relay::sessions::Sessions,
    windows: &crate::desktop::surface_windows::Windows,
    config: impl FnOnce(
        &str,
    ) -> Result<
        crate::device_authorization_config::Config,
        crate::device_authorization_config::Error,
    >,
) -> Result<(), String> {
    use crate::{live_relay_http::Persistence, surface_attach_runtime::Request};
    if sessions.attaching() || !windows.can_mount(origin, &node) {
        return Err("节点已有 Surface 或待处理记录，或窗口容量已满".into());
    }
    let now = crate::desktop::surface_windows::now_ms().map_err(|e| e.to_string())?;
    let lease = authorization
        .lease(origin, now)
        .map_err(|e| format!("请先获取有效授权：{e:?}"))?;
    let config = config(origin).map_err(|e| format!("配置无效：{e:?}"))?;
    let request =
        Request::new(art, node, report()?, Persistence::Temporary).map_err(|e| e.to_string())?;
    sessions
        .start_attach(config, lease, std::sync::Arc::new(request))
        .map_err(|e| format!("挂载任务未启动：{e}"))
}
/// 新建和显式恢复使用同一份真实宿主能力，不因恢复路径扩大支持范围。
pub(super) fn report() -> Result<crate::surface_host_capabilities::Report, String> {
    crate::desktop::surface_capabilities::report()
}
#[cfg(test)]
mod tests {
    #[test]
    fn report_advertises_verified_forms_and_bounded_images_without_claiming_scripts() {
        let value = serde_json::to_value(super::report().unwrap()).unwrap();
        assert!(value["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|node| node == "divider"));
        assert!(value["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|node| node == "spacer"));
        assert!(!value["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|node| node == "scroll"));
        assert_eq!(
            value["input"],
            serde_json::json!({"pointer":true,"hover":true,"touch":false,"keyboard":true})
        );
        let nodes = value["nodes"].as_array().unwrap();
        assert!(nodes.iter().any(|node| node == "button"));
        assert!(nodes.iter().any(|node| node == "icon"));
        assert!(nodes.iter().any(|node| node == "image"));
        assert_eq!(
            value["nodes"],
            serde_json::json!([
                "view", "row", "column", "stack", "text", "spacer", "divider", "progress",
                "button", "icon", "input", "textarea", "number", "switch", "select", "slider",
                "image"
            ])
        );
        assert_eq!(value["runtimes"], serde_json::json!(["declarative"]));
        assert_eq!(value["transports"], serde_json::json!(["loom_resource"]));
        assert_eq!(
            value["capabilities"],
            serde_json::json!(["remote_resources"])
        );
    }
    #[test]
    fn opening_art_without_authorization_does_not_start_a_job() {
        let mut auth = Default::default();
        let mut sessions = crate::desktop::composition::relay::sessions::Sessions::default();
        assert!(super::start(
            "https://localhost:1234",
            "art:test".into(),
            "node:test".into(),
            &mut auth,
            &mut sessions,
            &Default::default()
        )
        .is_err());
        assert!(!sessions.attaching());
    }
}
