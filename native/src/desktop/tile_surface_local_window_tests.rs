//! 自有离屏HWND真实通知；无handler成功以本地状态而非事件入队判定，不操作用户程序。
use super::*;
use crate::tile_surface_actions::tests::Fixture;
use serde_json::{json, Value as Json};
fn setup(scene: Json) -> (Fixture, Window) {
    let mut f = crate::tile_surface_value::tests::fixture(scene);
    let frame = Arc::new(Frame {
        key: f.frame.key.clone(),
        layout: f.frame.layout.clone(),
        size: Size {
            width: 64,
            height: 64,
        },
        images: f.frame.images.clone(),
        arts: f.frame.arts.clone(),
        live_until: None,
    });
    f.job.size = frame.size;
    f.shared.lock().frame = Some(frame.clone());
    f.shared.lock().presented_frame = None;
    f.frame = frame;
    let mut output = crate::tile_tests::output();
    (output.x, output.y, output.width, output.height) = (-30000, -30000, 64, 64);
    let w = Window::new(f.shared.clone(), &output).unwrap();
    w.tick().unwrap();
    w.tick().unwrap();
    (f, w)
}
fn scene(kind: &str) -> Json {
    json!({"id":"field","type":kind,"layout":{"width":64,"height":64},"props":{
        "value": if kind=="switch" {json!(false)} else if kind=="select" {json!("a")} else {json!(2)},
        "min":0,"max":10,"step":2,"options":[{"value":"a","label":"A"},{"value":"b","label":"B"}]}})
}
fn displayed(w: &Window) -> Value {
    value(&w.state, &focus(&w.state, [10, 10]).unwrap()).unwrap()
}
fn child(w: &Window) -> HWND {
    w.state
        .keyboard_value
        .borrow()
        .as_ref()
        .unwrap()
        .child()
        .unwrap()
}
fn keyboard_open(w: &Window) {
    unsafe {
        let _ = windows::Win32::UI::Input::KeyboardAndMouse::SetFocus(Some(w.hwnd));
    }
    keyboard::navigate(w.hwnd, &w.state, false);
}
unsafe fn key(h: HWND, k: usize) {
    SendMessageW(h, WM_KEYDOWN, Some(WPARAM(k)), None);
}
#[test]
fn local_switch_mouse_and_keyboard_persist_without_queue_and_preserve_ancestor_click() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let foreground = unsafe { GetForegroundWindow() };
    let (f, w) = setup(scene("switch"));
    assert!(!w.test_art_click([10, 10]));
    assert_eq!(displayed(&w), Value::Switch(true));
    w.tick().unwrap();
    assert!(!w.test_art_click([10, 10]));
    assert_eq!(displayed(&w), Value::Switch(false));
    keyboard_open(&w);
    unsafe {
        key(w.hwnd, 32);
        SendMessageW(w.hwnd, WM_KEYUP, Some(WPARAM(32)), None);
    }
    assert_eq!(displayed(&w), Value::Switch(true));
    assert!(f.shared.lock().actions.pending.is_empty());
    assert_eq!(unsafe { GetForegroundWindow() }, foreground);
    let (f, w) = setup(
        json!({"id":"root","type":"view","events":{"click":"outer"},"children":[scene("switch")]}),
    );
    assert!(w.test_art_click([10, 10]));
    assert_eq!(displayed(&w), Value::Switch(true));
    let s = f.shared.lock();
    assert_eq!(s.actions.pending.len(), 1);
    let crate::tile_surface_actions::Kind::Event(e) = &s.actions.pending[0].kind else {
        panic!()
    };
    assert_eq!(e["action"], "outer");
    assert_eq!(e["event"], "click");
}
#[test]
fn local_select_cancel_keeps_value_and_keyboard_reopen_uses_accepted_index() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let (f, w) = setup(scene("select"));
    assert!(!w.test_art_select([10, 10], Some(1)));
    assert_eq!(displayed(&w), Value::Select(Some(1)));
    assert!(!w.test_art_select([10, 10], None));
    assert_eq!(displayed(&w), Value::Select(Some(1)));
    keyboard_open(&w);
    assert_eq!(
        w.state
            .keyboard_value
            .borrow()
            .as_ref()
            .unwrap()
            .displayed_value(),
        json!("b")
    );
    unsafe {
        key(child(&w), 38);
    }
    assert_eq!(displayed(&w), Value::Select(Some(0)));
    keyboard::close_for_text(&w.state);
    keyboard_open(&w);
    assert_eq!(
        w.state
            .keyboard_value
            .borrow()
            .as_ref()
            .unwrap()
            .displayed_value(),
        json!("a")
    );
    assert!(f.shared.lock().actions.pending.is_empty());
}
#[test]
fn local_slider_drag_commits_cancel_restores_and_keyboard_reopen_retains_value() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let (f, w) = setup(scene("slider"));
    w.test_art_slider_message(WM_LBUTTONDOWN, [48, 32]);
    w.test_art_slider_message(WM_LBUTTONUP, [48, 32]);
    assert_eq!(displayed(&w), Value::Slider(8.));
    w.tick().unwrap();
    w.tick().unwrap();
    w.test_art_slider_message(WM_LBUTTONDOWN, [5, 32]);
    unsafe {
        SendMessageW(w.hwnd, WM_CANCELMODE, None, None);
    }
    assert_eq!(displayed(&w), Value::Slider(8.));
    keyboard_open(&w);
    assert_eq!(
        w.state
            .keyboard_value
            .borrow()
            .as_ref()
            .unwrap()
            .displayed_value(),
        json!(8.)
    );
    unsafe {
        key(child(&w), 37);
    }
    assert_eq!(displayed(&w), Value::Slider(6.));
    keyboard::close_for_text(&w.state);
    keyboard_open(&w);
    assert_eq!(
        w.state
            .keyboard_value
            .borrow()
            .as_ref()
            .unwrap()
            .displayed_value(),
        json!(6.)
    );
    assert!(f.shared.lock().actions.pending.is_empty());
}
#[test]
fn preparing_gap_rejects_new_native_values_and_freeze_black_hide_stop_clear_cache() {
    let _com = crate::desktop::Apartment::new().unwrap();
    for case in 0..4 {
        let (f, w) = setup(scene("slider"));
        keyboard_open(&w);
        let h = child(&w);
        unsafe {
            key(h, 39);
        }
        assert_eq!(displayed(&w), Value::Slider(4.));
        f.shared.lock().presented_frame = None;
        unsafe {
            key(h, 39);
        }
        assert_eq!(
            w.state
                .keyboard_value
                .borrow()
                .as_ref()
                .unwrap()
                .displayed_value(),
            json!(4.)
        );
        assert_eq!(
            w.state.local.borrow().snapshot()[0].value,
            Value::Slider(4.)
        );
        match case {
            0 => f.shared.lock().frozen = true,
            1 => f.shared.lock().black = true,
            2 => unsafe {
                let _ = ShowWindow(w.hwnd, SW_HIDE);
            },
            _ => f.shared.stop(),
        }
        w.tick().unwrap();
        assert!(w.state.local.borrow().snapshot().is_empty());
        assert!(f.shared.lock().actions.pending.is_empty());
        unsafe {
            key(h, 39);
        }
        assert!(w.state.local.borrow().snapshot().is_empty());
    }
}
#[test]
fn local_values_do_not_enter_text_form_and_real_button_queues_only_current_text() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let mut children = vec![json!({"id":"text","type":"input","props":{"value":"source"}})];
    for kind in ["switch", "select", "slider"] {
        let mut n = scene(kind);
        n["id"] = kind.into();
        children.push(n);
    }
    for n in &mut children {
        n["layout"] = json!({"width":64,"height":10,"minHeight":10});
    }
    children.push(
        json!({"id":"send","type":"button","layout":{"width":64,"height":24,"minHeight":24},
        "props":{"includeSurfaceValues":true},"events":{"click":"send_form"}}),
    );
    let (f, w) = setup(
        json!({"id":"root","type":"column","layout":{"width":64,"height":64},"children":children}),
    );
    w.test_art_text([5, 5], "本地🙂", false, true);
    assert!(!w.test_art_click([5, 15]));
    assert!(!w.test_art_select([5, 25], Some(1)));
    w.test_art_slider_message(WM_LBUTTONDOWN, [48, 35]);
    w.test_art_slider_message(WM_LBUTTONUP, [48, 35]);
    assert!(f.shared.lock().actions.pending.is_empty());
    assert_eq!(w.state.local.borrow().snapshot().len(), 3);
    assert!(w.test_art_click([5, 50]));
    let s = f.shared.lock();
    assert_eq!(s.actions.pending.len(), 1);
    let crate::tile_surface_actions::Kind::Event(e) = &s.actions.pending[0].kind else {
        panic!()
    };
    assert_eq!(e["class"], "discrete");
    assert_eq!(e["payload"], json!({"surfaceValues":{"text":"本地🙂"}}));
}
