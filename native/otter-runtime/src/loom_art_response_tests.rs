//! 对照 art_transport/formal_delivery 的包络和提交身份；不访问资源或发布 UI。
use super::*;
use serde_json::json;
fn request() -> Request {
    Request {
        id: UnitId::parse("request").unwrap(),
        generation: 3,
    }
}
fn commit() -> Value {
    json!({"protocolVersion":"loom.hook.v1","requestId":"request","nodeId":"unit","generation":3,"resultRevision":2,"previewRevision":1,"value":{"kind":"text","text":"preview"},"outputs":{"output":{"kind":"text","text":"final"}},"candidates":[1,2]})
}
fn message(method: &str) -> Value {
    json!({"protocolVersion":"loom.hook.v1","requestId":"request","method":method,"params":commit()})
}
fn decode(value: &Value) -> Result<Event, Error> {
    parse(
        &serde_json::to_vec(value).unwrap(),
        &UnitId::parse("unit").unwrap(),
        &request(),
    )
}

#[test]
fn method_and_status_result_envelopes_preserve_outputs_and_candidates() {
    let expected = Event::Result {
        revision: 2,
        outputs: commit()["outputs"].clone(),
        candidates: Some(json!([1, 2])),
    };
    assert_eq!(decode(&message("loom.hook.art.result")), Ok(expected));
    let status = json!({"protocolVersion":"loom.hook.v1","requestId":"request","status":"succeeded","data":commit()});
    assert!(matches!(
        decode(&status),
        Ok(Event::Result { revision: 2, .. })
    ));
    assert_eq!(
        decode(&message("loom.hook.art.preview")),
        Ok(Event::Preview {
            revision: 1,
            value: commit()["value"].clone()
        })
    );
}
#[test]
fn stale_preview_is_ignored_but_invalid_final_is_terminal_error() {
    for (field, wrong) in [
        ("nodeId", json!("other")),
        ("requestId", json!("old")),
        ("generation", json!(2)),
        ("protocolVersion", json!("wrong")),
        ("resultRevision", json!(0)),
        ("resultRevision", json!(-1)),
    ] {
        let mut value = message("loom.hook.art.result");
        value["params"][field] = wrong.clone();
        assert_eq!(decode(&value), Err(Error::InvalidResult));
        value["method"] = json!("loom.hook.art.preview");
        let preview_field = if field == "resultRevision" {
            "previewRevision"
        } else {
            field
        };
        value["params"][preview_field] = wrong;
        assert_eq!(decode(&value), Ok(Event::Ignore));
    }
}
#[test]
fn unrelated_and_malformed_messages_are_ignored() {
    let unit = UnitId::parse("unit").unwrap();
    for bytes in [b"broken".as_slice(), b"[]", b"null", b"\xff"] {
        assert_eq!(parse(bytes, &unit, &request()), Ok(Event::Ignore));
    }
    let mut value = message("loom.hook.art.result");
    value["protocolVersion"] = json!("wrong");
    assert_eq!(decode(&value), Ok(Event::Ignore));
    value["protocolVersion"] = json!("loom.hook.v1");
    value["requestId"] = json!("other");
    value["params"]["requestId"] = json!("other");
    assert_eq!(decode(&value), Ok(Event::Ignore));
    assert_eq!(decode(&message("unknown")), Ok(Event::Ignore));
}
#[test]
fn progress_and_failure_match_main_defaults_with_bounded_error_text() {
    let mut value = message("loom.hook.art.progress");
    value["params"]["value"] = json!(0.75);
    assert_eq!(decode(&value), Ok(Event::Progress(0.75)));
    value["params"]["value"] = json!("invalid");
    assert_eq!(decode(&value), Ok(Event::Progress(0.0)));
    value["method"] = json!("loom.hook.art.failure");
    assert_eq!(
        decode(&value),
        Ok(Event::Failure("Art execution failed".into()))
    );
    value["params"]["error"] = json!({"message":"错".repeat(2000)});
    assert_eq!(decode(&value), Ok(Event::Failure("错".repeat(1024))));
    assert_eq!(
        decode(
            &json!({"protocolVersion":"loom.hook.v1","requestId":"request","status":"cancelled","error":{"message":"cancelled"}})
        ),
        Ok(Event::Failure("cancelled".into()))
    );
}
