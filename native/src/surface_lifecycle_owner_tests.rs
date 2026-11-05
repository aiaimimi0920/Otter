//! 已确认 active、进行中 unknown 和终态必须区别，版本号本身不构成显示许可。
use super::*;
#[path = "surface_activation_event_tests.rs"]
mod event;
fn owner(state: &str) -> Owner {
    let bytes = format!(
        r#"{{"instance":{{"descriptor":{{"instanceId":"instance:test","generation":1}},"attachments":{{"attachment:test":{{"descriptor":{{"attachmentId":"attachment:test","instanceId":"instance:test","hookNodeId":"node:test","deviceId":"device:test"}},"lifecycle":"{state}","lifecycleRevision":1}}}}}}}}"#
    );
    Owner::new(
        Binding::from_mount(
            bytes.as_bytes(),
            "https://localhost:1234",
            "device:test",
            "node:test",
        )
        .unwrap(),
    )
}
fn ack(state: Lifecycle, revision: u64) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({"descriptor":{"attachmentId":"attachment:test","instanceId":"instance:test","hookNodeId":"node:test","deviceId":"device:test"},"lifecycle":state,"lifecycleRevision":revision})).unwrap()
}
#[test]
fn active_is_preserved_only_after_matching_ack_and_not_during_unknown() {
    let mut owner = owner("mounted");
    assert_eq!(owner.binding().lifecycle(), Lifecycle::Mounted);
    assert!(!owner.confirmed_active());
    let revision = owner.begin(Lifecycle::Active).unwrap();
    assert!(!owner.confirmed_active());
    assert!(owner
        .acknowledge(
            &ack(Lifecycle::Inactive, revision),
            Lifecycle::Active,
            revision
        )
        .is_err());
    assert!(!owner.confirmed_active());
    owner
        .acknowledge(
            &ack(Lifecycle::Active, revision),
            Lifecycle::Active,
            revision,
        )
        .unwrap();
    assert!(owner.confirmed_active());
    assert_eq!(owner.binding().lifecycle(), Lifecycle::Active);
    let revision = owner.begin(Lifecycle::Inactive).unwrap();
    assert!(!owner.confirmed_active());
    owner
        .acknowledge(
            &ack(Lifecycle::Inactive, revision),
            Lifecycle::Inactive,
            revision,
        )
        .unwrap();
    assert_eq!(owner.binding().lifecycle(), Lifecycle::Inactive);
    assert!(!owner.confirmed_active());
}
#[test]
fn recovered_active_is_not_equivalent_to_disposed_or_unconfirmed_recovery() {
    let mut owner = owner("active");
    assert!(owner.confirmed_active());
    owner.begin_recovery().unwrap();
    assert!(!owner.confirmed_active());
    let replacement = super::tests::owner("active");
    owner.recover(replacement.binding().clone()).unwrap();
    assert!(owner.confirmed_active());
    let revision = owner.begin(Lifecycle::Disposed).unwrap();
    owner
        .acknowledge(
            &ack(Lifecycle::Disposed, revision),
            Lifecycle::Disposed,
            revision,
        )
        .unwrap();
    assert!(owner.disposed());
    assert!(Owner::new(owner.binding().clone()).disposed());
    assert_eq!(owner.binding().lifecycle(), Lifecycle::Disposed);
    assert!(!owner.confirmed_active());
}

#[test]
fn activation_transfers_once_and_accepts_only_its_confirmed_next_state() {
    let mut activation = crate::surface_activation::Activation::new(owner("mounted"));
    let mut worker = activation.take().unwrap();
    assert!(activation.owner().is_none());
    assert!(activation.take().is_none());
    let revision = worker.begin(Lifecycle::Active).unwrap();
    worker
        .acknowledge(
            &ack(Lifecycle::Active, revision),
            Lifecycle::Active,
            revision,
        )
        .unwrap();
    assert!(activation.restore(worker).is_ok());
    assert!(activation.owner().unwrap().confirmed_active());
    assert!(activation.take().is_none());
    assert!(activation.restore(owner("mounted")).is_err());
}

#[test]
fn activation_retains_unknown_and_rejects_wrong_transition_without_losing_owner() {
    let mut activation = crate::surface_activation::Activation::new(owner("mounted"));
    let mut worker = activation.take().unwrap();
    let revision = worker.begin(Lifecycle::Inactive).unwrap();
    worker
        .acknowledge(
            &ack(Lifecycle::Inactive, revision),
            Lifecycle::Inactive,
            revision,
        )
        .unwrap();
    let rejected = activation.restore(worker).err().unwrap();
    assert_eq!(rejected.binding().lifecycle(), Lifecycle::Inactive);
    assert!(activation.owner().is_none());
    let mut unknown = owner("mounted");
    unknown.begin(Lifecycle::Active).unwrap();
    assert!(activation.restore(unknown).is_ok());
    assert!(activation.owner().unwrap().uncertain());
    assert!(!activation.owner().unwrap().confirmed_active());
    assert!(activation.take().is_none());
}

#[test]
fn closing_after_activation_uses_next_inactive_revision_without_disposal() {
    let mut transfer = crate::surface_activation::Activation::new(owner("mounted"));
    let mut active = transfer.take().unwrap();
    assert!(transfer.take_inactive().is_none());
    let revision = active.begin(Lifecycle::Active).unwrap();
    active
        .acknowledge(
            &ack(Lifecycle::Active, revision),
            Lifecycle::Active,
            revision,
        )
        .unwrap();
    assert!(transfer.restore(active).is_ok());
    let mut inactive = transfer.take_inactive().unwrap();
    let next = inactive.begin(Lifecycle::Inactive).unwrap();
    assert_eq!(next, revision + 1);
    inactive
        .acknowledge(&ack(Lifecycle::Inactive, next), Lifecycle::Inactive, next)
        .unwrap();
    assert!(transfer.restore(inactive).is_ok());
    assert_eq!(
        transfer.owner().unwrap().binding().lifecycle(),
        Lifecycle::Inactive
    );
    assert!(!transfer.owner().unwrap().disposed());
    assert!(transfer.take_inactive().is_none());
}

#[test]
fn unknown_deactivation_is_retained_without_replay() {
    let mut transfer = crate::surface_activation::Activation::new(owner("active"));
    let mut pending = transfer.take_inactive().unwrap();
    pending.begin(Lifecycle::Inactive).unwrap();
    assert!(transfer.restore(pending).is_ok());
    assert!(transfer.owner().unwrap().uncertain());
    assert!(transfer.take_inactive().is_none());
    assert!(transfer.take().is_none());
}
