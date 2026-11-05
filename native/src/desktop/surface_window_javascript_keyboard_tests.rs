//! 真实 EDIT 定向消息验证取消先于默认 change；保留草稿、焦点 ABA 和原唯一 FIFO。
use super::*;
use crate::surface_javascript::{keyboard_tests::SOURCE, Frame};
#[test]
fn javascript_native_enter_message_identity_does_not_alias_surface_notifications_or_controls() {
    use crate::desktop::{surface_edit_messages as messages, surface_slider_control as slider};
    let ids = [
        messages::ENTER_KEY,
        messages::COMPOSITION_START,
        messages::COMPOSITION_END,
        messages::NAVIGATE,
        messages::COMMIT,
        messages::EDIT_BEGIN,
        messages::EDIT_END,
        messages::STEP_UP,
        messages::STEP_DOWN,
        messages::CAN_PASTE,
        notifications::MESSAGE,
        focus_notifications::MESSAGE,
        structure_notifications::MESSAGE,
        slider::NOTIFY,
        slider::CLICK_BEGIN,
        slider::CLICK_CANCEL,
        slider::CLICK_END,
    ];
    assert_eq!(
        ids.iter().collect::<std::collections::BTreeSet<_>>().len(),
        ids.len()
    );
}
use windows::{
    core::HSTRING,
    Win32::UI::{Controls::*, Input::KeyboardAndMouse::*},
};
fn edit(window: &SurfaceWindow, node: &str) -> HWND {
    window
        .state
        .host
        .borrow()
        .as_ref()
        .unwrap()
        .accessibility_edit_hwnd(node)
        .unwrap()
}
unsafe fn type_text(hwnd: HWND, value: &str) {
    SendMessageW(hwnd, EM_SETSEL, Some(WPARAM(0)), Some(LPARAM(-1)));
    let value = HSTRING::from(value);
    SendMessageW(
        hwnd,
        EM_REPLACESEL,
        Some(WPARAM(1)),
        Some(LPARAM(value.as_ptr() as isize)),
    );
}
unsafe fn key(hwnd: HWND, up: bool, repeat: bool) {
    SendMessageW(
        hwnd,
        if up { WM_KEYUP } else { WM_KEYDOWN },
        Some(WPARAM(VK_RETURN.0 as usize)),
        Some(LPARAM(if repeat { 1 << 30 } else { 0 })),
    );
}
fn accept(window: &SurfaceWindow, id: &str, frame: Frame) -> (String, usize) {
    let text = fixture::text(&frame.scene).to_owned();
    let count = frame.events.len();
    let epoch = window.javascript_focus_epoch();
    window.replace(frame.scene).unwrap();
    window.settle_javascript_ticket(id).unwrap();
    window.apply_javascript_focus(&frame.focus, epoch).unwrap();
    window
        .complete_javascript_edit(id, frame.default_allowed)
        .unwrap();
    (text, count)
}
fn next(window: &SurfaceWindow, session: &mut Session) -> (String, usize) {
    let prepared = window
        .prepare_javascript_edit(&fixture::binding(), None)
        .unwrap()
        .unwrap();
    let frame = session.local_input(prepared.input, &|| false).unwrap();
    accept(window, &prepared.id, frame)
}
#[test]
fn javascript_native_enter_real_hwnd_cancel_default_fifo_repeat_and_later_draft() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let _com = SurfaceWindow::javascript_apartment_for_test();
    let (mut session, frame) = Session::mount(
        fixture::binding(),
        SOURCE.into(),
        fixture::snapshot(3, 42),
        fixture::resources(),
        &|| false,
    )
    .unwrap();
    let node = frame.scene.root().children()[1].id().to_owned();
    let window = SurfaceWindow::new(frame.scene, 320, 200).unwrap();
    window.enable_javascript_keyboard();
    window.bind_edits(&fixture::binding()).unwrap();
    window.set_input_policy(true, true).unwrap();
    let _foreground = unsafe { fixture::focus_foreground(window.hwnd) };
    let hwnd = edit(&window, &node);
    unsafe {
        let _ = SetFocus(Some(hwnd));
        type_text(hwnd, "prefix🙂");
        key(hwnd, false, false);
        key(hwnd, true, false);
    }
    assert_eq!(next(&window, &mut session), ("input:prefix🙂".into(), 0));
    assert_eq!(
        next(&window, &mut session),
        ("input:prefix🙂>down:cancel:first".into(), 0)
    );
    assert_eq!(
        next(&window, &mut session),
        ("input:prefix🙂>down:cancel:first>up".into(), 0)
    );
    assert!(!window.has_pending_edit());
    unsafe {
        key(hwnd, false, false);
        key(hwnd, false, true);
        key(hwnd, true, false);
        type_text(hwnd, "later🙂");
    }
    let (text, count) = next(&window, &mut session);
    assert!(text.ends_with("down:allow:first"));
    assert_eq!(count, 0);
    let (text, count) = next(&window, &mut session);
    assert!(text.ends_with("down:allow:first>change:prefix🙂"));
    assert_eq!(count, 2);
    let (text, count) = next(&window, &mut session);
    assert!(text.ends_with("change:prefix🙂>down:allow:repeat"));
    assert_eq!(count, 0);
    assert!(next(&window, &mut session)
        .0
        .ends_with("down:allow:repeat>up"));
    assert!(next(&window, &mut session).0.ends_with("up>input:later🙂"));
    assert!(!window.has_pending_edit());
    assert_eq!(edit(&window, &node), hwnd);
    assert_eq!(
        window
            .state
            .host
            .borrow_mut()
            .as_mut()
            .unwrap()
            .capture_form_values()[&node],
        "later🙂"
    );
    session.dispose(&|| false).unwrap();
    window.close().unwrap();
    assert!(!unsafe { IsWindow(Some(hwnd)) }.as_bool());
    assert!(window.take_error().is_none());
}
#[test]
fn javascript_native_enter_real_hwnd_stale_focus_policy_contract_and_hidden_do_not_replay_default()
{
    let _serial = fixture::SERIAL.lock().unwrap();
    let _com = SurfaceWindow::javascript_apartment_for_test();
    for mode in 0..4 {
        let source = SOURCE.replace("attempts = 0", "attempts = 1");
        let (mut session, frame) = Session::mount(
            fixture::binding(),
            source,
            fixture::snapshot(3, 42),
            fixture::resources(),
            &|| false,
        )
        .unwrap();
        let node = frame.scene.root().children()[1].id().to_owned();
        let window = SurfaceWindow::new(frame.scene, 320, 200).unwrap();
        window.enable_javascript_keyboard();
        window.bind_edits(&fixture::binding()).unwrap();
        window.set_input_policy(true, true).unwrap();
        let _foreground = unsafe { fixture::focus_foreground(window.hwnd) };
        let hwnd = edit(&window, &node);
        unsafe {
            let _ = SetFocus(Some(hwnd));
            type_text(hwnd, "dirty");
        }
        next(&window, &mut session);
        unsafe {
            key(hwnd, false, false);
        }
        let prepared = window
            .prepare_javascript_edit(&fixture::binding(), None)
            .unwrap()
            .unwrap();
        let frame = session.local_input(prepared.input, &|| false).unwrap();
        assert_eq!(frame.default_allowed, Some(true));
        let captured_epoch = window.javascript_focus_epoch();
        match mode {
            0 => unsafe {
                let _ = SetFocus(Some(window.hwnd));
                let _ = SetFocus(Some(hwnd));
            },
            1 => {
                window.set_input_policy(false, false).unwrap();
                window.set_input_policy(true, true).unwrap();
            }
            2 => {
                window
                    .replace(
                        crate::surface_scene::Scene::parse(br#"{"id":"root","type":"column"}"#)
                            .unwrap(),
                    )
                    .unwrap();
            }
            _ => unsafe {
                let _ = ShowWindow(window.hwnd, SW_HIDE);
            },
        }
        let native_focus_changed = window.javascript_focus_epoch() != captured_epoch;
        accept(&window, &prepared.id, frame);
        // ABA 的真实失焦可已有一个 change；只核新 default 槽没有被补入。
        let mut changes = 0;
        while window.has_pending_edit() {
            let (_, count) = next(&window, &mut session);
            if count != 0 {
                changes += 1;
            }
        }
        assert!(
            changes <= usize::from(native_focus_changed),
            "mode {mode}: unexpected default change count {changes}"
        );
        window.set_input_policy(true, true).unwrap();
        assert!(!window.has_pending_edit());
        session.dispose(&|| false).unwrap();
        window.close().unwrap();
        assert!(window.take_error().is_none());
    }
}
#[test]
fn javascript_native_enter_real_hwnd_default_keeps_listener_value_and_commits_dirty_epoch_once() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let _com = SurfaceWindow::javascript_apartment_for_test();
    let source = SOURCE.replace(
        "if (++attempts === 1) event.preventDefault();",
        "field.value = 'listener🙂';",
    );
    let (mut session, frame) = Session::mount(
        fixture::binding(),
        source,
        fixture::snapshot(3, 42),
        fixture::resources(),
        &|| false,
    )
    .unwrap();
    let node = frame.scene.root().children()[1].id().to_owned();
    let window = SurfaceWindow::new(frame.scene, 320, 200).unwrap();
    window.enable_javascript_keyboard();
    window.bind_edits(&fixture::binding()).unwrap();
    window.set_input_policy(true, true).unwrap();
    let _foreground = unsafe { fixture::focus_foreground(window.hwnd) };
    let hwnd = edit(&window, &node);
    unsafe {
        let _ = SetFocus(Some(hwnd));
        type_text(hwnd, "dirty");
    }
    next(&window, &mut session);
    unsafe {
        key(hwnd, false, false);
        key(hwnd, true, false);
        key(hwnd, false, false);
        key(hwnd, true, false);
    }
    assert_eq!(next(&window, &mut session).1, 0);
    let (label, count) = next(&window, &mut session);
    assert!(label.ends_with("change:listener🙂"));
    assert_eq!(count, 2);
    for _ in 0..3 {
        assert_eq!(next(&window, &mut session).1, 0);
    }
    assert!(!window.has_pending_edit());
    let mut buffer = [0u16; 64];
    let n = unsafe { GetWindowTextW(hwnd, &mut buffer) };
    assert_eq!(
        String::from_utf16(&buffer[..n as usize]).unwrap(),
        "listener🙂"
    );
    session.dispose(&|| false).unwrap();
    window.close().unwrap();
    assert!(window.take_error().is_none());
}
#[test]
fn javascript_native_enter_real_hwnd_background_edges_are_not_queued_or_replayed() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let _com = SurfaceWindow::javascript_apartment_for_test();
    let (mut session, frame) = Session::mount(
        fixture::binding(),
        SOURCE.into(),
        fixture::snapshot(3, 42),
        fixture::resources(),
        &|| false,
    )
    .unwrap();
    let node = frame.scene.root().children()[1].id().to_owned();
    let window = SurfaceWindow::new(frame.scene, 320, 200).unwrap();
    window.enable_javascript_keyboard();
    window.bind_edits(&fixture::binding()).unwrap();
    window.set_input_policy(true, true).unwrap();
    let _foreground = unsafe { fixture::focus_foreground(window.hwnd) };
    let hwnd = edit(&window, &node);
    let other = SurfaceWindow::new(
        crate::surface_scene::Scene::parse(br#"{"id":"root","type":"column"}"#).unwrap(),
        120,
        80,
    )
    .unwrap();
    unsafe {
        let _ = SetFocus(Some(hwnd));
        {
            let _background = fixture::focus_foreground(other.hwnd);
            assert_ne!(GetForegroundWindow(), window.hwnd);
            assert!(IsWindowEnabled(hwnd).as_bool() && IsWindowVisible(hwnd).as_bool());
            // 定向消息不等于真实键盘资格；后台边沿必须被消费且不能进入原 FIFO。
            key(hwnd, false, false);
            key(hwnd, true, false);
            assert!(!window.has_pending_edit());
        }
        let _ = SetFocus(Some(hwnd));
        assert_eq!(GetForegroundWindow(), window.hwnd);
        assert!(!window.has_pending_edit());
        key(hwnd, true, false);
    }
    assert_eq!(next(&window, &mut session), ("up".into(), 0));
    assert!(!window.has_pending_edit());
    session.dispose(&|| false).unwrap();
    other.close().unwrap();
    window.close().unwrap();
    assert!(window.take_error().is_none() && other.take_error().is_none());
}
