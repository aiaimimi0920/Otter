//! 自有 Win32 模拟类门禁：验证消息链，不声称真实 CLR/WinForms 兼容验收。
use super::*;

#[test]
#[ignore = "interactive desktop; owned simulated button class, no global input"]
fn actual_occluded_button_notifies_once_but_drag_and_cleanup_do_not() {
    let (mut fixture, root, child) = fixture_with_button_class(true);
    let root_hwnd = root.hwnd().0 as usize;
    let mut controller = Controller::new(Backend::new(root, None).unwrap());
    controller.enable().unwrap();
    let edge = |down| Event::Button {
        point: [0.2; 2],
        button: Button::Left,
        down,
        double: false,
    };
    controller.dispatch(1, edge(true)).unwrap();
    controller.dispatch(2, edge(false)).unwrap();
    controller.dispatch(3, edge(true)).unwrap();
    controller
        .dispatch(4, Event::Move { point: [0.5; 2] })
        .unwrap();
    controller
        .dispatch(5, Event::Move { point: [0.2; 2] })
        .unwrap();
    controller.dispatch(6, edge(false)).unwrap();
    controller.dispatch(7, edge(true)).unwrap();
    assert_eq!(controller.cancel().released, 1);
    let records = fixture.finish();
    let notifications: Vec<_> = records
        .iter()
        .filter(|(_, msg, _, _)| *msg == WM_COMMAND)
        .collect();
    assert_eq!(notifications.len(), 1, "{records:?}");
    assert_eq!(notifications[0].0, root_hwnd);
    assert_eq!(notifications[0].3, child as isize);
    assert_eq!(
        records
            .iter()
            .filter(|(hwnd, msg, _, _)| *hwnd == child && *msg == WM_LBUTTONDOWN)
            .count(),
        3
    );
    assert_eq!(
        records
            .iter()
            .filter(|(hwnd, msg, _, _)| *hwnd == child && *msg == WM_LBUTTONUP)
            .count(),
        3
    );
}
