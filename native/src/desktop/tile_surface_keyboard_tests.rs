//! 自有离屏HWND的真实键盘/原生子控件通知；不SendInput、不改用户焦点或真实profile。
use super::*;
use crate::tile_surface_actions::{tests::Fixture, Kind as Action};
use serde_json::{json, Value};
use windows::Win32::UI::Controls::{EM_REPLACESEL, EM_SETSEL};
fn setup(scene: Value) -> (Fixture, Window) {
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
    unsafe {
        let _ = SetFocus(Some(w.hwnd));
    }
    (f, w)
}
fn scene() -> Value {
    json!({"id":"root","type":"column","layout":{"width":64,"height":64},"children":[
        {"id":"first","type":"input","layout":{"width":64,"height":20,"minHeight":20},"props":{"value":"source"}},
        {"id":"disabled","type":"button","props":{"disabled":true},"layout":{"width":64,"height":10,"minHeight":10}},
        {"id":"send","type":"button","layout":{"width":64,"height":24,"minHeight":24},"props":{"includeSurfaceValues":true},"events":{"click":"go"}}]})
}
unsafe fn key(hwnd: HWND, msg: u32, key: usize, repeat: bool) {
    SendMessageW(
        hwnd,
        msg,
        Some(WPARAM(key)),
        Some(LPARAM(if repeat { 1 << 30 } else { 0 })),
    );
}
fn focus(w: &Window) -> String {
    w.state
        .keyboard
        .borrow()
        .focus
        .as_ref()
        .unwrap()
        .node
        .clone()
}
fn events(f: &Fixture) -> Vec<Value> {
    f.shared
        .lock()
        .actions
        .pending
        .iter()
        .filter_map(|c| match &c.kind {
            Action::Event(e) => Some(e.clone()),
            _ => None,
        })
        .collect()
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
#[test]
fn tab_edit_to_button_commits_local_draft_and_enter_repeat_does_not_duplicate() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let foreground = unsafe { GetForegroundWindow() };
    let (f, w) = setup(scene());
    unsafe {
        key(w.hwnd, WM_KEYDOWN, 9, false);
    }
    assert_eq!(focus(&w), "first");
    let h = unsafe { GetFocus() };
    assert_ne!(h, w.hwnd);
    unsafe {
        SendMessageW(h, EM_SETSEL, Some(WPARAM(0)), Some(LPARAM(-1)));
        let value = windows::core::HSTRING::from("本地🙂");
        SendMessageW(
            h,
            EM_REPLACESEL,
            Some(WPARAM(1)),
            Some(LPARAM(value.as_ptr() as isize)),
        );
        key(h, WM_KEYDOWN, 9, false);
    }
    assert_eq!(focus(&w), "send");
    assert_eq!(unsafe { GetFocus() }, w.hwnd);
    unsafe {
        key(w.hwnd, WM_KEYDOWN, 13, false);
        key(w.hwnd, WM_KEYDOWN, 13, true);
        key(w.hwnd, WM_KEYUP, 13, false);
    }
    let e = events(&f);
    assert_eq!(e.len(), 1);
    assert_eq!(e[0]["payload"]["surfaceValues"]["first"], "本地🙂");
    assert_eq!(unsafe { GetForegroundWindow() }, foreground);
    navigate(w.hwnd, &w.state, true);
    assert_eq!(focus(&w), "first");
    navigate(w.hwnd, &w.state, true);
    assert_eq!(focus(&w), "send");
    assert_eq!(f.art.snapshot.scene, serde_json::to_vec(&scene()).unwrap());
}
#[test]
fn button_space_release_cancels_on_tab_focus_loss_or_snapshot_gap_and_switch_ignores_enter() {
    let _com = crate::desktop::Apartment::new().unwrap();
    for cancel in 0..3 {
        let (f, w) = setup(
            json!({"id":"b","type":"button","layout":{"width":64,"height":64},"events":{"click":"go"}}),
        );
        navigate(w.hwnd, &w.state, false);
        unsafe {
            key(w.hwnd, WM_KEYDOWN, 32, false);
        }
        match cancel {
            0 => navigate(w.hwnd, &w.state, false),
            1 => unsafe {
                SendMessageW(w.hwnd, WM_KILLFOCUS, None, None);
            },
            _ => {
                f.shared
                    .lock()
                    .unavailable(f.job.generation, &f.job.key, "preparing");
                w.tick().unwrap();
            }
        }
        unsafe {
            key(w.hwnd, WM_KEYUP, 32, false);
        }
        assert!(events(&f).is_empty());
    }
    let (f, w) = setup(
        json!({"id":"switch","type":"switch","layout":{"width":64,"height":64},"props":{"value":false},"events":{"change":"save"}}),
    );
    navigate(w.hwnd, &w.state, false);
    unsafe {
        key(w.hwnd, WM_KEYDOWN, 13, false);
        key(w.hwnd, WM_KEYUP, 13, false);
    }
    assert!(events(&f).is_empty());
    unsafe {
        key(w.hwnd, WM_KEYDOWN, 32, false);
        key(w.hwnd, WM_KEYDOWN, 32, true);
        key(w.hwnd, WM_KEYUP, 32, false);
    }
    assert_eq!(events(&f).len(), 1);
    assert_eq!(events(&f)[0]["payload"]["value"], true);
}
#[test]
fn native_select_arrows_and_slider_page_home_end_use_existing_notifications_and_bounds() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let (f, w) = setup(
        json!({"id":"select","type":"select","layout":{"width":64,"height":64},"props":{"value":"a","options":[{"label":"A","value":"a"},{"label":"B","value":"b"}]},"events":{"change":"save"}}),
    );
    navigate(w.hwnd, &w.state, false);
    let h = child(&w);
    unsafe {
        key(h, WM_KEYDOWN, 40, false);
    }
    assert_eq!(events(&f).len(), 1);
    unsafe {
        key(h, WM_KEYDOWN, 38, false);
    }
    assert_eq!(events(&f).len(), 2);
    assert_eq!(events(&f).last().unwrap()["payload"]["value"], "a");
    assert_eq!(events(&f)[0]["payload"]["value"], "b");
    unsafe {
        key(h, WM_KEYDOWN, 40, false);
    }
    assert_eq!(events(&f).len(), 3);
    assert_eq!(events(&f).last().unwrap()["payload"]["value"], "b");
    unsafe {
        key(h, WM_KEYDOWN, 40, false);
    }
    assert_eq!(events(&f).len(), 3);
    drop(w);
    let (f, w) = setup(
        json!({"id":"slider","type":"slider","layout":{"width":64,"height":64},"props":{"value":2,"min":0,"max":10,"step":2},"events":{"input":"preview","change":"save"}}),
    );
    navigate(w.hwnd, &w.state, false);
    let h = child(&w);
    for (key_code, value) in [(39, 4.), (33, 6.), (35, 10.), (36, 0.)] {
        unsafe {
            key(h, WM_KEYDOWN, key_code, false);
        }
        let e = events(&f);
        assert_eq!(e[e.len() - 2]["event"], "input");
        assert_eq!(e.last().unwrap()["event"], "change");
        assert_eq!(e.last().unwrap()["payload"]["value"], value);
    }
    let count = events(&f).len();
    unsafe {
        key(h, WM_KEYDOWN, 36, false);
    }
    assert_eq!(events(&f).len(), count);
}
#[test]
fn value_lifecycle_clears_pending_and_owned_hwnds_without_replaying_late_messages() {
    let _com = crate::desktop::Apartment::new().unwrap();
    for case in 0..5 {
        let (f, w) = setup(
            json!({"id":"slider","type":"slider","layout":{"width":64,"height":64},"props":{"value":2,"min":0,"max":10,"step":2},"events":{"input":"preview","change":"save"}}),
        );
        navigate(w.hwnd, &w.state, false);
        let h = child(&w);
        let popup = unsafe { GetParent(h).unwrap() };
        unsafe {
            key(h, WM_KEYDOWN, 39, false);
        }
        assert_eq!(events(&f).len(), 2);
        unsafe {
            match case {
                0 => {
                    SendMessageW(w.hwnd, WM_CANCELMODE, None, None);
                }
                1 => {
                    SendMessageW(w.hwnd, WM_DPICHANGED, None, None);
                }
                2 => {
                    f.shared.lock().frozen = true;
                    w.tick().unwrap();
                }
                3 => {
                    f.shared.stop();
                    w.tick().unwrap();
                }
                _ => {
                    DestroyWindow(w.hwnd).unwrap();
                }
            }
        }
        assert!(events(&f).is_empty(), "case {case}");
        assert!(w.state.keyboard.borrow().focus.is_none());
        assert!(!unsafe { IsWindow(Some(h)) }.as_bool());
        assert!(!unsafe { IsWindow(Some(popup)) }.as_bool());
    }
}
#[test]
fn preparation_gap_rolls_native_values_back_and_keeps_only_previously_accepted_intents() {
    let _com = crate::desktop::Apartment::new().unwrap();
    for scene in [
        json!({"id":"select","type":"select","layout":{"width":64,"height":64},
        "props":{"value":"a","options":[{"label":"A","value":"a"},{"label":"B","value":"b"}]},"events":{"change":"save"}}),
        json!({"id":"slider","type":"slider","layout":{"width":64,"height":64},
        "props":{"value":2,"min":0,"max":10,"step":2},"events":{"input":"preview","change":"save"}}),
    ] {
        let (f, w) = setup(scene);
        navigate(w.hwnd, &w.state, false);
        let h = child(&w);
        let select = focus(&w) == "select";
        unsafe {
            key(h, WM_KEYDOWN, if select { 40 } else { 39 }, false);
        }
        let expected = if select { json!("b") } else { json!(4.) };
        let before = events(&f);
        assert!(!before.is_empty());
        f.shared.lock().presented_frame = None;
        unsafe {
            key(h, WM_KEYDOWN, if select { 38 } else { 39 }, false);
        }
        assert_eq!(events(&f), before);
        assert_eq!(
            w.state
                .keyboard_value
                .borrow()
                .as_ref()
                .unwrap()
                .displayed_value(),
            expected
        );
        w.tick().unwrap();
        assert!(w.state.keyboard_value.borrow().is_some());
        assert_eq!(events(&f), before);
    }
}
#[path = "tile_surface_keyboard_atomic_tests.rs"]
mod atomic;
#[path = "tile_surface_keyboard_cancel_tests.rs"]
mod cancellation;
#[path = "tile_surface_keyboard_fixture_tests.rs"]
mod fixture;
#[path = "tile_surface_scroll_window_tests.rs"]
mod scrolling;
#[path = "tile_surface_keyboard_transfer_tests.rs"]
mod transfer;
