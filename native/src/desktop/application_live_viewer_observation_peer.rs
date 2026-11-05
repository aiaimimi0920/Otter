//! TLS测试的独立常量业务oracle；不调用产品formatter或触发执行，不使用真实设备。
use super::*;
fn observation(sequence: u64, value: u64) -> Value {
    json!({"observationId":"progress","sequence":sequence,"state":"stable","source":"app_adapter","confidence":"high","observedAtMs":100+sequence,"stableSinceMs":100,"value":{"progress":value}})
}
fn audit(finalized: bool) -> Value {
    let mut v = json!({"triggerId":"t","bindingId":"binding:progress","conditionRevision":1,"observationId":"progress","observationSequence":1,"sourceDeviceId":"source:1","observationSource":"app_adapter","idempotencyKey":"trigger:key","outcome":"fired","evaluatedAtMs":102,"authorizedBy":"device-old","reason":"surface_action_dispatch_reserved"});
    if finalized {
        v["reason"] = Value::Null;
        v["actionRequestId"] = json!("action:123");
    }
    v
}
pub(super) fn decorate(response: &mut Value, stage: usize) {
    let snapshot = if stage == 2 || stage == 3 {
        &mut response["sessions"][0]
    } else if stage == 4 {
        response
    } else {
        return;
    };
    snapshot["session"]["observationCapabilities"] = json!(["adapter"]);
    snapshot["observations"] = json!([observation(1, 7)]);
    snapshot["triggerAudits"] = json!([audit(false)]);
    snapshot["triggers"] = json!([{
        "binding":{"bindingId":"binding:progress","observationId":"progress","conditionRevision":1,"authorizedBy":"device-old","enabled":true},
        "condition":{"conditionId":"condition:progress","revision":1,"observationId":"progress","operator":"greater_or_equal","operand":{"path":"/progress","value":8},"stableForMs":500,"risingEdge":true,"rearm":true,"minimumConfidence":"high"},
        "target":{"surfaceInstanceId":"instance:test","surfaceAttachmentId":"attachment:test","surfaceNodeId":"label","surfaceEvent":"click","surfaceAction":"run"},"armed":true,"lastMatch":false,"pendingOfflineMatch":false
    }]);
}
pub(super) fn delta() -> Vec<Value> {
    [("observation",observation(2,9)),("trigger_event",audit(true))].into_iter().enumerate().map(|(i,(kind,payload))|json!({"protocolVersion":"loom.live.v1","sessionId":"live:viewer","epoch":1,"sequence":8+i,"messageType":kind,"payload":payload})).collect()
}
