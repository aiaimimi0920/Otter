use super::*;
use crate::{
    surface_lifecycle_owner::{Error as OwnerError, Owner},
    surface_live_binding::Lifecycle,
};
#[test]
fn lifecycle_unknown_blocks_replay_until_authenticated_remount_then_disposes() {
    let (listener, client, config) = fixture();
    let mut owner = Owner::new(binding(&client.origin));
    let session = session(&client.origin, u64::MAX);
    let peer = std::thread::spawn(move || {
        for index in 0..3 {
            let mut stream = StreamOwned::new(
                ServerConnection::new(config.clone()).unwrap(),
                accept(&listener),
            );
            let (head, body) = request(&mut stream);
            let mut response = mounted();
            if index == 1 {
                assert!(head.starts_with("POST /v1/surfaces/instances/surface:test%2Fpart/mount "));
                response["instance"]["attachments"]["attachment:1"]["lifecycleRevision"] = 8.into();
                reply(&mut stream, 200, &serde_json::to_vec(&response).unwrap());
            } else {
                assert!(
                    head.starts_with("POST /v1/surfaces/instances/surface:test%2Fpart/lifecycle ")
                );
                assert_eq!(body["protocolVersion"], "loom.surface.v1");
                assert_eq!(body["instanceId"], "surface:test/part");
                assert_eq!(body["attachmentId"], "attachment:1");
                assert_eq!(body["revision"], if index == 0 { 8 } else { 9 });
                assert_eq!(
                    body["state"],
                    if index == 0 { "active" } else { "disposed" }
                );
                let mut ack = response["instance"]["attachments"]["attachment:1"].clone();
                ack["lifecycle"] = body["state"].clone();
                // 模拟服务端已切换，但响应版本损坏；客户端不能自行重试 revision 8。
                ack["lifecycleRevision"] = 9.into();
                reply(&mut stream, 200, &serde_json::to_vec(&ack).unwrap());
            }
        }
    });
    let cancel = Cancel::new();
    let control = Control::new(&client, &session, &cancel);
    assert!(control
        .transition_surface(&mut owner, Lifecycle::Active)
        .is_err());
    assert!(owner.uncertain());
    assert_eq!(owner.binding().next_lifecycle_revision(), Ok(8));
    assert_eq!(
        control.transition_surface(&mut owner, Lifecycle::Active),
        Err(LiveError::SurfaceLifecycle(OwnerError::Uncertain))
    );
    control.recover_surface(&mut owner).unwrap();
    assert!(!owner.uncertain());
    assert_eq!(owner.binding().next_lifecycle_revision(), Ok(9));
    control
        .transition_surface(&mut owner, Lifecycle::Disposed)
        .unwrap();
    assert!(owner.disposed());
    assert!(!owner.uncertain());
    assert_eq!(
        control.transition_surface(&mut owner, Lifecycle::Active),
        Err(LiveError::SurfaceLifecycle(OwnerError::Disposed))
    );
    assert!(matches!(
        control.recover_surface(&mut owner),
        Err(LiveError::SurfaceLifecycle(OwnerError::Disposed))
    ));
    peer.join().unwrap();
}
#[test]
fn lifecycle_invalid_state_and_foreign_binding_fail_before_io_without_marking_unknown() {
    let (_listener, client, _config) = fixture();
    let session = session(&client.origin, u64::MAX);
    let cancel = Cancel::new();
    let control = Control::new(&client, &session, &cancel);
    let mut owner = Owner::new(binding(&client.origin));
    assert_eq!(
        control.transition_surface(&mut owner, Lifecycle::Mounted),
        Err(LiveError::SurfaceLifecycle(OwnerError::State))
    );
    assert!(!owner.uncertain());
    let mut owner = Owner::new(binding("https://other.example"));
    assert!(matches!(
        control.transition_surface(&mut owner, Lifecycle::Active),
        Err(LiveError::SurfaceBinding(_))
    ));
    assert!(!owner.uncertain());
}

#[test]
fn recovery_cannot_rollback_known_revision_or_clear_unknown_without_evidence() {
    let initial = binding("https://loom.example");
    let mut owner = Owner::new(initial.clone());
    let revision = owner.begin(Lifecycle::Active).unwrap();
    let mut ack = mounted()["instance"]["attachments"]["attachment:1"].clone();
    ack["lifecycle"] = "active".into();
    ack["lifecycleRevision"] = revision.into();
    owner
        .acknowledge(
            &serde_json::to_vec(&ack).unwrap(),
            Lifecycle::Active,
            revision,
        )
        .unwrap();
    owner.begin_recovery().unwrap();
    assert!(owner.recover(initial).is_err());
    assert!(owner.uncertain());
    assert_eq!(owner.binding().lifecycle_revision(), 8);
    assert_eq!(owner.begin(Lifecycle::Inactive), Err(OwnerError::Uncertain));
}
