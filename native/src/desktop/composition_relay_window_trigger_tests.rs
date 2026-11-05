//! 自有隐藏Viewer的菜单准入/同一配置提交接缝/关闭围栏；不执行网络或人工菜单导航。
use super::*;
use crate::live_relay_control_runtime::interaction::Queue;
#[test]
fn actual_viewer_trigger_menu_requires_real_surface_and_closing_clears_pending() {
    let _apartment = Apartment::new().unwrap();
    let (media, permit) = media();
    let window = RelayWindow::new(media.clone(), permit, 128, 96).unwrap();
    let queue = Arc::new(Queue::default());
    window.bind_input(queue.clone());
    unsafe {
        let menu = GetSystemMenu(window.hwnd, false);
        assert_ne!(
            GetMenuState(menu, trigger::OPEN as u32, MF_BYCOMMAND) & MF_GRAYED.0,
            0
        );
        queue
            .triggers
            .configure(Some(("surface:real".into(), "attachment:real".into())));
        window.refresh().unwrap();
        assert_eq!(
            GetMenuState(menu, trigger::OPEN as u32, MF_BYCOMMAND) & MF_GRAYED.0,
            0
        );
        let text = r#"{"bindingId":"b","enabled":true,"target":{"surfaceInstanceId":"surface:real","surfaceAttachmentId":"attachment:real","surfaceNodeId":"n","surfaceEvent":"click","surfaceAction":"x"},"condition":{"conditionId":"c","revision":1,"observationId":"o","operator":"equals","operand":true,"stableForMs":0,"risingEdge":false,"rearm":false,"minimumConfidence":"exact"}}"#;
        assert!(trigger::submit_test(window.hwnd, text));
        assert_eq!(
            trigger::phase_test(window.hwnd),
            Some(crate::live_relay_viewer_trigger_queue::Phase::Pending)
        );
        assert_ne!(
            GetMenuState(menu, trigger::OPEN as u32, MF_BYCOMMAND) & MF_GRAYED.0,
            0
        );
        assert!(!trigger::submit_test(window.hwnd, text));
        assert_eq!(
            trigger::phase_test(window.hwnd),
            Some(crate::live_relay_viewer_trigger_queue::Phase::Pending)
        );
    }
    window.close().unwrap();
    assert!(queue.triggers.take().is_none());
    assert!(!queue
        .triggers
        .complete(crate::live_relay_viewer_trigger_queue::Phase::Confirmed));
}
