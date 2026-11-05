use super::*;
fn fixture() -> SurfaceWindow {
    let scene = Scene::parse(br#"{"id":"text","type":"text","props":{"text":"Copy selection","selectable":true},"layout":{"width":200,"height":40}}"#).unwrap();
    let window = SurfaceWindow::new(scene, 240, 120).unwrap();
    window.set_pointer_enabled(true).unwrap();
    unsafe {
        keyboard::focus_node(window.hwnd, &window.state, "text");
    }
    window
        .state
        .run(|host| {
            host.begin_text_selection([1, 8])?;
            host.update_text_selection([210, 8], true)?;
            Ok(())
        })
        .unwrap();
    window
}
#[test]
fn text_selection_copy_uses_current_text_and_rechecks_before_publication() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = fixture();
    let mut calls = 0;
    unsafe {
        assert!(copy_with(window.hwnd, &window.state, |text, current| {
            calls += 1;
            assert_eq!(text, "Copy selection");
            assert!(current());
            Ok(())
        }));
    }
    assert_eq!(calls, 1);
    assert!(!window.has_pending_click());
    assert!(!window.has_pending_edit());
    unsafe {
        assert!(copy_with(window.hwnd, &window.state, |_, current| {
            window.set_input_policy(false, false).unwrap();
            assert!(!current());
            Ok(())
        }));
    }
    unsafe {
        assert!(!copy_with(window.hwnd, &window.state, |_, _| panic!(
            "revoked copy"
        )));
    }
}
#[test]
fn text_selection_copy_empty_and_foreign_focus_do_not_publish_and_failures_surface() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = fixture();
    unsafe {
        assert!(copy_with(window.hwnd, &window.state, |_, _| Err(
            Error::new(E_FAIL, "test publish failure")
        )));
    }
    assert!(window.take_error().is_some());
    window
        .state
        .run(|host| {
            host.cancel_text_selection(true);
            Ok(())
        })
        .unwrap();
    unsafe {
        assert!(!copy_with(window.hwnd, &window.state, |_, _| panic!(
            "empty copy"
        )));
    }
    let other = fixture();
    unsafe {
        assert!(!copy_with(window.hwnd, &window.state, |_, _| panic!(
            "foreign focus copy"
        )));
    }
    drop(other);
}
