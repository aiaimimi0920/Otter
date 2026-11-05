//! 认证交付边界测试；全部为本地协议夹具，不发送创建或恢复请求。
use super::*;
#[path = "surface_authorized_window_tests.rs"]
mod authorized_window;
use crate::{
    live_relay_http::Persistence,
    surface_attach_runtime::{AdmissionError, Outcome, Request},
    surface_host_capabilities::{Input, Report, Runtime},
    surface_live_binding::Binding,
};
fn outcome() -> Outcome {
    let origin = "https://localhost:1234";
    let bytes = br#"{"instance":{"descriptor":{"instanceId":"instance:test","generation":1},"attachments":{"attachment:test":{"descriptor":{"attachmentId":"attachment:test","instanceId":"instance:test","hookNodeId":"node:test","deviceId":"viewer:test"},"lifecycle":"mounted","lifecycleRevision":1}}}}"#;
    let binding = Binding::from_mount(bytes, origin, "viewer:test", "node:test").unwrap();
    let report = Report::new(
        vec![Runtime::Declarative],
        vec![],
        vec![],
        vec![],
        Input::default(),
    )
    .unwrap();
    Outcome {
        origin: origin.into(), device: "viewer:test".into(),
        request: Arc::new(Request::new("art:test".into(), "node:test".into(), report, Persistence::Temporary).unwrap()),
        binding: Some(binding), uncertain: false,
        result: Ok(serde_json::value::RawValue::from_string(r#"{"protocolVersion":"loom.surface.v1","instanceId":"instance:test","attachmentId":"attachment:test","revision":1,"scene":{"id":"root","type":"text","props":{"text":"Native"}}}"#.into()).unwrap()),
    }
}
#[test]
fn mount_delivery_requires_current_authorization_and_preserves_evidence() {
    let mut grant = grant();
    let lease = grant.lease("https://localhost:1234", 0).unwrap();
    let result = outcome();
    let snapshot = result.snapshot_for(&lease, 99).unwrap();
    assert_eq!(snapshot.scene().root().id(), "root");
    assert_eq!(snapshot.revision(), 1);
    assert!(matches!(
        result.snapshot_for(&lease, 100),
        Err(AdmissionError::Authorization(_))
    ));
    drop(grant);
    assert!(matches!(
        result.snapshot_for(&lease, 0),
        Err(AdmissionError::Authorization(Error::Revoked))
    ));
    assert!(result.result.is_ok());
    assert!(result.binding.is_some());
}
#[test]
fn mount_delivery_rejects_uncertainty_identity_and_non_declarative_runtime() {
    let mut grant = grant();
    let lease = grant.lease("https://localhost:1234", 0).unwrap();
    let mut result = outcome();
    result.uncertain = true;
    assert!(matches!(
        result.snapshot_for(&lease, 0),
        Err(AdmissionError::Unconfirmed)
    ));
    result.uncertain = false;
    result.device = "other".into();
    assert!(matches!(
        result.snapshot_for(&lease, 0),
        Err(AdmissionError::Identity)
    ));
    result.device = "viewer:test".into();
    let raw = result
        .result
        .as_ref()
        .unwrap()
        .get()
        .replace("instance:test", "wrong");
    result.result = Ok(serde_json::value::RawValue::from_string(raw).unwrap());
    assert!(matches!(
        result.snapshot_for(&lease, 0),
        Err(AdmissionError::Identity)
    ));
    let mut result = outcome();
    let raw =
        result
            .result
            .as_ref()
            .unwrap()
            .get()
            .replacen("{", "{\"runtime\":\"javascript\",", 1);
    result.result = Ok(serde_json::value::RawValue::from_string(raw).unwrap());
    assert!(matches!(
        result.snapshot_for(&lease, 0),
        Err(AdmissionError::Scene(crate::surface_scene::Error::Runtime))
    ));
}

#[test]
fn snapshot_revision_is_retained_without_substituting_lifecycle_revision() {
    let mut grant = grant();
    let lease = grant.lease("https://localhost:1234", 0).unwrap();
    let mut result = outcome();
    let raw = result
        .result
        .as_ref()
        .unwrap()
        .get()
        .replace("\"revision\":1", "\"revision\":57");
    result.result = Ok(serde_json::value::RawValue::from_string(raw).unwrap());
    let snapshot = result.snapshot_for(&lease, 0).unwrap();
    assert_eq!(snapshot.revision(), 57);
    assert_eq!(result.binding.as_ref().unwrap().lifecycle_revision(), 1);
    assert_eq!(snapshot.scene().root().id(), "root");
}

#[test]
fn malformed_or_mismatched_snapshot_never_delivers_a_revision() {
    let mut grant = grant();
    let lease = grant.lease("https://localhost:1234", 0).unwrap();
    for revision in ["0", "-1", "1.5", "null", "\"57\""] {
        let mut result = outcome();
        let raw = result
            .result
            .as_ref()
            .unwrap()
            .get()
            .replace("\"revision\":1", &format!("\"revision\":{revision}"));
        result.result = Ok(serde_json::value::RawValue::from_string(raw).unwrap());
        assert!(matches!(
            result.snapshot_for(&lease, 0),
            Err(AdmissionError::Identity)
        ));
        assert!(result.result.is_ok());
    }
    let mut result = outcome();
    let raw = result
        .result
        .as_ref()
        .unwrap()
        .get()
        .replace("attachment:test", "attachment:other");
    result.result = Ok(serde_json::value::RawValue::from_string(raw).unwrap());
    assert!(matches!(
        result.snapshot_for(&lease, 0),
        Err(AdmissionError::Identity)
    ));
}
