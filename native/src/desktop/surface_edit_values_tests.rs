//! 真实 EDIT 草稿进入鼠标/键盘按钮 wire；不发网络请求。
use super::*;
#[test]
fn surface_edit_dependency_deferred_click_expiry_is_monotonic_and_bounded() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window =
        SurfaceWindow::new(scene("source", serde_json::json!({}), true), 240, 160).unwrap();
    window.bind_edits(&binding()).unwrap();
    window.set_input_policy(true, true).unwrap();
    unsafe {
        let point = LPARAM(10 | (10 << 16));
        SendMessageW(window.hwnd, WM_LBUTTONDOWN, None, Some(point));
        SendMessageW(window.hwnd, WM_LBUTTONUP, None, Some(point));
    }
    let click = window.take_click().unwrap();
    let deadline = click.captured_at + std::time::Duration::from_secs(30);
    let pending = window.defer_click(click, &binding(), 1).unwrap();
    use crate::desktop::surface_edit_mount::FenceState;
    assert_eq!(window.deferred_state(&pending), FenceState::Ready);
    assert_eq!(
        window.deferred_state_at(&pending, deadline - std::time::Duration::from_nanos(1)),
        FenceState::Ready
    );
    assert_eq!(
        window.deferred_state_at(&pending, deadline),
        FenceState::Invalid
    );
}
fn scene(value: &str, payload: serde_json::Value, include: bool) -> Scene {
    Scene::parse(serde_json::json!({"id":"root","type":"view","children":[
        {"id":"submit","type":"button","layout":{"width":180,"height":50},
         "props":{"label":"Submit","includeSurfaceValues":include,"eventPayload":payload},"events":{"click":"submit"}},
        {"id":"field","type":"input","layout":{"width":180,"height":32},
         "props":{"value":value},"events":{"input":"typing"}}
    ]}).to_string().as_bytes()).unwrap()
}
fn payload(window: &SurfaceWindow, keyboard: bool) -> serde_json::Value {
    let events = if keyboard {
        let mut events = Vec::new();
        window
            .state
            .run(|host| {
                events = host.keyboard_events("submit", &binding(), 1, Default::default())?;
                Ok(())
            })
            .unwrap();
        events
    } else {
        window
            .click_events([10, 10], &binding(), 1, Default::default())
            .unwrap()
    };
    assert_eq!(events.len(), 1);
    serde_json::from_slice::<serde_json::Value>(events[0].bytes()).unwrap()["payload"].clone()
}
#[test]
fn surface_values_real_edit_reaches_pointer_and_keyboard_and_verified_settlement() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = SurfaceWindow::new(
        scene(
            "source",
            serde_json::json!({"tag":7,"surfaceValues":"old"}),
            true,
        ),
        240,
        160,
    )
    .unwrap();
    window.bind_edits(&binding()).unwrap();
    window.set_input_policy(true, true).unwrap();
    assert_eq!(payload(&window, false)["surfaceValues"]["field"], "source");
    type_text(child(&window), "中文🙂");
    for keyboard in [false, true] {
        assert_eq!(
            payload(&window, keyboard),
            serde_json::json!({"tag":7,"surfaceValues":{"field":"中文🙂"}})
        );
    }
    let events = window.prepare_edit(&binding(), 1).unwrap();
    window
        .replace(scene("accepted", serde_json::json!({}), true))
        .unwrap();
    assert_eq!(payload(&window, false)["surfaceValues"]["field"], "中文🙂");
    window.settle_edits(&events).unwrap();
    assert_eq!(payload(&window, true)["surfaceValues"]["field"], "accepted");
    assert!(window.take_error().is_none());
}
#[test]
fn surface_values_opt_in_wraps_primitives_but_preserves_opt_out_payload() {
    let _com = crate::desktop::Apartment::new().unwrap();
    for configured in [
        serde_json::json!(false),
        serde_json::json!([1, 2]),
        serde_json::json!("text"),
    ] {
        for include in [false, true] {
            let window =
                SurfaceWindow::new(scene("source", configured.clone(), include), 240, 160).unwrap();
            window.bind_edits(&binding()).unwrap();
            let expected = if include {
                serde_json::json!({"configured":configured,"surfaceValues":{"field":"source"}})
            } else {
                configured.clone()
            };
            assert_eq!(payload(&window, false), expected);
        }
    }
}
#[test]
fn surface_values_escaped_wire_still_obeys_existing_event_budget() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = SurfaceWindow::new(
        scene(&"\u{1}".repeat(16 * 1024), serde_json::json!({}), true),
        240,
        160,
    )
    .unwrap();
    window.bind_edits(&binding()).unwrap();
    assert!(window
        .click_events([10, 10], &binding(), 1, Default::default())
        .is_err());
}
#[test]
fn surface_values_candidate_freezes_activation_value_for_mouse_and_keyboard() {
    let _com = crate::desktop::Apartment::new().unwrap();
    for keyboard in [false, true] {
        let window =
            SurfaceWindow::new(scene("source", serde_json::json!({}), true), 240, 160).unwrap();
        window.bind_edits(&binding()).unwrap();
        window.set_input_policy(true, true).unwrap();
        type_text(child(&window), "at activation");
        unsafe {
            if keyboard {
                let _ = SetFocus(Some(window.hwnd));
                assert_eq!(
                    GetFocus(),
                    window.hwnd,
                    "values keyboard fixture lost focus"
                );
                SendMessageW(
                    window.hwnd,
                    WM_KEYDOWN,
                    Some(WPARAM(VK_TAB.0 as usize)),
                    None,
                );
                assert_eq!(
                    window.state.keyboard.borrow().focused(),
                    Some("submit"),
                    "values Tab did not select submit: focus={:?} modifiers={:?}",
                    GetFocus(),
                    pointer::modifiers()
                );
                SendMessageW(
                    window.hwnd,
                    WM_KEYDOWN,
                    Some(WPARAM(VK_RETURN.0 as usize)),
                    None,
                );
            } else {
                let point = LPARAM(10 | (10 << 16));
                SendMessageW(window.hwnd, WM_LBUTTONDOWN, None, Some(point));
                assert_eq!(
                    GetCapture(),
                    window.hwnd,
                    "values pointer fixture lost capture"
                );
                SendMessageW(window.hwnd, WM_LBUTTONUP, None, Some(point));
            }
        }
        let candidate = window
            .take_click()
            .unwrap_or_else(|| unsafe {
                panic!("activation must queue a candidate: keyboard={keyboard} focus={:?} foreground={:?} parent={:?} capture={:?} logical={:?} modifiers={:?} error={:?}", GetFocus(), GetForegroundWindow(), window.hwnd, GetCapture(), window.state.keyboard.borrow().focused(), pointer::modifiers(), window.take_error())
            });
        type_text(child(&window), "later input");
        let preceding = window.prepare_edit(&binding(), 1).unwrap();
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(preceding[0].bytes()).unwrap()["payload"]
                ["value"],
            "at activation"
        );
        let events = window
            .click_events_for_candidate(candidate, &binding(), 1)
            .unwrap();
        let wire: serde_json::Value = serde_json::from_slice(events[0].bytes()).unwrap();
        assert_eq!(wire["payload"]["surfaceValues"]["field"], "at activation");
        assert_eq!(
            payload(&window, false)["surfaceValues"]["field"],
            "later input"
        );
        assert!(window.take_error().is_none());
    }
}
