use super::*;
use serde_json::json;
fn owner() -> Owner {
    Owner {
        binding: Binding {
            origin: "https://localhost".into(),
            request_id: "request:1".into(),
            device_id: "device:1".into(),
            workspace_id: "workspace:1".into(),
            instance_id: "instance:1".into(),
            generation: 1,
            art_id: "vendor/art".into(),
            art_version: "1.0.0".into(),
            package_digest: "a".repeat(64),
            sdk_version: "otter.art.v1".into(),
        },
        grant_id: "grant:1".into(),
        call_id: "call:1".into(),
        service: "capability:vendor:service.run".into(),
    }
}
fn reply() -> Reply {
    Reply {
        protocol_version: PROTOCOL.into(),
        owner: owner(),
        sequence: 1,
        status: Status::Accepted,
        output: None,
        error: None,
        progress: None,
    }
}
fn parse(value: &Reply, prior: Option<&Reply>) -> Result<Reply, super::super::Error> {
    Reply::parse(&serde_json::to_vec(value).unwrap(), &owner(), prior)
}
#[test]
fn service_wire_preserves_success_null_but_rejects_contradictory_terminal() {
    let mut value = reply();
    value.status = Status::Succeeded;
    assert_eq!(parse(&value, None), Err(super::super::Error::Wire));
    value.output = Some(Value::Null);
    assert_eq!(parse(&value, None).unwrap().output, Some(Value::Null));
    value.status = Status::Cancelled;
    assert_eq!(parse(&value, None), Err(super::super::Error::Wire));
    value.output = None;
    value.error = Some("secret=not-a-code".into());
    value.status = Status::Failed;
    assert_eq!(parse(&value, None), Err(super::super::Error::Wire));
}
#[test]
fn service_wire_rejects_progress_regression_and_terminal_resurrection() {
    let mut prior = reply();
    prior.status = Status::Running;
    let mut value = reply();
    value.sequence = 2;
    assert_eq!(
        parse(&value, Some(&prior)),
        Err(super::super::Error::Sequence)
    );
    prior.status = Status::Succeeded;
    prior.output = Some(json!({}));
    value.status = Status::Running;
    assert_eq!(
        parse(&value, Some(&prior)),
        Err(super::super::Error::Sequence)
    );
    assert_eq!(parse(&prior, Some(&prior)), Ok(prior.clone()));
}
#[test]
fn service_wire_rejects_unknown_duplicate_and_oversized_reply_fields() {
    let mut value = serde_json::to_value(reply()).unwrap();
    value["credential"] = json!("forged");
    assert_eq!(
        Reply::parse(&serde_json::to_vec(&value).unwrap(), &owner(), None),
        Err(super::super::Error::Wire)
    );
    let text = serde_json::to_string(&reply()).unwrap();
    let duplicate = text.replacen("{", "{\"sequence\":2,", 1);
    assert_eq!(
        Reply::parse(duplicate.as_bytes(), &owner(), None),
        Err(super::super::Error::Wire)
    );
    let mut value = reply();
    value.progress = Some(json!("x".repeat(4096)));
    assert_eq!(parse(&value, None), Err(super::super::Error::Limit));
    assert_eq!(
        Reply::parse(&vec![b' '; MAX_REPLY + 1], &owner(), None),
        Err(super::super::Error::Limit)
    );
}
