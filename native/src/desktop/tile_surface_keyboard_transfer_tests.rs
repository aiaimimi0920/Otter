//! 焦点转移失败保原owner，IME/同步重入与键盘控件内鼠标拖动的自有HWND回归。
use super::*;
use windows::Win32::UI::{Input::KeyboardAndMouse::GetCapture, Shell::*};
fn value_scene(kind: &str) -> Value {
    json!({"id":"value","type":kind,"layout":{"width":64,"height":64},
        "props":{"value":if kind=="select" {json!("a")} else {json!(2)},
        "options":[{"label":"A","value":"a"},{"label":"B","value":"b"}],
        "min":0,"max":10,"step":2},"events":{"input":"preview","change":"save"}})
}
unsafe fn click(hwnd: HWND, pixel: [i32; 2]) {
    let point = LPARAM(pixel[0] as isize | ((pixel[1] as isize) << 16));
    SendMessageW(hwnd, WM_LBUTTONDOWN, None, Some(point));
    SendMessageW(hwnd, WM_LBUTTONUP, None, Some(point));
}
#[test]
fn composition_keeps_actual_and_logical_field_on_pointer_rejection_then_tab_moves_once() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let (f, w) = setup(
        json!({"id":"root","type":"column","layout":{"width":64,"height":64},"children":[
        {"id":"first","type":"input","layout":{"width":64,"height":20,"minHeight":20},"props":{"value":"a"}},
        {"id":"second","type":"input","layout":{"width":64,"height":20,"minHeight":20},"props":{"value":"b"}},
        {"id":"send","type":"button","layout":{"width":64,"height":24,"minHeight":24},"events":{"click":"go"}}]}),
    );
    navigate(w.hwnd, &w.state, false);
    let h = text::active_child(&w.state);
    unsafe {
        SendMessageW(h, WM_IME_STARTCOMPOSITION, None, None);
    }
    for (pixel, expected) in [([8, 28], "second"), ([8, 52], "send")] {
        let hit = actions::active(&w.state)
            .and_then(|frame| w.state.renderer.borrow().as_ref()?.focus_at(&frame, pixel))
            .unwrap();
        assert_eq!(hit.node, expected);
        unsafe {
            click(w.hwnd, pixel);
        }
        assert_eq!(focus(&w), "first");
        assert_eq!(text::active_child(&w.state), h);
        assert_eq!(unsafe { GetFocus() }, h);
        assert!(events(&f).is_empty());
    }
    unsafe {
        SendMessageW(h, WM_IME_ENDCOMPOSITION, None, None);
        key(h, WM_KEYDOWN, 9, false);
    }
    assert_eq!(focus(&w), "second");
    assert_ne!(text::active_child(&w.state), h);
    assert_eq!(unsafe { GetFocus() }, text::active_child(&w.state));
}
#[test]
fn navigation_search_error_preserves_select_slider_hwnds_focus_values_and_pending_intents() {
    let _com = crate::desktop::Apartment::new().unwrap();
    for kind in ["select", "slider"] {
        let (f, w) = setup(value_scene(kind));
        navigate(w.hwnd, &w.state, false);
        let h = child(&w);
        let popup = unsafe { GetParent(h).unwrap() };
        unsafe {
            key(h, WM_KEYDOWN, if kind == "select" { 40 } else { 39 }, false);
        }
        let before = events(&f);
        let value = w
            .state
            .keyboard_value
            .borrow()
            .as_ref()
            .unwrap()
            .displayed_value();
        super::super::transfer(
            w.hwnd,
            &w.state,
            Some(Err(Error::new(E_FAIL, "focus budget"))),
        );
        assert_eq!(
            f.shared.lock().actions.notice,
            "Art键盘可见区域搜索预算不足"
        );
        assert_eq!(focus(&w), "value");
        assert_eq!(child(&w), h);
        assert!(unsafe { IsWindow(Some(popup)) }.as_bool());
        assert_eq!(unsafe { GetFocus() }, h);
        assert_eq!(
            w.state
                .keyboard_value
                .borrow()
                .as_ref()
                .unwrap()
                .displayed_value(),
            value
        );
        assert_eq!(events(&f), before);
    }
}
#[test]
fn unchanged_slider_tick_keeps_capture_until_real_move_and_release_commit() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let (f, w) = setup(value_scene("slider"));
    navigate(w.hwnd, &w.state, false);
    let h = child(&w);
    // x=16落在声明值2的step区间，按下不产生preview。
    let point = |x: isize| LPARAM(x | (32isize << 16));
    unsafe {
        SendMessageW(h, WM_LBUTTONDOWN, None, Some(point(16)));
    }
    assert!(events(&f).is_empty());
    assert_eq!(unsafe { GetCapture() }, h);
    w.tick().unwrap();
    assert_eq!(unsafe { GetCapture() }, h);
    assert_eq!(child(&w), h);
    unsafe {
        SendMessageW(h, WM_MOUSEMOVE, None, Some(point(59)));
        SendMessageW(h, WM_LBUTTONUP, None, Some(point(59)));
    }
    assert_ne!(unsafe { GetCapture() }, h);
    let e = events(&f);
    assert_eq!(e.len(), 2);
    assert_eq!(e[0]["event"], "input");
    assert_eq!(e[1]["event"], "change");
    assert_eq!(e[1]["payload"]["value"], 10.);
}
unsafe extern "system" fn cancel_on_focus(
    hwnd: HWND,
    msg: u32,
    wp: WPARAM,
    lp: LPARAM,
    _: usize,
    root: usize,
) -> LRESULT {
    if msg == WM_SETFOCUS {
        SendMessageW(HWND(root as *mut _), WM_CANCELMODE, None, None);
    }
    DefSubclassProc(hwnd, msg, wp, lp)
}
#[test]
fn synchronous_text_focus_cancel_cannot_republish_logical_focus_or_edit_owner() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let (_, w) = setup(scene());
    navigate(w.hwnd, &w.state, false);
    let h = text::active_child(&w.state);
    assert!(text::commit_for_click(&w.state));
    unsafe {
        let _ = SetFocus(Some(w.hwnd));
        assert!(
            SetWindowSubclass(h, Some(cancel_on_focus), 0x544b4354, w.hwnd.0 as usize).as_bool()
        );
    }
    assert!(text::begin(w.hwnd, &w.state, [8, 8]));
    assert!(w.state.keyboard.borrow().focus.is_none());
    assert!(w.state.text_preview.borrow().is_empty());
    assert!(!unsafe { IsWindow(Some(h)) }.as_bool());
}
#[test]
fn pointer_text_entry_releases_the_previously_active_native_value_popup() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let mut value = value_scene("select");
    value["layout"] = json!({"width":64,"height":32,"minHeight":32});
    let (_, w) = setup(
        json!({"id":"root","type":"column","layout":{"width":64,"height":64},"children":[
        value,{"id":"field","type":"input","layout":{"width":64,"height":32,"minHeight":32},"props":{"value":"local"}}]}),
    );
    navigate(w.hwnd, &w.state, false);
    let h = child(&w);
    let popup = unsafe { GetParent(h).unwrap() };
    unsafe {
        click(w.hwnd, [8, 44]);
    }
    assert_eq!(focus(&w), "field");
    assert!(w.state.keyboard_value.borrow().is_none());
    assert!(!unsafe { IsWindow(Some(h)) }.as_bool());
    assert!(!unsafe { IsWindow(Some(popup)) }.as_bool());
    assert_eq!(unsafe { GetFocus() }, text::active_child(&w.state));
}
#[test]
fn preparing_tab_preserves_text_select_slider_owner_focus_and_accepted_intents() {
    let _com = crate::desktop::Apartment::new().unwrap();
    for kind in ["input", "select", "slider"] {
        let s = if kind == "input" {
            json!({"id":"value","type":"input","layout":{"width":64,"height":64},"props":{"value":"source"}})
        } else {
            value_scene(kind)
        };
        let (f, w) = setup(s);
        navigate(w.hwnd, &w.state, false);
        let h = if kind == "input" {
            text::active_child(&w.state)
        } else {
            child(&w)
        };
        let before = events(&f);
        f.shared.lock().presented_frame = None;
        assert!(w
            .state
            .keyboard
            .borrow()
            .focus
            .as_ref()
            .unwrap()
            .awaiting(&f.shared.lock(), Instant::now()));
        unsafe {
            key(h, WM_KEYDOWN, 9, false);
        }
        assert_eq!(focus(&w), "value");
        assert_eq!(unsafe { GetFocus() }, h);
        assert!(unsafe { IsWindow(Some(h)) }.as_bool());
        assert_eq!(events(&f), before);
        w.tick().unwrap();
        assert_eq!(focus(&w), "value");
        assert_eq!(unsafe { GetFocus() }, h);
        assert_eq!(events(&f), before);
    }
}
#[test]
fn failed_navigation_clears_and_invalidates_already_painted_space_press() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let (f, w) = setup(
        json!({"id":"button","type":"button","layout":{"width":64,"height":64},"events":{"click":"go"}}),
    );
    navigate(w.hwnd, &w.state, false);
    unsafe {
        key(w.hwnd, WM_KEYDOWN, 32, false);
    }
    w.tick().unwrap();
    assert!(visual(&w.state).unwrap().1);
    assert!(!w.state.text_dirty.get());
    let renderer = w.state.renderer.borrow_mut().take();
    navigate(w.hwnd, &w.state, false);
    *w.state.renderer.borrow_mut() = renderer;
    assert!(!visual(&w.state).unwrap().1);
    assert!(w.state.text_dirty.get());
    assert_eq!(focus(&w), "button");
    assert!(events(&f).is_empty());
}
#[test]
fn select_cancel_current_then_switch_keyboard_command_keeps_fresh_dispatch_contract() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let mut select = value_scene("select");
    select["layout"] = json!({"width":64,"height":32,"minHeight":32});
    let (f, w) = setup(
        json!({"id":"root","type":"column","layout":{"width":64,"height":64},"children":[
        select,{"id":"flag","type":"switch","layout":{"width":64,"height":32,"minHeight":32},"props":{"value":false},"events":{"change":"save"}}]}),
    );
    assert!(!w.test_art_keyboard_select(None));
    assert!(!w.test_art_keyboard_select(Some(0)));
    assert!(w.test_art_keyboard_switch([8, 44]));
    let mut command = f.shared.lock().actions.pending.pop_front().unwrap();
    assert!(
        crate::tile_surface_action_dispatch::validate(
            &mut command,
            &f.state,
            Some(&f.art),
            &f.job,
            &f.shared
        ),
        "{}",
        w.test_art_keyboard_queue()
    );
}
