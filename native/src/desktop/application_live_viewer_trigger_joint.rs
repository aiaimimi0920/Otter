//! 实际Application Viewer配置接缝→原控制线程；独立Loom父门核终态与业务，不冒充人工菜单导航。
use super::*;
use crate::desktop::composition::relay::window::trigger::{phase_test, submit_test};
use crate::live_relay_viewer_trigger_queue::Phase;
pub(super) unsafe fn run(
    context: &context::Context,
    app: &mut Application,
    hwnd: HWND,
    instance: &str,
    attachment: &str,
) {
    let mut command = serde_json::json!({"bindingId":"binding:native-joint","enabled":true,
        "target":{"surfaceInstanceId":instance,"surfaceAttachmentId":attachment,"surfaceNodeId":"refresh","surfaceEvent":"click","surfaceAction":"refresh_price"},
        "condition":{"conditionId":"condition:native-joint","revision":1,"observationId":context.data["observationId"],
        "operator":"equals","operand":{"path":"/value/text","value":"not-matched"},"stableForMs":500,
        "risingEdge":false,"rearm":false,"minimumConfidence":"exact"}});
    assert!(submit_test(hwnd, &command.to_string()));
    context.spin(app, |_| phase_test(hwnd) == Some(Phase::Confirmed));
    context.record(
        "viewer-trigger-added.json",
        serde_json::json!({"revision":1,"confirmed":true,"configurationSeam":true}),
    );
    command["condition"]["revision"] = serde_json::json!(2);
    command["condition"]["operand"]["value"] = serde_json::json!("value-eight");
    assert!(submit_test(hwnd, &command.to_string()));
    context.spin(app, |_| phase_test(hwnd) == Some(Phase::Confirmed));
    context.record(
        "viewer-trigger-updated.json",
        serde_json::json!({"revision":2,"confirmed":true}),
    );
    context.signal(app, "viewer-disable");
    command["condition"]["revision"] = serde_json::json!(3);
    command["enabled"] = serde_json::json!(false);
    assert!(submit_test(hwnd, &command.to_string()));
    context.spin(app, |_| phase_test(hwnd) == Some(Phase::Confirmed));
    context.record(
        "viewer-trigger-disabled.json",
        serde_json::json!({"revision":3,"confirmed":true,"doesNotCancelReserved":true}),
    );
}
