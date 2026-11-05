use super::*;
fn bounds() -> RECT {
    RECT {
        left: -1920,
        top: -1080,
        right: 0,
        bottom: 0,
    }
}
fn target() -> Target {
    let screen = RECT {
        left: -1900,
        top: -1000,
        right: -1800,
        bottom: -900,
    };
    Target {
        screen,
        identity: Identity {
            monitor: HMONITOR(std::ptr::dangling_mut()),
            bounds: bounds(),
            name: [1; 32],
        },
        region: local(screen, bounds()).unwrap(),
    }
}
#[test]
fn negative_physical_coordinates_and_exact_edges_do_not_apply_dpi_scaling() {
    let target = target();
    assert_eq!(
        target.region(),
        Region {
            left: 20,
            top: 80,
            width: 100,
            height: 100
        }
    );
    assert_eq!(
        local(bounds(), bounds()).unwrap(),
        Region {
            left: 0,
            top: 0,
            width: 1920,
            height: 1080
        }
    );
    for screen in [
        RECT {
            right: 1,
            ..bounds()
        },
        RECT {
            left: -1921,
            ..bounds()
        },
        RECT {
            top: -1081,
            ..bounds()
        },
        RECT {
            bottom: 1,
            ..bounds()
        },
    ] {
        assert!(matches!(local(screen, bounds()), Err(Error::CrossMonitor)));
    }
    for screen in [
        RECT::default(),
        RECT {
            left: 1,
            right: 0,
            top: 0,
            bottom: 1,
        },
        RECT {
            left: i32::MIN,
            right: i32::MAX,
            top: 0,
            bottom: 1,
        },
    ] {
        assert!(matches!(Target::resolve(screen), Err(Error::Dimensions)));
    }
}
#[test]
fn identity_change_before_capture_blocks_it_and_after_capture_drops_result() {
    let target = target();
    let cancel = AtomicBool::new(false);
    let mut changed = target.identity;
    changed.bounds.left -= 1;
    assert!(matches!(
        target.run(
            &cancel,
            || Ok(changed),
            || panic!("must not capture changed target")
        ),
        Err::<(), _>(Error::Changed)
    ));
    let count = std::cell::Cell::new(0);
    struct Output<'a>(&'a std::cell::Cell<u32>);
    impl Drop for Output<'_> {
        fn drop(&mut self) {
            self.0.set(self.0.get() + 1);
        }
    }
    let mut calls = 0;
    let result = target.run(
        &cancel,
        || {
            calls += 1;
            Ok(if calls == 1 { target.identity } else { changed })
        },
        || Ok(Output(&count)),
    );
    assert!(matches!(result, Err(Error::Changed)));
    assert_eq!(count.get(), 1);
    assert_eq!(calls, 2);
    let mut renamed = target.identity;
    renamed.name[0] = 2;
    assert!(matches!(
        target.run(&cancel, || Ok(renamed), || Ok(())),
        Err(Error::Changed)
    ));
}
#[test]
fn cancelled_or_failed_capture_never_revalidates_or_publishes() {
    let target = target();
    let cancel = AtomicBool::new(true);
    assert!(matches!(
        target.run(&cancel, || panic!("no lookup"), || Ok(())),
        Err(Error::Cancelled)
    ));
    cancel.store(false, Ordering::Release);
    let calls = std::cell::Cell::new(0);
    let lookup = || {
        calls.set(calls.get() + 1);
        Ok(target.identity)
    };
    assert!(matches!(
        target.run(&cancel, lookup, || {
            cancel.store(true, Ordering::Release);
            Ok(())
        }),
        Err(Error::Cancelled)
    ));
    assert_eq!(calls.get(), 1);
    cancel.store(false, Ordering::Release);
    assert!(matches!(
        target.run(&cancel, lookup, || Err::<(), _>(Error::NoMonitor)),
        Err(Error::NoMonitor)
    ));
}
