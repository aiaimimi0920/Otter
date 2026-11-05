//! 实时几何/滚动/生命周期回归；可见测试仅 SW_SHOWNOACTIVATE，不注入系统输入。
use super::*;
use windows::Win32::UI::HiDpi::GetDpiForWindow;

fn scene() -> Scene {
    Scene::parse(br#"{"id":"root","type":"scroll","layout":{"width":100,"height":100},"children":[{"id":"space","type":"view","layout":{"height":80,"minHeight":80}},{"id":"button","type":"button","props":{"label":"Run"},"layout":{"height":32,"minHeight":32}},{"id":"tail","type":"view","layout":{"height":200,"minHeight":200}}]}"#).unwrap()
}
#[test]
fn accessibility_screen_bounds_follow_window_move_and_keep_unclipped_size() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = SurfaceWindow::new(scene(), 240, 200).unwrap();
    let handle = window.accessibility_node("button").unwrap();
    let (before, hidden) = bounds(window.hwnd, &handle).unwrap();
    let scale = unsafe { GetDpiForWindow(window.hwnd) } as f64 / 96.;
    assert_eq!(before.height, 32. * scale);
    assert!(hidden);
    let mut rect = RECT::default();
    unsafe {
        GetWindowRect(window.hwnd, &mut rect).unwrap();
        SetWindowPos(
            window.hwnd,
            None,
            rect.left + 27,
            rect.top + 31,
            0,
            0,
            SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOZORDER,
        )
        .unwrap();
    }
    let (after, _) = bounds(window.hwnd, &handle).unwrap();
    assert_eq!(after.left - before.left, 27.);
    assert_eq!(after.top - before.top, 31.);
    assert_eq!(after.height, before.height);
}

#[test]
fn accessibility_point_and_offscreen_follow_scroll_clips_and_visibility() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = SurfaceWindow::new(scene(), 240, 200).unwrap();
    let handle = window.accessibility_node("button").unwrap();
    let root = window.accessibility_node("root").unwrap();
    let (before, _) = bounds(window.hwnd, &handle).unwrap();
    let center = |rect: UiaRect| [rect.left + 10., rect.top + rect.height / 2.];
    assert!(matches!(
        hit(window.hwnd, &root, center(before)).unwrap(),
        Hit::Outside
    ));
    unsafe {
        let _ = ShowWindow(window.hwnd, SW_SHOWNOACTIVATE);
    }
    assert!(!offscreen(window.hwnd, &handle).unwrap());
    let Hit::Node(node) = hit(window.hwnd, &root, center(before)).unwrap() else {
        panic!("missing button hit");
    };
    assert_eq!(node.runtime_id(), handle.runtime_id());
    window
        .state
        .run(|host| {
            assert!(host.scroll([5, 5], 1, 60., false)?);
            Ok(())
        })
        .unwrap();
    let (after, clipped) = bounds(window.hwnd, &handle).unwrap();
    let scale = unsafe { GetDpiForWindow(window.hwnd) } as f64 / 96.;
    assert_eq!(before.top - after.top, 60. * scale);
    assert!(!clipped);
    assert!(matches!(
        hit(window.hwnd, &root, center(after)).unwrap(),
        Hit::Node(_)
    ));
    window
        .state
        .run(|host| {
            assert!(host.scroll([5, 5], 1, 100., false)?);
            Ok(())
        })
        .unwrap();
    assert!(offscreen(window.hwnd, &handle).unwrap());
    assert!(hit(window.hwnd, &root, [f64::NAN, 0.]).is_err());
    assert!(matches!(
        hit(window.hwnd, &root, [-1e30, 0.]).unwrap(),
        Hit::Outside
    ));
}

#[test]
fn accessibility_bridge_rejects_busy_foreign_and_replaced_snapshots() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = SurfaceWindow::new(scene(), 240, 200).unwrap();
    let handle = window.accessibility_node("button").unwrap();
    let host = window.state.host.borrow_mut().take();
    assert!(bounds(window.hwnd, &handle).is_err());
    *window.state.host.borrow_mut() = host;
    let foreign = crate::surface_accessibility::Owner::new(&scene());
    assert!(bounds(window.hwnd, &foreign.node("button").unwrap()).is_err());
    window.replace(scene()).unwrap();
    assert!(bounds(window.hwnd, &handle).is_err());
    assert!(bounds(window.hwnd, &window.accessibility_node("button").unwrap()).is_ok());
}

#[test]
fn accessibility_bridge_reentrant_close_does_not_restore_host_or_retain_state() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = SurfaceWindow::new(scene(), 240, 200).unwrap();
    let weak = Rc::downgrade(&window.state);
    let handle = window.accessibility_node("button").unwrap();
    let result = query(window.hwnd, &handle, |_host| {
        window.close()?;
        Ok(())
    });
    assert!(result.is_err());
    assert!(window.state.host.borrow().is_none());
    assert_eq!(window.state.accessibility_key.get(), 0);
    assert!(!WINDOWS.with(|windows| windows.borrow().contains_key(&(window.hwnd.0 as usize))));
    drop(result);
    drop(window);
    assert!(weak.upgrade().is_none());
}

#[test]
fn accessibility_registry_unregister_does_not_remove_another_state_identity() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let first = SurfaceWindow::new(scene(), 240, 200).unwrap();
    let second = SurfaceWindow::new(scene(), 240, 200).unwrap();
    let key = first.hwnd.0 as usize;
    WINDOWS.with(|windows| {
        windows
            .borrow_mut()
            .insert(key, Rc::downgrade(&second.state))
    });
    first.close().unwrap();
    assert!(WINDOWS.with(|windows| windows
        .borrow()
        .get(&key)
        .is_some_and(|entry| std::ptr::eq(entry.as_ptr(), &*second.state))));
    WINDOWS.with(|windows| windows.borrow_mut().remove(&key));
}
