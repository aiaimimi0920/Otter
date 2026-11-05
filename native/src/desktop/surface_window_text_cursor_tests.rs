//! 只查自有窗口的正式命中决策，不移动系统指针或改写用户光标。
use super::*;
fn window(json: &[u8]) -> SurfaceWindow {
    let window = SurfaceWindow::new(Scene::parse(json).unwrap(), 240, 160).unwrap();
    window.set_pointer_enabled(true).unwrap();
    window
}
#[test]
fn selectable_text_cursor_respects_text_hit_and_local_policy_without_changing_selection() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = window(br#"{"id":"text","type":"text","props":{"text":"Text","selectable":true},"layout":{"width":200,"height":40}}"#);
    assert_eq!(at(&window.state, [1, 8]).unwrap(), Some(Cursor::Text));
    assert_eq!(at(&window.state, [190, 8]).unwrap(), None);
    assert!(window
        .state
        .host
        .borrow()
        .as_ref()
        .unwrap()
        .presentation()
        .selected_text()
        .unwrap()
        .is_none());
    window.set_submission_enabled(false).unwrap();
    assert_eq!(at(&window.state, [1, 8]).unwrap(), Some(Cursor::Text));
    window.set_input_policy(false, false).unwrap();
    assert_eq!(at(&window.state, [1, 8]).unwrap(), None);
}
#[test]
fn selectable_text_cursor_does_not_override_covering_button_or_nonselectable_text() {
    let _com = crate::desktop::Apartment::new().unwrap();
    for disabled in [false, true] {
        let json = format!(
            r#"{{"id":"root","type":"stack","layout":{{"position":"relative","width":200,"height":100}},"children":[{{"id":"text","type":"text","props":{{"text":"Text","selectable":true}},"layout":{{"width":200,"height":40}}}},{{"id":"cover","type":"button","props":{{"text":"Button","disabled":{disabled}}},"layout":{{"position":"absolute","left":0,"top":0,"width":200,"height":40}}}}]}}"#
        );
        let window = window(json.as_bytes());
        assert_eq!(
            at(&window.state, [1, 8]).unwrap(),
            Some(if disabled {
                Cursor::Forbidden
            } else {
                Cursor::Hand
            })
        );
    }
    let window = window(br#"{"id":"text","type":"text","props":{"text":"Text"},"layout":{"width":200,"height":40}}"#);
    assert_eq!(at(&window.state, [1, 8]).unwrap(), None);
}
