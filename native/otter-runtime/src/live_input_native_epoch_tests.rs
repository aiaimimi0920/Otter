//! Native publication fixes the UI admission epoch; owned source hooks/windows, without loading the product DLL or performing media operations.
use super::*;
use crate::live_window_target::recovery::popup_layer::fixture::Fixture;

fn destination(fixture: &Fixture) -> Destination {
    let root = Target::bind(fixture.root.0).unwrap();
    Destination::bind(&root, root.hwnd()).unwrap()
}

#[test]
fn old_control_cannot_replace_a_current_committed_publication() {
    let fixture = Fixture::new();
    let target = destination(&fixture);
    let old = fixture.owner.identity().capture_control().unwrap();
    fixture.owner.control(false).unwrap();
    fixture.owner.control(true).unwrap();
    let current = fixture.owner.identity().capture_control().unwrap();
    let packet = Message::motion([3, 4], 0).unwrap();
    let publication =
        Publication::publish(current, &target, packet, 201, Default::default()).unwrap();
    assert!(matches!(
        Publication::publish(old, &target, packet, 202, Default::default()),
        Err(Error::ControlRevoked)
    ));
    assert_eq!(protocol::value(fixture.b.0, protocol::COMMIT), 201);
    assert_eq!(
        protocol::value(fixture.b.0, protocol::REQUEST_EPOCH),
        current.epoch()
    );
    assert!(protocol::Request::read(fixture.b.0, 201, target.hwnd()).is_some());
    drop(publication);
    assert_eq!(protocol::value(fixture.b.0, protocol::COMMIT), 0);
}

#[test]
fn source_rejects_committed_old_epoch_after_resume_instead_of_rebinding_it() {
    let fixture = Fixture::new();
    let target = destination(&fixture);
    let original = fixture.owner.identity().capture_control().unwrap();
    let publication = Publication::publish(
        original,
        &target,
        Message::motion([3, 4], 0).unwrap(),
        203,
        Default::default(),
    )
    .unwrap();
    let request = protocol::Request::read(fixture.b.0, 203, target.hwnd()).unwrap();
    assert_eq!(request.epoch, original.epoch());
    fixture.owner.control(false).unwrap();
    fixture.owner.control(true).unwrap();
    assert!(protocol::Request::read(fixture.b.0, 203, target.hwnd()).is_none());
    assert_eq!(
        protocol::value(fixture.b.0, protocol::REQUEST_EPOCH),
        original.epoch()
    );
    assert!(!publication.finished());
    drop(publication);
    assert_eq!(protocol::value(fixture.b.0, protocol::COMMIT), 0);
}

#[test]
fn completed_publication_remains_completed_after_control_is_revoked() {
    let fixture = Fixture::new();
    let target = destination(&fixture);
    let control = fixture.owner.identity().capture_control().unwrap();
    let publication = Publication::publish(
        control,
        &target,
        Message::motion([3, 4], 0).unwrap(),
        204,
        Default::default(),
    )
    .unwrap();
    let wire = publication.message().unwrap();
    unsafe {
        SendMessageW(target.hwnd(), wire.id, Some(wire.wp), Some(wire.lp));
    }
    assert!(
        publication.finished(),
        "real source hook must finish the request"
    );
    fixture.owner.control(false).unwrap();
    assert!(!control.valid());
    assert!(
        publication.finished(),
        "completion belongs to nonce, not current epoch"
    );
}

#[test]
fn old_publication_drop_does_not_clear_the_next_requests_properties() {
    let fixture = Fixture::new();
    let target = destination(&fixture);
    let control = fixture.owner.identity().capture_control().unwrap();
    let old = Publication::publish(
        control,
        &target,
        Message::motion([3, 4], 0).unwrap(),
        205,
        Default::default(),
    )
    .unwrap();
    let current = Publication::publish(
        control,
        &target,
        Message::motion([5, 6], 0).unwrap(),
        206,
        Default::default(),
    )
    .unwrap();
    drop(old);
    assert_eq!(protocol::value(fixture.b.0, protocol::COMMIT), 206);
    assert!(protocol::Request::read(fixture.b.0, 206, target.hwnd()).is_some());
    drop(current);
    for key in protocol::request_keys() {
        assert_eq!(protocol::value(fixture.b.0, key), 0);
    }
}

#[test]
fn committed_pending_or_rejected_modal_cannot_be_classified_as_unsent_revocation() {
    let fixture = Fixture::new();
    let target = destination(&fixture);
    let control = fixture.owner.identity().capture_control().unwrap();
    let mut modal = Modal::default();
    modal.token = 207;
    modal.destination = Some(target.clone());
    modal.publication = Some(
        Publication::publish(
            control,
            &target,
            Message::motion([3, 4], 0).unwrap(),
            207,
            Default::default(),
        )
        .unwrap(),
    );
    CALLBACK.with(|slot| slot.set((207, false)));
    fixture.owner.control(false).unwrap();
    assert_eq!(modal.check_revoked_control(), Err(Error::DeliveryFailed));
    // A returned callback without source confirmation still fails; an epoch change must not disguise it as certain nondelivery.
    CALLBACK.with(|slot| slot.set((207, true)));
    assert_eq!(modal.check_revoked_control(), Err(Error::DeliveryFailed));
    assert_eq!(modal.token, 0);
    assert!(modal.publication.is_none());
}
