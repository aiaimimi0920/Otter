//! 自有离屏HWND；逻辑时钟驱动原真实timer，不移动鼠标、不向用户应用注入消息。
use super::*;
use crate::tile_surface_actions::{tests::Fixture, Kind};
use serde_json::{json, Value};
use std::time::Duration;
fn scene() -> Value {
    json!({"id":"root","type":"scroll","layout":{"width":64,"height":64},"children":[
        {"id":"first","type":"input","layout":{"width":64,"height":20,"minHeight":20},"props":{"value":"source"},"events":{"input":"preview","change":"save"}},
        {"id":"gap","type":"view","layout":{"height":980,"minHeight":980}},
        {"id":"send","type":"button","layout":{"width":64,"height":20,"minHeight":20},"props":{"includeSurfaceValues":true},"events":{"click":"go"}}]})
}
fn setup() -> (Fixture, Window) {
    let mut f = crate::tile_surface_value::tests::fixture(scene());
    let frame = Arc::new(Frame {
        key: f.frame.key.clone(),
        layout: f.frame.layout.clone(),
        size: Size {
            width: 64,
            height: 64,
        },
        images: Default::default(),
        arts: f.frame.arts.clone(),
        live_until: None,
    });
    f.job.size = frame.size;
    f.frame = frame.clone();
    f.shared.lock().frame = Some(frame);
    f.shared.lock().presented_frame = None;
    let mut output = crate::tile_tests::output();
    (output.x, output.y, output.width, output.height) = (-30000, -30000, 64, 64);
    let w = Window::new(f.shared.clone(), &output).unwrap();
    w.tick().unwrap();
    w.tick().unwrap();
    (f, w)
}
fn send(w: &Window, msg: u32, p: [i32; 2]) {
    w.test_art_slider_message(msg, p);
}
fn field(w: &Window) -> Field {
    let frame = actions::active(&w.state).unwrap();
    w.state
        .renderer
        .borrow()
        .as_ref()
        .unwrap()
        .scrollbar(&frame, [60, 5])
        .unwrap()
}
fn thumb(w: &Window, offset: f32) {
    assert!(
        (field(w).bar.thumb.top - 46. * offset / 956.).abs() < 0.001,
        "{} != offset {offset}",
        field(w).bar.thumb.top
    );
}
fn timer(w: &Window) -> usize {
    let owner = w.state.scrollbar.borrow();
    let Motion::Page(r) = &owner.gesture.as_ref().unwrap().motion else {
        panic!()
    };
    r.id()
}
fn fire(w: &Window, id: usize, now: Instant) {
    drive(w.hwnd, &w.state, WM_TIMER, [0, 0], id, now, false).unwrap();
}
fn events(f: &Fixture) -> Vec<Value> {
    f.shared
        .lock()
        .actions
        .pending
        .iter()
        .filter_map(|c| {
            if let Kind::Event(v) = &c.kind {
                Some(v.clone())
            } else {
                None
            }
        })
        .collect()
}
#[test]
fn thumb_drag_commits_actual_edit_once_preserves_form_and_never_synthesizes_scroll_click() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let (f, w) = setup();
    let foreground = unsafe { GetForegroundWindow() };
    w.test_art_text([8, 8], "滚动🙂草稿", true, false);
    send(&w, WM_LBUTTONDOWN, [60, 5]);
    assert_eq!(unsafe { GetCapture() }, w.hwnd);
    assert_eq!(
        events(&f)
            .iter()
            .map(|v| v["event"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["input", "change"]
    );
    send(&w, WM_MOUSEMOVE, [60, 51]);
    thumb(&w, 956.);
    send(&w, WM_LBUTTONUP, [60, 200]);
    assert_ne!(unsafe { GetCapture() }, w.hwnd);
    assert!(w.state.scrollbar.borrow().gesture_is_idle());
    assert_eq!(events(&f).len(), 2);
    assert!(w.test_art_click([8, 52]));
    let commands = std::mem::take(&mut f.shared.lock().actions.pending);
    assert_eq!(commands.len(), 3);
    for mut c in commands {
        assert!(crate::tile_surface_action_dispatch::validate(
            &mut c,
            &f.state,
            Some(&f.art),
            &f.job,
            &f.shared
        ));
        if let Kind::Event(v) = &c.kind {
            if v["event"] == "click" {
                assert_eq!(v["payload"]["surfaceValues"]["first"], "滚动🙂草稿");
            }
        }
    }
    assert_eq!(unsafe { GetForegroundWindow() }, foreground);
}
#[test]
fn track_repeat_waits_coalesces_stops_under_thumb_and_resumes_only_in_original_direction() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let (f, w) = setup();
    let start = Instant::now();
    send(&w, WM_LBUTTONDOWN, [60, 50]);
    thumb(&w, 64.);
    let id = timer(&w);
    fire(&w, id, start);
    thumb(&w, 64.);
    let due = start + Duration::from_secs(1);
    fire(&w, id, due);
    thumb(&w, 128.);
    fire(&w, id, due);
    thumb(&w, 128.);
    fire(&w, id, due + Duration::from_secs(10));
    thumb(&w, 192.);
    for n in 1..=20 {
        fire(
            &w,
            id,
            due + Duration::from_secs(10) + Duration::from_millis(n * 50),
        );
    }
    thumb(&w, 704.);
    send(&w, WM_MOUSEMOVE, [8, 60]);
    fire(&w, id, due + Duration::from_secs(20));
    thumb(&w, 704.);
    send(&w, WM_MOUSEMOVE, [60, 1]);
    fire(&w, id, due + Duration::from_secs(21));
    thumb(&w, 704.);
    send(&w, WM_MOUSEMOVE, [60, 63]);
    for n in 0..=8 {
        fire(
            &w,
            id,
            due + Duration::from_secs(22) + Duration::from_millis(n * 50),
        );
    }
    thumb(&w, 956.);
    send(&w, WM_LBUTTONUP, [60, 63]);
    assert!(w.state.scrollbar.borrow().gesture_is_idle());
    assert_ne!(unsafe { GetCapture() }, w.hwnd);
    assert!(events(&f).is_empty());
    send(&w, WM_LBUTTONDOWN, [60, 1]);
    let next = timer(&w);
    assert_ne!(next, id);
    thumb(&w, 892.);
    assert!(!drive(
        w.hwnd,
        &w.state,
        WM_TIMER,
        [0, 0],
        id,
        due + Duration::from_secs(100),
        false
    )
    .unwrap());
    thumb(&w, 892.);
    cancel(w.hwnd, &w.state);
}
#[test]
fn ime_composition_rejects_bar_without_losing_child_or_draft() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let (f, w) = setup();
    w.test_art_text([8, 8], "草稿", false, false);
    let h = text::active_child(&w.state);
    let before = events(&f);
    unsafe {
        SendMessageW(h, WM_IME_STARTCOMPOSITION, None, None);
    }
    send(&w, WM_LBUTTONDOWN, [60, 5]);
    assert!(w.state.scrollbar.borrow().gesture_is_idle());
    thumb(&w, 0.);
    assert_eq!(text::active_child(&w.state), h);
    assert_eq!(events(&f), before);
    unsafe {
        SendMessageW(h, WM_IME_ENDCOMPOSITION, None, None);
    }
    send(&w, WM_LBUTTONDOWN, [60, 5]);
    assert!(captured(&w.state));
    cancel(w.hwnd, &w.state);
}
#[path = "tile_surface_scrollbar_lifecycle_tests.rs"]
mod lifecycle;
