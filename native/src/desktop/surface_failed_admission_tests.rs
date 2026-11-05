//! 未建窗附件的停用与证据转交；纯协议夹具，不创建窗口或发送网络请求。
use super::*;
use crate::{
    live_relay_http::Persistence,
    surface_attach_runtime::{Error, Request},
    surface_host_capabilities::{Input, Report, Runtime},
    surface_live_binding::Binding,
};
use std::sync::Arc;

fn record() -> surface_attach_runtime::Outcome {
    let origin = "https://localhost:1234";
    let bytes = br#"{"instance":{"descriptor":{"instanceId":"instance:test","generation":1},"attachments":{"attachment:test":{"descriptor":{"attachmentId":"attachment:test","instanceId":"instance:test","hookNodeId":"node:test","deviceId":"viewer:test"},"lifecycle":"mounted","lifecycleRevision":1}}}}"#;
    surface_attach_runtime::Outcome {
        origin: origin.into(),
        device: "viewer:test".into(),
        request: Arc::new(
            Request::new(
                "art:test".into(),
                "node:test".into(),
                Report::new(
                    vec![Runtime::Declarative],
                    vec![],
                    vec![],
                    vec![],
                    Input::default(),
                )
                .unwrap(),
                Persistence::Temporary,
            )
            .unwrap(),
        ),
        binding: Some(Binding::from_mount(bytes, origin, "viewer:test", "node:test").unwrap()),
        result: Err(Error::Stopped),
        uncertain: false,
    }
}
fn acknowledge(owner: &mut Owner) {
    let state = Lifecycle::Inactive;
    let revision = owner.begin(state).unwrap();
    let binding = owner.binding();
    let bytes = serde_json::to_vec(&serde_json::json!({"descriptor":{
        "instanceId":binding.instance(), "attachmentId":binding.attachment(),
        "hookNodeId":binding.node(), "deviceId":binding.device()},
        "lifecycle":"inactive", "lifecycleRevision":revision}))
    .unwrap();
    owner.acknowledge(&bytes, state, revision).unwrap();
}

#[test]
fn confirmed_failed_admission_can_stop_once_without_window_or_snapshot() {
    let mut failed = FailedAdmission::new(record(), "unsupported scene".into(), false);
    let original = failed.outcome.binding.clone();
    let mut owner = failed.begin_inactive().unwrap();
    assert!(failed.pending());
    assert!(failed.begin_inactive().is_none());
    acknowledge(&mut owner);
    assert!(failed
        .accept(Outcome {
            owner,
            result: Ok(None)
        })
        .is_ok());
    assert!(failed.settled());
    let mut records = Vec::new();
    failed.append_recovery_records(&mut records);
    let saved: serde_json::Value =
        serde_json::from_slice(&crate::surface_recovery_record::encode(&records).unwrap()).unwrap();
    assert_eq!(saved["records"][0]["binding"]["lifecycle"], "inactive");
    assert_eq!(saved["records"][0]["binding"]["revision"], 2);
    assert!(!failed.pending());
    assert!(failed.begin_inactive().is_none());
    assert_eq!(failed.outcome.binding, original);
    assert_eq!(failed.outcome.result.unwrap_err(), Error::Stopped);
}

#[test]
fn unknown_duplicate_and_mismatched_identity_never_create_cleanup_owner() {
    let mut unknown = record();
    unknown.uncertain = true;
    let mut wrong_device = record();
    wrong_device.device = "another-device".into();
    let mut wrong_origin = record();
    wrong_origin.origin = "https://localhost:5678".into();
    for (record, duplicate) in [
        (unknown, false),
        (wrong_device, false),
        (wrong_origin, false),
        (record(), true),
    ] {
        let mut failed = FailedAdmission::new(record, "failed".into(), duplicate);
        assert!(failed.begin_inactive().is_none());
        assert!(!failed.settled());
    }
}

#[test]
fn failed_start_and_unknown_reply_preserve_owner_without_retry() {
    for unknown in [false, true] {
        let mut failed = FailedAdmission::new(record(), "failed".into(), false);
        let mut owner = failed.begin_inactive().unwrap();
        if unknown {
            owner.begin(Lifecycle::Inactive).unwrap();
            assert!(failed
                .accept(Outcome {
                    owner,
                    result: Err(crate::surface_lifecycle_runtime::Error::Stopped)
                })
                .is_ok());
        } else {
            failed.failed_start(StartError {
                owner: Box::new(owner),
                error: std::io::ErrorKind::WouldBlock.into(),
            });
        }
        assert!(!failed.settled());
        assert_eq!(failed.lifecycle().unwrap().uncertain(), unknown);
        assert!(failed.begin_inactive().is_none());
    }
}

#[test]
fn collection_keeps_pending_failure_and_routes_ack_without_display() {
    use super::super::Windows;
    let mut failed = FailedAdmission::new(record(), "unsupported scene".into(), false);
    let mut owner = failed.begin_inactive().unwrap();
    let mut windows = Windows::default();
    windows.failures.push(failed);
    assert!(windows.take_failure().is_none());
    acknowledge(&mut owner);
    assert!(windows
        .accept_activation(
            Outcome {
                owner,
                result: Ok(None)
            },
            0,
            true
        )
        .is_ok());
    assert!(windows.wait_handles().is_empty());
    assert!(windows.settle_shutdown(&mut Default::default(), 0).is_ok());
    assert!(windows.take_failure().unwrap().settled());
}

#[test]
fn duplicate_failed_mount_cannot_start_a_second_cleanup_transaction() {
    use super::super::Windows;
    let mut windows = Windows::default();
    for _ in 0..2 {
        assert!(windows
            .admit(record(), |_| panic!("stopped scene must not authorize"), 0)
            .is_ok());
    }
    assert!(windows.failures[0].begin_inactive().is_some());
    assert!(windows.failures[1].begin_inactive().is_none());
    assert!(windows.take_failure().unwrap().lifecycle().is_none());
    assert!(windows.take_failure().is_none());
}
