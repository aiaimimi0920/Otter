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
fn moved_resized_target_uses_current_top_left_and_copied_size() {
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
            x: 620,
            y: 620,
            width: 400,
            height: 200
        }
    );
    assert_eq!(request.frame(None).x, 650); // Retired unit falls back to the saved anchor.
}
#[test]
fn stationary_cursor_cascades_past_the_latest_pin_instead_of_wrapping() {
    for cursor in [(-200, 100), (-150, 140), (200, 300)] {
        let mut session = Session::new(origin());
        let pending = session.request(cursor);
        for step in 1..=10 {
            let request = session.refresh(&pending);
            let frame = request.frame(Some(origin().frame));
            assert_eq!((frame.x, frame.y), (-200 + 20 * step, 100 + 20 * step));
            assert_eq!((frame.width, frame.height), (400, 200));
            session.commit(&request, unit(&format!("pasted-{step}")), frame);
        }
    }
}
#[test]
fn moving_cursor_reanchors_then_stationary_outside_pastes_keep_cascading() {
    let mut session = Session::new(origin());
    for step in 0..5 {
        let request = session.request((1000, -100));
        let frame = request.frame(None);
        assert_eq!((frame.x, frame.y), (950 + 20 * step, -140 + 20 * step));
        session.commit(&request, unit("pasted"), frame);
    }
    let target = PinRect {
        x: 600,
        y: 600,
        ..origin().frame
    };
    let request = session.request((700, 700));
    assert!(!request.continuing());
    let frame = request.frame(Some(target));
    assert_eq!((frame.x, frame.y), (620, 620));
    session.commit(&request, unit("other-target-paste"), frame);
    assert_eq!(session.request((700, 700)).frame(Some(target)).x, 640);
    assert_eq!(session.request((1500, -500)).frame(None).x, 1450);
}
#[test]
fn moving_cursor_inside_original_source_keeps_one_cascade() {
    let mut session = Session::new(origin());
    let points = [
        (-199, 101),
        (-197, 102),
        (199, 299),
        (-180, 280),
        (50, 150),
        (-198, 103),
        (180, 290),
        (-100, 200),
        (100, 180),
        (-199, 102),
    ];
    // 不同坐标的请求均在首个提交前采样；交付时必须刷新累计次数而非固定鼠标。
    let pending: Vec<_> = points
        .into_iter()
        .map(|point| session.request(point))
        .collect();
    for (index, pending) in pending.iter().enumerate() {
        let request = session.refresh(pending);
        assert_eq!(request.source(), (&unit("source"), origin().frame));
        assert!(request.contains_cursor(origin().frame));
        let frame = request.cascade_in_source(origin().frame);
        let offset = 20 * (index as i32 + 1);
        assert_eq!((frame.x, frame.y), (-200 + offset, 100 + offset));
        // 未提交的重试仍在同一层；只有最后的成功提交才能推进下一层。
        assert_eq!(
            session.refresh(pending).cascade_in_source(origin().frame),
            frame
        );
        session.commit(&request, unit(&format!("copy-{index}")), frame);
    }
}
#[test]
fn source_cascade_uses_current_source_rect_and_copied_size() {
    let mut session = Session::new(origin());
    for (index, x) in [600, 800].into_iter().enumerate() {
        let current = PinRect {
            x,
            y: 600,
            width: 1000,
            height: 1000,
        };
        let request = session.request((x + 10, 610));
        let frame = request.cascade_in_source(current);
        let offset = 20 * (index as i32 + 1);
        assert_eq!(
            (frame.x, frame.y, frame.width, frame.height),
            (x + offset, 600 + offset, 400, 200)
        );
        session.commit(&request, unit("pasted"), frame);
    }
    let request = session.request((2000, -100));
    assert_eq!(
        (request.relative_frame().x, request.relative_frame().y),
        (1950, -140)
    );
    let next = Session::new(origin()).request((-199, 101));
    assert_eq!(next.cascade_in_source(origin().frame).x, -180);
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
