use super::*;
fn record() -> Record {
    let attachment = crate::surface_recovery_record::decode(br#"{"version":1,"records":[{"origin":"https://localhost:1234","device":"viewer:test","node":"node:test","art":null,"binding":{"instance":"instance:test","attachment":"attachment:test","generation":1,"lifecycle":"active","revision":2},"uncertain":false,"inFlight":false}]}"#).unwrap().remove(0);
    Record {
        batch: None,
        result_revision: None,
        decision: None,
        attachment,
        event_id: "event:test".into(),
        base_revision: 1,
        attempted: true,
        ack: Some(AckHint {
            request_id: "request:test".into(),
            accepted: true,
            status: Status::Succeeded,
        }),
        convergence: Stage::Succeeded(2),
        presentation: Stage::Succeeded(2),
    }
}
#[test]
fn bounded_metadata_round_trip_has_no_payload_scene_or_credentials() {
    let record = record();
    let bytes = encode(std::slice::from_ref(&record)).unwrap();
    assert_eq!(decode(&bytes).unwrap(), vec![record.clone()]);
    let text = std::str::from_utf8(&bytes).unwrap();
    for forbidden in ["payload", "scene", "token", "password", "error", "detail"] {
        assert!(!text.contains(forbidden));
    }
    assert_eq!(record.status(), Some(Status::Succeeded));
    assert_eq!(record.convergence(), &Stage::Succeeded(2));
    assert!(record.attempted());
    assert!(encode(&vec![record; MAX_RECORDS + 1]).is_err());
    assert!(decode(&vec![b' '; MAX_BYTES + 1]).is_err());
}
#[test]
fn strict_decoder_rejects_invalid_identity_versions_and_inconsistent_evidence() {
    let value: serde_json::Value = serde_json::from_slice(&encode(&[record()]).unwrap()).unwrap();
    for (pointer, replacement) in [
        ("/version", serde_json::json!(2)),
        ("/records/0/attachment/binding", serde_json::Value::Null),
        ("/records/0/eventId", serde_json::json!("")),
        ("/records/0/attempted", serde_json::json!(false)),
        ("/records/0/baseRevision", serde_json::json!(0)),
        ("/records/0/ack/status", serde_json::json!("running")),
        ("/records/0/convergence/revision", serde_json::json!(0)),
        ("/records/0/presentation/revision", serde_json::json!(99)),
    ] {
        let mut invalid = value.clone();
        *invalid.pointer_mut(pointer).unwrap() = replacement;
        assert!(
            decode(&serde_json::to_vec(&invalid).unwrap()).is_err(),
            "{pointer}"
        );
    }
    let mut invalid = value;
    invalid["records"][0]["payload"] = serde_json::json!({});
    assert!(decode(&serde_json::to_vec(&invalid).unwrap()).is_err());
}
#[test]
fn event_store_is_append_only_and_separate_from_attachment_records() {
    let guid = unsafe { windows::Win32::System::Com::CoCreateGuid() }.unwrap();
    let root = std::path::PathBuf::from("C:/Users/Public/nas_home/AI/GameEditor/linshi")
        .join(format!("surface-event-record-{guid:?}"));
    let records = vec![record()];
    let first = crate::surface_recovery_store::save_events(&root, &records).unwrap();
    let second = crate::surface_recovery_store::save_events(&root, &records).unwrap();
    assert_ne!(first, second);
    assert_eq!(
        crate::surface_recovery_store::load_events(&first).unwrap(),
        records
    );
    assert!(crate::surface_recovery_store::load(&first).is_err());
    assert!(crate::surface_recovery_store::save_events(&root, &[]).is_err());
    std::fs::write(&second, b"{").unwrap();
    assert!(crate::surface_recovery_store::load_events(&second).is_err());
    assert_eq!(
        crate::surface_recovery_store::load_events(&first).unwrap(),
        records
    );
}

#[test]
fn decision_metadata_is_optional_but_cannot_claim_unattempted_confirmation() {
    let legacy = record();
    assert!(
        decode(&encode(std::slice::from_ref(&legacy)).unwrap()).unwrap()[0]
            .decision()
            .is_none()
    );
    let mut value: serde_json::Value = serde_json::from_slice(&encode(&[legacy]).unwrap()).unwrap();
    value["records"][0]["decision"] = serde_json::json!({"confirmationId":"confirmation:test","approved":false,"attempted":true,"confirmed":true});
    let decoded = decode(&serde_json::to_vec(&value).unwrap()).unwrap();
    assert!(decoded[0].decision().unwrap().confirmed());
    assert!(!decoded[0].decision().unwrap().approved());
    value["records"][0]["decision"]["attempted"] = false.into();
    assert!(decode(&serde_json::to_vec(&value).unwrap()).is_err());
    value["records"][0]["decision"]["attempted"] = true.into();
    value["records"][0]["decision"]["confirmationId"] = "bad\n".into();
    assert!(decode(&serde_json::to_vec(&value).unwrap()).is_err());
}
