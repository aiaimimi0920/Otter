//! 真 EDIT 通知→原草稿/FIFO/fence→同 child；后续输入不被前帧/旧 ticket 覆盖。
use super::*;
use windows::{
    core::HSTRING,
    Win32::UI::{Controls::*, Input::KeyboardAndMouse::IsWindowEnabled},
};
fn child(window: &SurfaceWindow) -> HWND {
    unsafe { FindWindowExW(Some(window.hwnd), None, windows::core::w!("EDIT"), None).unwrap() }
}
fn text(hwnd: HWND) -> String {
    let mut buffer = [0u16; 256];
    let n = unsafe { GetWindowTextW(hwnd, &mut buffer) };
    String::from_utf16(&buffer[..n as usize]).unwrap()
}
fn type_text(hwnd: HWND, value: &str) {
    unsafe {
        SendMessageW(hwnd, EM_SETSEL, Some(WPARAM(0)), Some(LPARAM(-1)));
        let value = HSTRING::from(value);
        SendMessageW(
            hwnd,
            EM_REPLACESEL,
            Some(WPARAM(1)),
            Some(LPARAM(value.as_ptr() as isize)),
        );
    }
}
#[test]
fn javascript_native_input_real_hwnd_preserves_prefix_click_later_draft_and_settlement() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let _com = SurfaceWindow::javascript_apartment_for_test();
    let binding = fixture::binding();
    let (mut session, frame) = Session::mount(
        binding.clone(),
        crate::surface_javascript::input_tests::SOURCE.into(),
        fixture::snapshot(3, 42),
        fixture::resources(),
        &|| false,
    )
    .unwrap();
    let button = frame.scene.root().children()[2].id().to_owned();
    let window = SurfaceWindow::new(frame.scene, 320, 200).unwrap();
    window.bind_edits(&binding).unwrap();
    window.set_input_policy(true, true).unwrap();
    let edit = child(&window);
    assert_eq!(text(edit), "初始🙂");
    type_text(edit, "prefix🙂");
    unsafe {
        SendMessageW(
            window.hwnd,
            crate::desktop::surface_edit_messages::COMMIT,
            None,
            Some(LPARAM(edit.0 as isize)),
        );
    }
    window.javascript_queue_click_for_test(&button);
    let pending = window
        .defer_click(window.take_click().unwrap(), &binding, 3)
        .unwrap();
    type_text(edit, "later🙂");
    for expected in ["input|prefix🙂|1", "change|prefix🙂|2"] {
        let prepared = window
            .prepare_javascript_edit(&binding, Some(&pending))
            .unwrap()
            .unwrap();
        window.set_submission_enabled(false).unwrap();
        assert!(unsafe { IsWindowEnabled(edit) }.as_bool());
        let next = session.local_input(prepared.input, &|| false).unwrap();
        assert_eq!(fixture::text(&next.scene), expected);
        window.replace(next.scene).unwrap();
        window.settle_javascript_edit(&prepared.id).unwrap();
        assert_eq!(child(&window), edit);
        assert_eq!(text(edit), "later🙂");
        window.set_submission_enabled(true).unwrap();
    }
    assert_eq!(
        window.deferred_state(&pending),
        crate::desktop::surface_edit_mount::FenceState::Ready
    );
    assert!(window
        .prepare_javascript_edit(&binding, Some(&pending))
        .unwrap()
        .is_none());
    assert!(window
        .prepare_deferred(&pending, &binding, 3)
        .unwrap()
        .is_empty());
    let next = session.click(pending.target(), &|| false).unwrap();
    assert_eq!(fixture::text(&next.scene), "click|prefix🙂|2");
    window.replace(next.scene).unwrap();
    assert_eq!(text(edit), "later🙂");
    let prepared = window
        .prepare_javascript_edit(&binding, None)
        .unwrap()
        .unwrap();
    let next = session.local_input(prepared.input, &|| false).unwrap();
    assert_eq!(fixture::text(&next.scene), "input|later🙂|3");
    window.replace(next.scene).unwrap();
    window.settle_javascript_edit(&prepared.id).unwrap();
    assert!(!window.has_pending_edit());
    assert_eq!(text(edit), "later🙂");
    session.dispose(&|| false).unwrap();
    window.close().unwrap();
    assert!(!unsafe { IsWindow(Some(edit)) }.as_bool());
    assert!(window.take_error().is_none());
}
#[test]
fn javascript_native_input_real_hwnd_ime_waits_for_composition_end() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let _com = SurfaceWindow::javascript_apartment_for_test();
    let binding = fixture::binding();
    let (mut session, frame) = Session::mount(
        binding.clone(),
        crate::surface_javascript::input_tests::SOURCE.into(),
        fixture::snapshot(3, 42),
        fixture::resources(),
        &|| false,
    )
    .unwrap();
    let window = SurfaceWindow::new(frame.scene, 320, 200).unwrap();
    window.bind_edits(&binding).unwrap();
    window.set_input_policy(true, true).unwrap();
    let edit = child(&window);
    unsafe {
        SendMessageW(edit, WM_IME_STARTCOMPOSITION, None, None);
    }
    type_text(edit, "输入法🙂");
    assert!(!window.has_pending_edit());
    assert!(window
        .prepare_javascript_edit(&binding, None)
        .unwrap()
        .is_none());
    unsafe {
        SendMessageW(edit, WM_IME_ENDCOMPOSITION, None, None);
    }
    let prepared = window
        .prepare_javascript_edit(&binding, None)
        .unwrap()
        .unwrap();
    let frame = session.local_input(prepared.input, &|| false).unwrap();
    assert_eq!(fixture::text(&frame.scene), "input|输入法🙂|1");
    window.replace(frame.scene).unwrap();
    window.settle_javascript_edit(&prepared.id).unwrap();
    assert_eq!(text(edit), "输入法🙂");
    session.dispose(&|| false).unwrap();
    window.close().unwrap();
}
