//! 真实自有隐藏管理窗：同一消息泵内排队命令后关闭，不消费已销毁owner的启动意图。
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
