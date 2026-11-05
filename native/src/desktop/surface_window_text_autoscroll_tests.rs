//! 自有 HWND 的 timer/捕获/选区闭环；直接消息不冒充系统输入注入。
use super::*;
#[path = "surface_text_autoscroll_timer_tests.rs"]
mod system_timer;
fn window() -> SurfaceWindow {
    let text = (0..20).map(|n| format!("Line {n}\n")).collect::<String>();
    let scene = serde_json::json!({"id":"root","type":"scroll","layout":{"width":200,"height":60},"events":{"click":"unexpected"},"children":[{"id":"text","type":"text","props":{"text":text,"selectable":true},"layout":{"height":500,"minHeight":500}}]});
    let window = SurfaceWindow::new(
        Scene::parse(&serde_json::to_vec(&scene).unwrap()).unwrap(),
        240,
        140,
    )
    .unwrap();
    window.set_pointer_enabled(true).unwrap();
    window
}
unsafe fn send(window: &SurfaceWindow, message: u32, x: i16, y: i16) {
    SendMessageW(
        window.hwnd,
        message,
        None,
        Some(LPARAM(((y as u16 as u32) << 16 | x as u16 as u32) as isize)),
    );
}
fn y(window: &SurfaceWindow) -> f32 {
    window
        .state
        .host
        .borrow()
        .as_ref()
        .unwrap()
        .presentation()
        .geometry()
        .nodes()
        .iter()
        .find(|n| n.id() == "text")
        .unwrap()
        .origin()[1]
}
fn selected(window: &SurfaceWindow) -> String {
    window
        .state
        .host
        .borrow()
        .as_ref()
        .unwrap()
        .presentation()
        .selected_text()
        .unwrap()
        .unwrap_or_default()
}
fn start(window: &SurfaceWindow) -> usize {
    unsafe {
        send(window, WM_LBUTTONDOWN, 5, 8);
        send(window, WM_MOUSEMOVE, 5, 58);
    }
    window
        .state
        .pointer
        .borrow()
        .selection_scroll
        .as_ref()
        .unwrap()
        .id
}
fn tick(window: &SurfaceWindow, id: usize) {
    if let Some(timer) = window.state.pointer.borrow_mut().selection_scroll.as_mut() {
        timer.due = Instant::now();
    }
    unsafe {
        SendMessageW(window.hwnd, WM_TIMER, Some(WPARAM(id)), None);
    }
}
#[test]
fn text_autoscroll_hwnd_stationary_pointer_extends_without_remeasurement_or_actions() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = window();
    let id = start(&window);
    let before = selected(&window);
    let measured = crate::surface_text::measure_calls();
    assert!(timer(window.hwnd, &window.state, id, Instant::now()));
    assert_eq!(y(&window), 0.);
    for _ in 0..6 {
        tick(&window, id);
    }
    assert!(y(&window) < -60.);
    assert!(selected(&window).len() > before.len());
    assert_eq!(crate::surface_text::measure_calls(), measured);
    unsafe {
        send(&window, WM_LBUTTONUP, 5, 58);
    }
    assert!(window.state.pointer.borrow().selection_scroll.is_none());
    assert_ne!(unsafe { GetCapture() }, window.hwnd);
    let after = selected(&window);
    assert!(!timer(
        window.hwnd,
        &window.state,
        id,
        Instant::now() + Duration::from_secs(1)
    ));
    assert_eq!(selected(&window), after);
    assert!(!window.has_pending_click());
    assert!(!window.has_pending_edit());
    assert!(window.take_error().is_none());
}
#[test]
fn text_autoscroll_hwnd_interior_stops_and_reentry_uses_new_identity() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = window();
    let old = start(&window);
    tick(&window, old);
    unsafe {
        send(&window, WM_MOUSEMOVE, 70, 30);
    }
    assert!(window.state.pointer.borrow().selection_scroll.is_none());
    unsafe {
        send(&window, WM_MOUSEMOVE, 5, 58);
    }
    let next = window
        .state
        .pointer
        .borrow()
        .selection_scroll
        .as_ref()
        .unwrap()
        .id;
    assert_ne!(next, old);
    let before = y(&window);
    assert!(!timer(
        window.hwnd,
        &window.state,
        old,
        Instant::now() + Duration::from_secs(1)
    ));
    assert_eq!(y(&window), before);
    tick(&window, next);
    assert!(y(&window) < before);
    assert!(window.take_error().is_none());
}
#[test]
fn text_autoscroll_hwnd_boundary_stops_instead_of_idle_polling() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = window();
    let id = start(&window);
    for _ in 0..50 {
        tick(&window, id);
    }
    assert!(window.state.pointer.borrow().selection_scroll.is_none());
    assert!(y(&window) < -100.);
    assert!(window.take_error().is_none());
}
#[test]
fn text_autoscroll_hwnd_cancel_revoke_focus_and_reentrant_cancel_drop_timer() {
    let _com = crate::desktop::Apartment::new().unwrap();
    for mode in 0..6 {
        let window = window();
        let id = start(&window);
        match mode {
            0 => unsafe {
                send(&window, WM_CANCELMODE, 0, 0);
            },
            1 => window.set_input_policy(false, false).unwrap(),
            2 => unsafe {
                send(&window, WM_KILLFOCUS, 0, 0);
            },
            3 => {
                window
                    .state
                    .run(|_| {
                        unsafe {
                            send(&window, WM_CANCELMODE, 0, 0);
                        }
                        Ok(())
                    })
                    .unwrap();
            }
            4 => window
                .replace(
                    Scene::parse(br#"{"id":"new","type":"text","props":{"text":"replacement"}}"#)
                        .unwrap(),
                )
                .unwrap(),
            _ => window.close().unwrap(),
        }
        assert!(window.state.pointer.borrow().selection_scroll.is_none());
        assert!(!timer(
            window.hwnd,
            &window.state,
            id,
            Instant::now() + Duration::from_secs(1)
        ));
        if window.is_alive() {
            assert!(selected(&window).is_empty());
        }
        assert!(!window.has_pending_click());
        assert!(window.take_error().is_none());
    }
}
#[test]
fn text_autoscroll_pending_reentrant_cancel_cannot_resurrect_timer() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = window();
    let old = start(&window);
    update_using(window.hwnd, &window.state, [5, 58], || {
        window
            .state
            .run(|host| {
                assert!(host.presentation().text_drag_scroll_pending([5., 58.]));
                unsafe {
                    send(&window, WM_CANCELMODE, 0, 0);
                }
                Ok(())
            })
            .unwrap();
        true
    });
    assert!(window.state.pointer.borrow().selection_scroll.is_none());
    assert!(selected(&window).is_empty());
    assert!(!timer(
        window.hwnd,
        &window.state,
        old,
        Instant::now() + Duration::from_secs(1)
    ));
}
#[test]
fn text_autoscroll_upper_edge_reverses_direction() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = window();
    let id = start(&window);
    for _ in 0..6 {
        tick(&window, id);
    }
    let before = y(&window);
    unsafe {
        send(&window, WM_MOUSEMOVE, 5, 1);
    }
    tick(&window, id);
    assert!(y(&window) > before);
    assert!(window.take_error().is_none());
}
#[test]
fn text_autoscroll_nested_consumes_inner_before_outer_and_keeps_owner_visible() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let scene = serde_json::json!({"id":"root","type":"scroll","layout":{"width":200,"height":80},"children":[{"id":"inner","type":"scroll","layout":{"height":50,"minHeight":50},"children":[{"id":"text","type":"text","props":{"text":"A\nB\nC\nD\nE\nF\nG\nH","selectable":true},"layout":{"height":500,"minHeight":500}}]},{"id":"tail","type":"view","layout":{"height":300,"minHeight":300}}]});
    let window = SurfaceWindow::new(
        Scene::parse(&serde_json::to_vec(&scene).unwrap()).unwrap(),
        240,
        140,
    )
    .unwrap();
    window.set_pointer_enabled(true).unwrap();
    unsafe {
        send(&window, WM_LBUTTONDOWN, 1, 8);
        send(&window, WM_MOUSEMOVE, 5, 48);
    }
    let id = window
        .state
        .pointer
        .borrow()
        .selection_scroll
        .as_ref()
        .unwrap()
        .id;
    let inner_y = || {
        window
            .state
            .host
            .borrow()
            .as_ref()
            .unwrap()
            .presentation()
            .geometry()
            .nodes()
            .iter()
            .find(|n| n.id() == "inner")
            .unwrap()
            .origin()[1]
    };
    tick(&window, id);
    assert_eq!(inner_y(), 0.);
    assert!(y(&window) < 0.);
    for _ in 0..40 {
        tick(&window, id);
    }
    assert!(inner_y() < 0.);
    assert!(window.state.pointer.borrow().selection_scroll.is_none());
    assert!(!window
        .state
        .host
        .borrow()
        .as_ref()
        .unwrap()
        .presentation()
        .text()
        .items()
        .iter()
        .find(|item| item.node_id() == "text")
        .unwrap()
        .clip()
        .empty());
    assert!(window.take_error().is_none());
}
#[test]
fn text_autoscroll_horizontal_auto_moves_but_hidden_does_not() {
    let _com = crate::desktop::Apartment::new().unwrap();
    for mode in ["auto", "hidden"] {
        let scene = serde_json::json!({"id":"root","type":"view","layout":{"width":80,"height":60,"overflowX":mode},"children":[{"id":"text","type":"text","props":{"text":"ABCDEFGHIJKLMNOPQRSTUVWXYZ","selectable":true},"layout":{"width":500,"minWidth":500,"height":40}}]});
        let window = SurfaceWindow::new(
            Scene::parse(&serde_json::to_vec(&scene).unwrap()).unwrap(),
            240,
            140,
        )
        .unwrap();
        window.set_pointer_enabled(true).unwrap();
        unsafe {
            send(&window, WM_LBUTTONDOWN, 1, 8);
            send(&window, WM_MOUSEMOVE, 78, 8);
        }
        let id = window
            .state
            .pointer
            .borrow()
            .selection_scroll
            .as_ref()
            .map(|t| t.id);
        assert_eq!(id.is_some(), mode == "auto");
        if let Some(id) = id {
            for _ in 0..4 {
                tick(&window, id);
            }
            let x = window
                .state
                .host
                .borrow()
                .as_ref()
                .unwrap()
                .presentation()
                .geometry()
                .nodes()
                .iter()
                .find(|n| n.id() == "text")
                .unwrap()
                .origin()[0];
            assert!(x < 0.);
            unsafe {
                send(&window, WM_MOUSEMOVE, 1, 8);
            }
            tick(&window, id);
            let next = window
                .state
                .host
                .borrow()
                .as_ref()
                .unwrap()
                .presentation()
                .geometry()
                .nodes()
                .iter()
                .find(|n| n.id() == "text")
                .unwrap()
                .origin()[0];
            assert!(next > x);
        }
        assert!(window.take_error().is_none());
    }
}
