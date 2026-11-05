use super::*;
fn unit(value: &str) -> UnitId {
    UnitId::parse(value).unwrap()
}
fn origin() -> Origin {
    Origin {
        unit: unit("source"),
        frame: PinRect {
            x: -200,
            y: 100,
            width: 400,
            height: 200,
        },
        offset: (50, 40),
        step: 20,
    }
}
#[test]
fn cursor_relative_and_cascade_preserve_copied_physical_size() {
    let mut session = Session::new(origin());
    let first = session.request((-150, 140));
    let frame = first.frame(None);
    assert_eq!(
        frame,
        PinRect {
            x: -180,
            y: 120,
            width: 400,
            height: 200
        }
    );
    // No commit means cancellation/failure does not consume a cascade step.
    assert_eq!(session.request((-150, 140)).frame(None), frame);
    session.commit(&first, unit("pasted"), frame);
    assert_eq!(session.refresh(&first).frame(None).x, -160);
    let moved = session.request((1000, -100));
    assert_eq!(
        moved.frame(None),
        PinRect {
            x: 950,
            y: -140,
            width: 400,
            height: 200
        }
    );
}
#[test]
fn moved_resized_anchor_affects_membership_not_copied_size_or_next_cascade() {
    let session = Session::new(origin());
    let request = session.request((700, 700));
    let current = PinRect {
        x: 600,
        y: 600,
        width: 1000,
        height: 1000,
    };
    assert_eq!(
        request.frame(Some(current)),
        PinRect {
            x: -180,
            y: 120,
            width: 400,
            height: 200
        }
    );
    assert_eq!(request.frame(None).x, 650); // Retired unit falls back to the saved anchor.
}
#[test]
fn old_request_cannot_advance_a_new_copy_or_replay_a_committed_step() {
    let mut session = Session::new(origin());
    let request = session.request((-150, 140));
    let frame = request.frame(None);
    session.commit(&request, unit("first"), frame);
    session.commit(&request, unit("replay"), PinRect { x: 99, ..frame });
    assert_eq!(session.request((-150, 140)).unit(), &unit("first"));
    let mut next_copy = Session::new(origin());
    next_copy.commit(&request, unit("stale"), frame);
    assert_eq!(next_copy.request((0, 0)).unit(), &unit("source"));
    assert_eq!(next_copy.refresh(&request).unit(), &unit("source"));
}
#[test]
fn inside_edges_preserve_web_clipboard_inclusive_semantics() {
    let session = Session::new(origin());
    for cursor in [(-200, 100), (200, 100), (-200, 300), (200, 300)] {
        assert_eq!(session.request(cursor).frame(None).x, -180);
    }
    assert_eq!(session.request((201, 100)).frame(None).x, 151);
    assert_eq!(session.request((-200, 301)).frame(None).y, 261);
}
#[test]
fn negative_monitors_and_integer_edges_remain_valid() {
    let mut source = origin();
    source.frame.x = i32::MAX - source.frame.width as i32;
    source.frame.y = i32::MIN;
    let session = Session::new(source);
    assert!(valid(session.request((i32::MAX, i32::MIN)).frame(None)));
    assert!(valid(session.request((i32::MIN, i32::MAX)).frame(None)));
    assert!(rectangle(RECT {
        left: i32::MIN,
        top: 0,
        right: i32::MAX,
        bottom: 1
    })
    .is_none());
}
