use super::*;
use serde_json::json;
fn value() -> Value {
    json!({"protocol":catalog::PROTOCOL, "apiVersion":catalog::API, "requestId":"request", "status":"succeeded",
        "output":{"text":"合成结果"}, "effects":[{"type":"attachment.upsert", "payload":{"attachmentId":"neuro.official/ocr.result"}}]})
}
fn parse(value: Value) -> Result<Outcome, Error> {
    Outcome::parse(
        value,
        "request",
        "neuro.official/ocr".into(),
        "scope:ocr".into(),
        7,
    )
}
#[test]
fn successful_result_keeps_exact_data_and_captured_authority_without_applying_effects() {
    let outcome = parse(value()).unwrap();
    assert_eq!(outcome.plugin(), "neuro.official/ocr");
    assert_eq!(outcome.scope(), "scope:ocr");
    assert_eq!(outcome.generation(), 7);
    assert_eq!(outcome.output()["text"], "合成结果");
    assert_eq!(outcome.effects()[0].kind(), &EffectKind::AttachmentUpsert);
    assert_eq!(
        outcome.effects()[0].payload()["attachmentId"],
        "neuro.official/ocr.result"
    );
}
#[test]
fn mismatched_request_invalid_protocol_effect_type_and_count_are_not_deliverable() {
    for case in 0..7 {
        let mut data = value();
        match case {
            0 => data["requestId"] = json!("other"),
            1 => data["apiVersion"] = json!("2.0"),
            2 => data["protocol"] = json!("other"),
            3 => data["effects"][0]["type"] = json!("host.execute"),
            4 => data["effects"] = json!(vec![json!({"type":"notice.show", "payload":{}}); 257]),
            5 => data["effects"] = Value::Null,
            _ => data["error"] = json!({"code":"", "message":"invalid code"}),
        }
        assert!(matches!(parse(data), Err(Error::Wire)), "case {case}");
    }
}
#[test]
fn failure_cancel_and_nonterminal_status_never_escape_as_successful_effect_batches() {
    for status in ["failed", "cancelled", "accepted", "progress"] {
        let mut data = value();
        data["status"] = json!(status);
        let result = parse(data);
        match status {
            "failed" => assert!(matches!(result, Err(Error::Denied))),
            "cancelled" => assert!(matches!(result, Err(Error::Cancelled))),
            _ => assert!(matches!(result, Err(Error::Wire))),
        }
    }
    let mut data = value();
    data["status"] = json!("failed");
    data["error"] = json!({"code":"revoked", "message":"grant was revoked"});
    assert!(
        matches!(parse(data), Err(Error::Remote { code, message }) if code == "revoked" && message == "grant was revoked")
    );
}
#[test]
fn optional_null_error_and_one_malformed_effect_payload_do_not_discard_independent_effects() {
    let mut data = value();
    data["error"] = Value::Null;
    data["effects"]
        .as_array_mut()
        .unwrap()
        .push(json!({"type":"notice.show"}));
    let outcome = parse(data).unwrap();
    assert_eq!(outcome.effects().len(), 2);
    assert_eq!(outcome.effects()[1].payload(), &Value::Null);
}
