//! 控制消息身份、释放边界与回执状态；无共享内存操作。
use super::*;
fn id(value: &str) -> UnitId {
    UnitId::parse(value).unwrap()
}
#[test]
fn release_preserves_execution_identity_and_rejects_unbounded_or_duplicate_handles() {
    let request = Request {
        id: id("execution"),
        generation: 7,
    };
    let wire = release_message(
        &id("unit"),
        &request,
        &id("release:fixture"),
        &["Loom_Buffer_fixture"],
    )
    .unwrap();
    let value: Value = serde_json::from_str(&wire).unwrap();
    assert_eq!(value["method"], "loom.hook.art.resources.release");
    assert_eq!(
        value["params"],
        json!({"protocolVersion":"loom.hook.v1","requestId":"release:fixture","executionRequestId":"execution","nodeId":"unit","generation":7,"deviceId":"device:local","handles":["Loom_Buffer_fixture"]})
    );
    for handles in [
        vec![],
        vec!["foreign"],
        vec!["Loom_Buffer_"],
        vec!["Loom_Buffer_a", "Loom_Buffer_a"],
        vec!["Loom_Buffer_\n"],
        vec!["Loom_Buffer_a"; 129],
    ] {
        assert_eq!(
            release_message(&id("unit"), &request, &id("release:fixture"), &handles),
            Err(Error::InvalidHandles)
        );
    }
}
#[test]
fn receipt_filters_identity_and_distinguishes_missing_request_from_failure() {
    let mut value =
        json!({"protocolVersion":"loom.hook.v1","requestId":"request","status":"succeeded"});
    let decode = |value: &Value| response(&serde_json::to_vec(value).unwrap(), &id("request"));
    assert_eq!(decode(&value), Ok(Some(Outcome::Succeeded)));
    value["status"] = json!("failed");
    value["error"] = json!({"code":"request_not_found","message":"secret remote text"});
    assert_eq!(decode(&value), Ok(Some(Outcome::RequestNotFound)));
    value["error"]["code"] = json!("denied");
    assert_eq!(decode(&value), Ok(Some(Outcome::Failed)));
    value["status"] = json!("cancelled");
    assert_eq!(decode(&value), Ok(Some(Outcome::Cancelled)));
    value["status"] = json!("unknown");
    assert_eq!(decode(&value), Err(Error::InvalidResponse));
    value["requestId"] = json!("other");
    assert_eq!(decode(&value), Ok(None));
    value["requestId"] = json!("request");
    value["method"] = json!("notification");
    assert_eq!(decode(&value), Ok(None));
    assert_eq!(response(b"broken", &id("request")), Ok(None));
    assert_eq!(
        response(&vec![b' '; MAX_BYTES + 1], &id("request")),
        Err(Error::TooLarge)
    );
}
