use super::*;
fn region() -> Region {
    Region {
        left: -10,
        top: -20,
        width: 4,
        height: 4,
    }
}
fn desktop() -> Region {
    Region {
        left: -100,
        top: -100,
        width: 200,
        height: 200,
    }
}
fn frame() -> Frame {
    Frame::new(4, 4, vec![10; 64].into()).unwrap()
}
#[test]
fn candidate_capture_commits_only_on_finish_and_rejection_preserves_reference() {
    let cancel = AtomicBool::new(false);
    let mut source = Source::new(region(), Axis::Vertical, None);
    assert!(source.can_move());
    assert!(!source.can_scroll());
    source
        .prepare_with((2, -1), &cancel, desktop(), |candidate, target, _| {
            assert_eq!((candidate.left, candidate.top), (-8, -21));
            assert!(target.is_none());
            Ok(frame())
        })
        .unwrap();
    assert_eq!(source.region, region());
    source.finish_move(false);
    assert!(source.pending.is_none());
    assert_eq!(source.region, region());
    source
        .prepare_with((2, -1), &cancel, desktop(), |_, _, _| Ok(frame()))
        .unwrap();
    source.finish_move(true);
    assert_eq!((source.region.left, source.region.top), (-8, -21));
    source
        .prepare_with((-1, 2), &cancel, desktop(), |candidate, _, _| {
            assert_eq!((candidate.left, candidate.top), (-9, -19));
            Ok(frame())
        })
        .unwrap();
    source.finish_move(true);
    assert_eq!((source.region.left, source.region.top), (-9, -19));
    assert_eq!(source.remaining, 511); // 移动只采集，不发送滚轮或消耗滚动次数。
}
#[test]
fn invalid_moves_fail_before_capture_and_capture_errors_clear_candidates() {
    let cancel = AtomicBool::new(false);
    let mut source = Source::new(region(), Axis::Vertical, None);
    for movement in [(4, 0), (0, -4), (i32::MIN, 0), (0, i32::MAX)] {
        assert!(source
            .prepare_with(movement, &cancel, desktop(), |_, _, _| panic!(
                "must not capture"
            ))
            .is_err());
        source.finish_move(false);
        assert_eq!(source.region, region());
    }
    assert!(source
        .prepare_with((1, 0), &cancel, desktop(), |_, _, _| Err("fixture".into()))
        .is_err());
    assert!(source.pending.is_none());
    assert!(source
        .prepare_with((1, 0), &cancel, desktop(), |_, _, _| Frame::new(
            1,
            1,
            vec![0; 4].into()
        )
        .map_err(|e| format!("{e:?}")))
        .is_err());
    assert!(source.pending.is_none());
    assert!(source
        .prepare_with((1, 0), &cancel, desktop(), |_, _, signal| {
            signal.store(true, Ordering::Release);
            Ok(frame())
        })
        .is_err());
    source.finish_move(true); // 迟到的提交也不能提交失败/取消的候选。
    assert!(source.pending.is_none());
    assert_eq!(source.region, region());
}
#[test]
fn geometry_keeps_negative_coordinates_and_checks_edges_and_overflow() {
    assert_eq!(
        translated(region(), (-3, 3), desktop()).unwrap(),
        Region {
            left: -13,
            top: -17,
            ..region()
        }
    );
    assert!(translated(region(), (-1, 0), region()).is_err());
    assert!(translated(region(), (1, 0), region()).is_err());
    assert!(translated(
        Region {
            left: i32::MIN,
            ..region()
        },
        (-1, 0),
        desktop()
    )
    .is_err());
    assert!(translated(
        Region {
            left: i32::MAX - 4,
            ..region()
        },
        (1, 0),
        desktop()
    )
    .is_err());
    assert!(translated(
        region(),
        (0, 0),
        Region {
            width: 0,
            ..desktop()
        }
    )
    .is_err());
}
