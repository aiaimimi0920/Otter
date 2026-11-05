//! 离线调用固定失败而不连网，范围和取消仍失败关闭。
use super::*;
use services::Dispatcher as _;
#[test]
fn offline_service_fails_only_the_call_and_preserves_scope_and_cancellation() {
    let mut dispatcher = Offline {
        services: vec!["service:echo".into()],
    };
    let call = |service: &str| services::Call {
        image_output: false,
        confirmation: false,
        id: 1,
        service: service.into(),
        input: serde_json::Value::Null,
        timeout_ms: 100,
        images: Vec::new(),
        canvases: Vec::new(),
    };
    let deadline = Instant::now() + Duration::from_secs(1);
    assert!(
        matches!(dispatcher.invoke(call("service:echo"),deadline,&||false).unwrap(),services::ResultValue::Failed{error} if error=="offline")
    );
    assert!(dispatcher
        .invoke(call("service:other"), deadline, &|| false)
        .is_err());
    assert!(dispatcher
        .invoke(call("service:echo"), deadline, &|| true)
        .is_err());
}
