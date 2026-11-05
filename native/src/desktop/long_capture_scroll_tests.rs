use super::*;
#[test]
fn relocating_preserves_identity_or_downgrades_outside_the_window() {
    let identity = Identity {
        window: 1,
        root: 2,
        process: 3,
        thread: 4,
        bounds: (-100, -100, 100, 100),
        child_bounds: (-100, -100, 100, 100),
        dpi: 96,
    };
    let target = Target {
        identity,
        point: (0, 0),
    };
    let region = Region {
        left: -90,
        top: -80,
        width: 20,
        height: 10,
    };
    let next = target.region_target(region).unwrap().unwrap();
    assert_eq!(next.identity, identity);
    assert_eq!(next.point, (-80, -75));
    assert_eq!(target.point, (0, 0));
    assert!(target
        .region_target(Region { left: 90, ..region })
        .unwrap()
        .is_none());
    assert!(target
        .region_target(Region {
            left: -101,
            ..region
        })
        .unwrap()
        .is_none());
    assert!(target
        .region_target(Region {
            left: i32::MAX,
            ..region
        })
        .is_err());
    let wide = Target {
        identity: Identity {
            bounds: (0, 0, 50000, 50000),
            ..identity
        },
        point: (0, 0),
    };
    assert!(wide
        .region_target(Region {
            left: 40000,
            top: 0,
            width: 20,
            height: 20
        })
        .unwrap()
        .is_none());
}
#[test]
fn signed_coordinates_and_both_wheel_directions_are_preserved() {
    let point = center(Region {
        left: -2000,
        top: -100,
        width: 200,
        height: 100,
    })
    .unwrap();
    assert_eq!(point, (-1900, -50));
    for (axis, expected) in [(Axis::Vertical, -120), (Axis::Horizontal, 120)] {
        for forward in [true, false] {
            let (msg, wp, lp) = message(axis, forward, point);
            assert!(matches!(msg, WM_MOUSEWHEEL | WM_MOUSEHWHEEL));
            assert_eq!(
                ((wp.0 >> 16) as u16) as i16,
                if forward { expected } else { -expected }
            );
            assert_eq!(wp.0 & 0xffff, 0);
            assert_eq!((lp.0 as u16) as i16, -1900);
            assert_eq!(((lp.0 >> 16) as u16) as i16, -50);
        }
    }
    for region in [
        Region {
            left: 40000,
            top: 0,
            width: 10,
            height: 10,
        },
        Region {
            left: i32::MAX,
            top: 0,
            width: 2,
            height: 2,
        },
        Region {
            left: 0,
            top: 0,
            width: 0,
            height: 2,
        },
    ] {
        assert!(center(region).is_err());
    }
}
#[test]
fn changed_identity_same_thread_and_cancellation_never_send() {
    let initial = Identity {
        window: 1,
        root: 2,
        process: 3,
        thread: 4,
        bounds: (0, 0, 100, 100),
        child_bounds: (0, 0, 100, 100),
        dpi: 96,
    };
    let target = Target {
        identity: initial,
        point: (50, 50),
    };
    let count = std::cell::Cell::new(0);
    assert!(
        send_checked(&target, initial, 5, &AtomicBool::new(false), || {
            count.set(count.get() + 1);
            Ok(())
        })
        .is_ok()
    );
    assert_eq!(count.get(), 1);
    for changed in [
        Identity {
            window: 9,
            ..initial
        },
        Identity { root: 9, ..initial },
        Identity {
            dpi: 144,
            ..initial
        },
        Identity {
            process: 9,
            ..initial
        },
        Identity {
            thread: 9,
            ..initial
        },
        Identity {
            bounds: (1, 0, 101, 100),
            ..initial
        },
        Identity {
            child_bounds: (1, 1, 99, 99),
            ..initial
        },
    ] {
        assert!(
            send_checked(&target, changed, 5, &AtomicBool::new(false), || panic!(
                "unexpected delivery"
            ))
            .is_err()
        );
    }
    assert!(
        send_checked(&target, initial, 4, &AtomicBool::new(false), || panic!(
            "same-thread delivery"
        ))
        .is_err()
    );
    assert!(
        send_checked(&target, initial, 5, &AtomicBool::new(true), || panic!(
            "cancelled delivery"
        ))
        .is_err()
    );
    let cancel = AtomicBool::new(false);
    assert!(send_checked(&target, initial, 5, &cancel, || {
        cancel.store(true, Ordering::Release);
        Ok(())
    })
    .is_err());
    assert!(
        send_checked(&target, initial, 5, &AtomicBool::new(false), || Err(
            "injected timeout".into()
        ))
        .is_err()
    );
}
