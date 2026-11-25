//! Actual owned hidden manager window: queue commands then close in one message pump; never consume startup intents from destroyed owners.
use super::*;
#[test]
fn queued_output_command_is_not_consumed_after_manager_close() {
    let output = crate::tile_tests::output();
    let window = unsafe { create(&[output]).unwrap() };
    assert_eq!(window.take_action(), Some(0));
    unsafe {
        assert!(!IsWindowVisible(window.hwnd).as_bool());
        PostMessageW(Some(window.hwnd), WM_COMMAND, WPARAM(100), LPARAM(0)).unwrap();
        PostMessageW(Some(window.hwnd), WM_CLOSE, WPARAM(0), LPARAM(0)).unwrap();
    }
    pump().unwrap();
    assert!(window.state.closed.get());
    assert_eq!(
        window.state.action.get(),
        100,
        "the command really reached this owner"
    );
    assert_eq!(
        window.take_action(),
        None,
        "closed owner cannot launch an output"
    );
    title(&window, "must not write the destroyed HWND");
    unsafe {
        assert!(!IsWindow(Some(window.hwnd)).as_bool());
    }
}
