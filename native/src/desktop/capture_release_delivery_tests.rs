//! 松手采集次序和取消合同；不捕获用户桌面。
use super::*;
use std::cell::Cell;

#[test]
fn final_pixels_are_sampled_after_selection_not_from_shortcut_time() {
    let color = Cell::new(10u8);
    let result = acquire_with(
        || {
            assert_eq!(color.get(), 10);
            color.set(200);
            Ok(Some(RECT {
                left: -40,
                top: 20,
                right: -36,
                bottom: 23,
            }))
        },
        |screen| {
            assert_eq!(screen.left, -40);
            assert_eq!(
                (screen.right - screen.left, screen.bottom - screen.top),
                (4, 3)
            );
            Ok(Some(color.get()))
        },
    )
    .unwrap();
    assert_eq!(result, Some(200));
}

#[test]
fn cancellation_and_selector_failure_never_start_capture() {
    assert!(acquire_with(
        || Ok(None),
        |_| -> Result<Option<()>> { panic!("Cancelled selection must not capture") }
    )
    .unwrap()
    .is_none());
    let error = acquire_with(
        || Err(Error::new(E_ABORT, "Selection interrupted")),
        |_| -> Result<Option<()>> { panic!("Failed selection must not capture") },
    )
    .unwrap_err();
    assert_eq!(error.code(), E_ABORT);
}
