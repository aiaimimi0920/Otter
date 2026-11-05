//! 实际自有离屏Tile HWND消息/捕获与绘制缓存；不SendInput、不移动鼠标、不激活用户窗口。
use super::*;
use crate::tile_surface_slider::tests::scene;
use crate::tile_surface_value::tests::fixture;
use windows::Win32::System::Com::*;
struct Apartment;
impl Apartment {
    fn new() -> Self {
        unsafe {
            CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok().unwrap();
        }
        Self
    }
}
impl Drop for Apartment {
    fn drop(&mut self) {
        unsafe {
            CoUninitialize();
        }
    }
}
fn window(f: &crate::tile_surface_actions::tests::Fixture) -> Window {
    let mut output = crate::tile_tests::output();
    (output.x, output.y) = (-30000, -30000);
    let w = Window::new(f.shared.clone(), &output).unwrap();
    w.tick().unwrap();
    w.tick().unwrap();
    assert!(w.state.painted.get());
    w
}
unsafe fn send(w: &Window, message: u32, pixel: [i32; 2]) {
    SendMessageW(
        w.hwnd,
        message,
        Some(WPARAM(0)),
        Some(LPARAM(
            (pixel[0] as u16 as isize) | ((pixel[1] as u16 as isize) << 16),
        )),
    );
}
#[test]
fn actual_messages_coalesce_drag_and_commit_then_release_capture_without_foreground() {
    let _com = Apartment::new();
    let f = fixture(scene());
    let w = window(&f);
    let foreground = unsafe { GetForegroundWindow() };
    unsafe {
        send(&w, WM_LBUTTONDOWN, [0, 0]);
    }
    assert_eq!(unsafe { GetCapture() }, w.hwnd);
    for i in 0..100 {
        unsafe {
            send(&w, WM_MOUSEMOVE, [i % 2, 0]);
        }
    }
    w.tick().unwrap();
    assert_eq!(f.shared.lock().actions.pending.len(), 1);
    assert!(matches!(&*w.state.pressed.borrow(),Gesture::Slider(d) if d.value==6.));
    unsafe {
        send(&w, WM_LBUTTONUP, [1, 0]);
    }
    assert_ne!(unsafe { GetCapture() }, w.hwnd);
    assert_eq!(unsafe { GetForegroundWindow() }, foreground);
    assert!(matches!(*w.state.pressed.borrow(), Gesture::None));
    assert!(w.state.slider_preview.borrow().is_some());
    w.tick().unwrap();
    assert!(w.state.slider_preview.borrow().is_some()); // 抬起后的待发/在途不得闪回旧权威值。
    let mut s = f.shared.lock();
    assert_eq!(s.actions.pending.len(), 2);
    let first = s.actions.take(&f.state, Instant::now()).unwrap();
    let last = s.actions.take(&f.state, Instant::now()).unwrap();
    assert!(
        matches!(&first.kind,Kind::Event(e) if e["event"]=="input" && e["payload"]==serde_json::json!({"value":6.0}))
    );
    assert!(
        matches!(&last.kind,Kind::Event(e) if e["event"]=="change" && e["payload"]==serde_json::json!({"value":6.0}))
    );
    s.actions
        .attempted(&first, &f.ack(&first, "running"), Instant::now());
    s.actions
        .attempted(&last, &f.ack(&last, "running"), Instant::now());
    s.actions
        .observe(f.state.clone(), "origin", "device", Instant::now());
    drop(s);
    w.tick().unwrap();
    assert!(w.state.slider_preview.borrow().is_none());
}
#[test]
fn capture_loss_hide_freeze_stop_dpi_or_destroy_discard_unsent_drag_and_never_commit() {
    let _com = Apartment::new();
    for case in 0..8 {
        let f = fixture(scene());
        let w = window(&f);
        unsafe {
            send(&w, WM_LBUTTONDOWN, [0, 0]);
        }
        assert_eq!(f.shared.lock().actions.pending.len(), 1);
        unsafe {
            match case {
                0 => {
                    send(&w, WM_CANCELMODE, [0, 0]);
                }
                1 => {
                    send(&w, WM_KILLFOCUS, [0, 0]);
                }
                2 => {
                    ReleaseCapture().unwrap();
                }
                3 => {
                    let _ = ShowWindow(w.hwnd, SW_HIDE);
                }
                4 => {
                    f.shared.lock().frozen = true;
                    w.tick().unwrap();
                }
                5 => {
                    f.shared.stop();
                    w.tick().unwrap();
                }
                6 => {
                    send(&w, WM_DPICHANGED, [0, 0]);
                }
                _ => {
                    DestroyWindow(w.hwnd).unwrap();
                }
            }
            if !w.state.closed.get() {
                send(&w, WM_LBUTTONUP, [1, 0]);
            }
        }
        assert!(f.shared.lock().actions.pending.is_empty(), "case {case}");
        assert_ne!(unsafe { GetCapture() }, w.hwnd, "case {case}");
    }
}
#[test]
fn captured_outside_drag_clamps_on_both_sides_and_commits_once() {
    let _com = Apartment::new();
    let f = fixture(scene());
    let w = window(&f);
    unsafe {
        send(&w, WM_LBUTTONDOWN, [0, 0]);
        send(&w, WM_MOUSEMOVE, [-100, 0]);
        send(&w, WM_LBUTTONUP, [100, 0]);
    }
    let s = f.shared.lock();
    assert!(
        matches!(&s.actions.pending.back().unwrap().kind,Kind::Event(e) if e["event"]=="change" && e["payload"]==serde_json::json!({"value":10.0}))
    );
}
#[test]
fn own_value_snapshot_preparation_gap_retains_capture_but_never_samples_blank_content() {
    let _com = Apartment::new();
    let f = fixture(scene());
    let w = window(&f);
    unsafe {
        send(&w, WM_LBUTTONDOWN, [0, 0]);
    }
    let count = f.shared.lock().actions.test_offered;
    f.shared.lock().unavailable(
        f.job.generation,
        &f.job.key,
        "preparing changed Art snapshot",
    );
    w.tick().unwrap();
    assert!(matches!(&*w.state.pressed.borrow(), Gesture::Slider(_)));
    assert_eq!(unsafe { GetCapture() }, w.hwnd);
    unsafe {
        send(&w, WM_MOUSEMOVE, [1, 0]);
    }
    assert_eq!(f.shared.lock().actions.test_offered, count);
    let mut updated = scene();
    updated["props"]["value"] = serde_json::json!(6);
    let next = fixture(updated);
    f.shared
        .lock()
        .actions
        .observe(next.state.clone(), "origin", "device", Instant::now());
    f.shared.lock().art = Some(Arc::new(crate::tile_surface_worker::Prepared {
        frames: next.frame.arts.clone(),
        pixels: 0,
    }));
    // Art先就绪，但Assets尚未发布完整帧；仍不能把这个准备阶段当成取消。
    w.tick().unwrap();
    assert!(matches!(&*w.state.pressed.borrow(), Gesture::Slider(_)));
    assert_eq!(unsafe { GetCapture() }, w.hwnd);
    unsafe {
        send(&w, WM_MOUSEMOVE, [1, 0]);
    }
    assert_eq!(f.shared.lock().actions.test_offered, count);
    f.shared.lock().frame = Some(next.frame.clone());
    // 候选已发布但还没有真实paint；鼠标消息同样只保留手势、不借旧画面采样。
    unsafe {
        send(&w, WM_MOUSEMOVE, [1, 0]);
    }
    assert!(matches!(&*w.state.pressed.borrow(), Gesture::Slider(_)));
    assert_eq!(f.shared.lock().actions.test_offered, count);
    w.tick().unwrap();
    assert!(matches!(&*w.state.pressed.borrow(), Gesture::Slider(_)));
    unsafe {
        send(&w, WM_MOUSEMOVE, [1, 0]);
        send(&w, WM_LBUTTONUP, [1, 0]);
    }
    assert!(
        matches!(f.shared.lock().actions.pending.back().unwrap().kind,Kind::Event(ref e) if e["event"]=="change" && e["payload"]["value"]==6.0)
    );
}
#[test]
fn input_only_release_keeps_pending_preview_without_fabricating_change() {
    let _com = Apartment::new();
    let mut input = scene();
    input["events"].as_object_mut().unwrap().remove("change");
    let f = fixture(input);
    let w = window(&f);
    unsafe {
        send(&w, WM_LBUTTONDOWN, [0, 0]);
        send(&w, WM_MOUSEMOVE, [1, 0]);
        send(&w, WM_LBUTTONUP, [1, 0]);
    }
    assert!(matches!(*w.state.pressed.borrow(), Gesture::None));
    assert_ne!(unsafe { GetCapture() }, w.hwnd);
    assert!(w.state.slider_preview.borrow().is_some());
    w.tick().unwrap();
    assert!(w.state.slider_preview.borrow().is_some());
    let s = f.shared.lock();
    assert_eq!(s.actions.pending.len(), 1);
    assert!(matches!(&s.actions.pending[0].kind,Kind::Event(e)
        if e["event"] == "input" && e["payload"]["value"] == 6.0));
}
