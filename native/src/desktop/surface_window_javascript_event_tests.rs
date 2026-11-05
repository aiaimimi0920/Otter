//! 自有 HWND 的定向点击经原候选验证进入事件阶段；不注入全局输入或授予远端动作。
use super::*;
#[test]
fn javascript_event_target_real_hwnd_click_phases_pixels_and_same_child_update() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let _com = SurfaceWindow::javascript_apartment_for_test();
    let binding = fixture::binding();
    let (mut session, frame) = Session::mount(
        binding.clone(),
        include_str!("../../tests/fixtures/javascript-batch-events.js").into(),
        fixture::snapshot(3, 42),
        fixture::resources(),
        &|| false,
    )
    .unwrap();
    let button = frame.scene.root().children()[1].children()[0]
        .id()
        .to_owned();
    let window = SurfaceWindow::new(frame.scene, 320, 200).unwrap();
    window.set_pointer_enabled(true).unwrap();
    assert_eq!(window.javascript_text_for_test(), "原生🙂:42:0");
    let before = css_tests::pixels(&window);
    let mut node = window.accessibility_node(&button).unwrap();
    assert!(node.read().is_some());
    for round in 0..2 {
        window.javascript_queue_click_for_test(&button);
        let candidate = window.take_click().unwrap();
        let target = candidate.node_id.clone();
        assert_eq!(target, button);
        assert!(window
            .click_events_for_candidate(candidate, &binding, round + 3)
            .unwrap()
            .is_empty());
        let frame = session.click(&target, &|| false).unwrap();
        assert_eq!(frame.events.len(), 2);
        window.replace(frame.scene).unwrap();
        let quote = if round == 0 { 45 } else { 48 };
        let frame = session
            .update(
                fixture::snapshot(round + 4, quote),
                fixture::resources(),
                &|| false,
            )
            .unwrap();
        window.replace(frame.scene).unwrap();
        assert_eq!(
            window.javascript_text_for_test(),
            format!("原生🙂:{quote}:{}", round + 1)
        );
        // replace 撤销旧 UIA 身份；DOM/child identity 不等于旧 provider 句柄永久有效。
        assert!(node.read().is_none());
        node = window.accessibility_node(&button).unwrap();
        assert!(node.read().is_some());
    }
    assert_ne!(before, css_tests::pixels(&window));
    assert!(window.take_error().is_none());
    session.dispose(&|| false).unwrap();
    window.close().unwrap();
    assert!(!window.is_alive() && node.read().is_none());
}
