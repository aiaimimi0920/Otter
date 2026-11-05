//! 真实 HWND 激活 + 合成完整结果证据，验证编辑前缀、依赖点击、后续输入顺序。
use super::*;
#[test]
fn surface_edit_dependency_handlerless_prefix_releases_click_in_same_dispatch() {
    let _com = Apartment::new();
    let mut grant = grant();
    let mut window = active_window(&mut grant);
    let mut update = completed(&mut window);
    update.convergence.as_mut().unwrap().as_mut().unwrap().scene = Scene::parse(br#"{"id":"root","type":"column","children":[{"id":"button","type":"button","layout":{"width":180,"height":50},"props":{"includeSurfaceValues":true},"events":{"click":"submit"}},{"id":"field","type":"input","layout":{"width":180,"height":32}}]}"#).unwrap();
    window
        .accept_event(update, 0)
        .unwrap_or_else(|_| panic!("owner rejected"));
    window.sync_input(true, true, 0).unwrap();
    type_text(field(&window), "local");
    click(&window);
    let (_, wire) = take(&mut window);
    assert_eq!(wire["event"], "click");
    assert_eq!(wire["payload"]["surfaceValues"]["field"], "local");
}
fn scene(value: &str, action: &str) -> Scene {
    Scene::parse(serde_json::json!({"id":"root","type":"column","children":[
        {"id":"button","type":"button","layout":{"width":180,"height":50},"props":{"label":"Submit","includeSurfaceValues":true},"events":{"click":action}},
        {"id":"field","type":"input","layout":{"width":180,"height":32},"props":{"value":value},"events":{"input":"typing"}}
    ]}).to_string().as_bytes()).unwrap()
}
fn install_form(window: &mut AuthorizedWindow) {
    let mut update = completed(window);
    update.convergence.as_mut().unwrap().as_mut().unwrap().scene = scene("source", "submit");
    let receipt = window
        .accept_event(update, 0)
        .unwrap_or_else(|_| panic!("initial owner rejected"));
    assert!(matches!(receipt.presentation, Ok(Some(2))));
    window.sync_input(true, true, 0).unwrap();
}
fn click(window: &AuthorizedWindow) {
    unsafe {
        let point = LPARAM(10 | (10 << 16));
        SendMessageW(window.edit_test_parent(), WM_LBUTTONDOWN, None, Some(point));
        SendMessageW(window.edit_test_parent(), WM_LBUTTONUP, None, Some(point));
    }
}
fn take(window: &mut AuthorizedWindow) -> (Input, serde_json::Value) {
    let events = window.take_prepared_click(0).unwrap().unwrap();
    assert_eq!(events.len(), 1);
    let wire = serde_json::from_slice(events[0].bytes()).unwrap();
    let revision = window.snapshot_revision();
    let owner = window.begin_batch(&events, 0).unwrap().unwrap();
    (
        Input {
            events,
            revision,
            owner,
        },
        wire,
    )
}
fn result(input: Input, value: &str, action: &str) -> Outcome {
    let mut outcome = success(input, value);
    outcome
        .observation
        .as_mut()
        .unwrap()
        .as_mut()
        .unwrap()
        .snapshot
        .as_mut()
        .unwrap()
        .scene = scene(value, action);
    outcome
}
#[test]
fn surface_edit_dependency_click_waits_for_verified_prefix_but_not_post_click_input() {
    let _com = Apartment::new();
    for keyboard in [false, true] {
        let mut grant = grant();
        let mut window = active_window(&mut grant);
        install_form(&mut window);
        type_text(field(&window), "at click");
        if keyboard {
            use windows::Win32::UI::Input::KeyboardAndMouse::{SetFocus, VK_RETURN, VK_TAB};
            unsafe {
                SetFocus(Some(window.edit_test_parent())).unwrap();
                SendMessageW(
                    window.edit_test_parent(),
                    WM_KEYDOWN,
                    Some(WPARAM(VK_TAB.0 as usize)),
                    None,
                );
                SendMessageW(
                    window.edit_test_parent(),
                    WM_KEYDOWN,
                    Some(WPARAM(VK_RETURN.0 as usize)),
                    None,
                );
            }
        } else {
            click(&window);
        }
        type_text(field(&window), "after click");
        let (first, wire) = take(&mut window);
        assert_eq!(wire["event"], "input");
        // 仅在失败时读取诊断，不改变激活/取队列顺序或补发点击。
        assert_eq!(
            wire["payload"]["value"],
            "at click",
            "keyboard={keyboard}, parent={:?}, focus={:?}, foreground={:?}",
            window.edit_test_parent(),
            unsafe { windows::Win32::UI::Input::KeyboardAndMouse::GetFocus() },
            unsafe { GetForegroundWindow() }
        );
        assert!(window.take_prepared_click(0).unwrap().is_none());
        let receipt = window
            .accept_batch(result(first, "accepted first", "submit"), 0)
            .unwrap_or_else(|_| panic!("owner rejected"));
        assert!(matches!(receipt.presentation, Ok(Some(3))));
        window.sync_input(true, true, 0).unwrap();
        let (button, wire) = take(&mut window);
        assert_eq!(wire["event"], "click");
        assert_eq!(wire["baseRevision"], 3);
        assert_eq!(wire["payload"]["surfaceValues"]["field"], "at click");
        window
            .accept_batch(result(button, "button accepted", "submit"), 0)
            .unwrap_or_else(|_| panic!("owner rejected"));
        window.sync_input(true, true, 0).unwrap();
        let (_, wire) = take(&mut window);
        assert_eq!(wire["event"], "input");
        assert_eq!(wire["payload"]["value"], "after click");
        assert_eq!(wire["baseRevision"], 4);
    }
}
#[test]
fn surface_edit_dependency_click_without_prefix_precedes_later_input() {
    let _com = Apartment::new();
    let mut grant = grant();
    let mut window = active_window(&mut grant);
    install_form(&mut window);
    click(&window);
    type_text(field(&window), "after click");
    let (button, wire) = take(&mut window);
    assert_eq!(wire["event"], "click");
    assert_eq!(wire["payload"]["surfaceValues"]["field"], "source");
    window
        .accept_batch(result(button, "accepted", "submit"), 0)
        .unwrap_or_else(|_| panic!("owner rejected"));
    window.sync_input(true, true, 0).unwrap();
    let (_, wire) = take(&mut window);
    assert_eq!(wire["event"], "input");
    assert_eq!(wire["payload"]["value"], "after click");
    assert_eq!(wire["baseRevision"], 3);
}
#[test]
fn surface_edit_dependency_failure_or_changed_button_never_rebinds_old_click() {
    let _com = Apartment::new();
    for failed in [false, true] {
        let mut grant = grant();
        let mut window = active_window(&mut grant);
        install_form(&mut window);
        type_text(field(&window), "at click");
        click(&window);
        let (first, _) = take(&mut window);
        let mut outcome = result(
            first,
            "new authority",
            if failed { "submit" } else { "different" },
        );
        if failed {
            outcome
                .submission
                .as_mut()
                .unwrap()
                .acknowledgements
                .clear();
        }
        window
            .accept_batch(outcome, 0)
            .unwrap_or_else(|_| panic!("owner rejected"));
        window.sync_input(true, true, 0).unwrap();
        assert!(window.take_prepared_click(0).is_err());
        assert!(window.take_prepared_click(0).unwrap().is_none());
    }
}
