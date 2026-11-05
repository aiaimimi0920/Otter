//! 合并/确认/失败/销毁与非几何变更；真实 HWND 和保存链由 checkpoint geometry tests 保护。
use super::*;

#[test]
fn position_or_size_changes_coalesce_until_acknowledged() {
    let wake = Wake::default();
    let count = Cell::new(0);
    let send = || {
        count.set(count.get() + 1);
        true
    };
    wake.request(SWP_NOSIZE, send);
    for _ in 0..1000 {
        wake.request(SWP_NOMOVE, || panic!("duplicate geometry wake"));
    }
    assert_eq!(count.get(), 1);
    assert_eq!(
        unsafe { dispatch(HWND::default(), &wake, MESSAGE, LPARAM(0)) },
        Some(LRESULT(0))
    );
    wake.request(SWP_NOMOVE, send);
    assert_eq!(count.get(), 2);
    assert!(wake.pending.get());
}

#[test]
fn failed_post_does_not_latch_and_destroy_disables_future_posts() {
    for message in [WM_DESTROY, WM_NCDESTROY] {
        let wake = Wake::default();
        wake.request(SWP_NOSIZE, || false);
        assert!(!wake.pending.get());
        wake.request(SWP_NOSIZE, || true);
        assert!(wake.pending.get());
        assert_eq!(
            unsafe { dispatch(HWND::default(), &wake, message, LPARAM(0)) },
            None
        );
        assert!(!wake.active.get());
        assert!(!wake.pending.get());
        unsafe { dispatch(HWND::default(), &wake, MESSAGE, LPARAM(0)) };
        wake.request(SWP_NOSIZE, || panic!("destroyed window wake"));
    }
}

#[test]
fn z_order_visibility_and_other_non_geometry_messages_do_not_notify() {
    let wake = Wake::default();
    for flags in [
        SWP_NOZORDER,
        SWP_SHOWWINDOW,
        SWP_HIDEWINDOW,
        SWP_FRAMECHANGED,
    ] {
        let mut position = WINDOWPOS {
            flags: flags | SWP_NOMOVE | SWP_NOSIZE,
            ..Default::default()
        };
        assert_eq!(
            unsafe {
                dispatch(
                    HWND::default(),
                    &wake,
                    WM_WINDOWPOSCHANGED,
                    LPARAM(&mut position as *mut _ as isize),
                )
            },
            None
        );
        assert!(!wake.pending.get());
    }
    assert_eq!(
        unsafe { dispatch(HWND::default(), &wake, WM_MOVE, LPARAM(0)) },
        None
    );
    assert!(!wake.pending.get());
}
