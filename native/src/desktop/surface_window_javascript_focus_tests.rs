//! 真实 EDIT 切焦/失焦提交；窗口在屏外，保存并恢复前台，不注入全局按键或鼠标。
use super::*;
use crate::{surface_javascript::focus_tests, surface_javascript_wire::focus::Request};
use windows::{
    core::HSTRING,
    Win32::UI::{Controls::*, Input::KeyboardAndMouse::*},
};
#[test]
fn javascript_native_focus_real_hwnd_blur_enqueues_original_change_once() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let _com = SurfaceWindow::javascript_apartment_for_test();
    let binding = fixture::binding();
    let (mut session, frame) = Session::mount(
        binding.clone(),
        focus_tests::SOURCE.into(),
        fixture::snapshot(3, 42),
        fixture::resources(),
        &|| false,
    )
    .unwrap();
    let field = frame.scene.root().children()[1].id().to_owned();
    let focus = frame.scene.root().children()[2].id().to_owned();
    let blur = frame.scene.root().children()[3].id().to_owned();
    let window = SurfaceWindow::new(frame.scene, 320, 200).unwrap();
    window.bind_edits(&binding).unwrap();
    window.set_input_policy(true, true).unwrap();
    let _foreground = unsafe { fixture::focus_foreground(window.hwnd) };
    let epoch = window.javascript_focus_epoch();
    let frame = session.click(&focus, &|| false).unwrap();
    let requests = frame.focus;
    window.replace(frame.scene).unwrap();
    window.apply_javascript_focus(&requests, epoch).unwrap();
    let edit = window
        .state
        .host
        .borrow()
        .as_ref()
        .unwrap()
        .accessibility_edit_hwnd(&field)
        .unwrap();
    assert_eq!(unsafe { GetFocus() }, edit);
    unsafe {
        SendMessageW(edit, EM_SETSEL, Some(WPARAM(0)), Some(LPARAM(-1)));
        let value = HSTRING::from("焦点🙂");
        SendMessageW(
            edit,
            EM_REPLACESEL,
            Some(WPARAM(1)),
            Some(LPARAM(value.as_ptr() as isize)),
        );
    }
    let prepared = window
        .prepare_javascript_edit(&binding, None)
        .unwrap()
        .unwrap();
    let epoch = window.javascript_focus_epoch();
    let frame = session.local_input(prepared.input, &|| false).unwrap();
    assert!(frame.events.is_empty());
    let requests = frame.focus;
    window.replace(frame.scene).unwrap();
    window.settle_javascript_edit(&prepared.id).unwrap();
    window.apply_javascript_focus(&requests, epoch).unwrap();
    assert_eq!(unsafe { GetFocus() }, window.hwnd);
    let epoch = window.javascript_focus_epoch();
    let frame = session.click(&blur, &|| false).unwrap();
    assert!(frame.events.is_empty());
    let requests = frame.focus;
    window.replace(frame.scene).unwrap();
    window.apply_javascript_focus(&requests, epoch).unwrap();
    assert_eq!(unsafe { GetFocus() }, window.hwnd);
    let prepared = window
        .prepare_javascript_edit(&binding, None)
        .unwrap()
        .unwrap();
    assert!(matches!(
        prepared.input,
        crate::surface_javascript_wire::LocalInput::Text(
            crate::surface_javascript_wire::TextInput {
                phase: crate::surface_javascript_wire::InputPhase::Change,
                ..
            }
        )
    ));
    let frame = session.local_input(prepared.input, &|| false).unwrap();
    assert_eq!(fixture::text(&frame.scene), "change|焦点🙂");
    assert_eq!(frame.events.len(), 2);
    window.replace(frame.scene).unwrap();
    window.settle_javascript_edit(&prepared.id).unwrap();
    window
        .apply_javascript_focus(&requests, window.javascript_focus_epoch())
        .unwrap();
    assert!(!window.has_pending_edit());
    session.dispose(&|| false).unwrap();
    window.close().unwrap();
    assert!(!unsafe { IsWindow(Some(edit)) }.as_bool());
    assert!(window.take_error().is_none());
}
#[test]
fn javascript_native_focus_real_hwnd_rejects_stale_aba_policy_hidden_and_foreign_targets() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let _com = SurfaceWindow::javascript_apartment_for_test();
    let scene = crate::surface_scene::Scene::parse(br#"{"id":"root","type":"column","children":[{"id":"field","type":"input"},{"id":"disabled","type":"input","props":{"disabled":true}}]}"#).unwrap();
    let window = SurfaceWindow::new(scene, 320, 200).unwrap();
    window.bind_edits(&fixture::binding()).unwrap();
    window.set_input_policy(true, true).unwrap();
    let _foreground = unsafe { fixture::focus_foreground(window.hwnd) };
    let edit = window
        .state
        .host
        .borrow()
        .as_ref()
        .unwrap()
        .accessibility_edit_hwnd("field")
        .unwrap();
    let requests = [Request {
        target: "field".into(),
        focus: true,
    }];
    let stale = window.javascript_focus_epoch();
    unsafe {
        let _ = SetFocus(Some(edit));
        let _ = SetFocus(Some(window.hwnd));
    }
    assert!(window.javascript_focus_epoch() > stale);
    window.apply_javascript_focus(&requests, stale).unwrap();
    assert_eq!(unsafe { GetFocus() }, window.hwnd);
    for target in ["missing", "root", "disabled"] {
        window
            .apply_javascript_focus(
                &[Request {
                    target: target.into(),
                    focus: true,
                }],
                window.javascript_focus_epoch(),
            )
            .unwrap();
        assert_eq!(unsafe { GetFocus() }, window.hwnd);
    }
    let other = SurfaceWindow::new(
        crate::surface_scene::Scene::parse(br#"{"id":"other","type":"column"}"#).unwrap(),
        200,
        100,
    )
    .unwrap();
    {
        let _other_foreground = unsafe { fixture::focus_foreground(other.hwnd) };
        let epoch = window.javascript_focus_epoch();
        window.apply_javascript_focus(&requests, epoch).unwrap();
        assert_eq!(unsafe { GetForegroundWindow() }, other.hwnd);
        assert_eq!(unsafe { GetFocus() }, other.hwnd);
        other.close().unwrap();
    }
    unsafe {
        let _ = SetFocus(Some(window.hwnd));
    }
    window.set_input_policy(false, false).unwrap();
    window
        .apply_javascript_focus(&requests, window.javascript_focus_epoch())
        .unwrap();
    assert_ne!(unsafe { GetFocus() }, edit);
    window.set_input_policy(true, true).unwrap();
    unsafe {
        let _ = ShowWindow(window.hwnd, SW_HIDE);
    }
    window
        .apply_javascript_focus(&requests, window.javascript_focus_epoch())
        .unwrap();
    assert_ne!(unsafe { GetFocus() }, edit);
    window.close().unwrap();
    window
        .apply_javascript_focus(&requests, window.javascript_focus_epoch())
        .unwrap();
    assert!(window.take_error().is_none());
}

#[test]
#[ignore = "explicit cross-process foreground check; creates only its own bounded fixture"]
fn javascript_native_focus_thread_focus_alone_cannot_authorize_foreground_or_replay() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let _com = SurfaceWindow::javascript_apartment_for_test();
    let scene = Scene::parse(
        br#"{"id":"root","type":"column","children":[{"id":"field","type":"input"}]}"#,
    )
    .unwrap();
    let window = SurfaceWindow::new(scene, 320, 200).unwrap();
    window.bind_edits(&fixture::binding()).unwrap();
    window.set_input_policy(true, true).unwrap();
    window.enable_javascript_keyboard();
    let _foreground = unsafe { fixture::focus_foreground(window.hwnd) };
    // 同线程控制实验一并激活了窗口；借原有进程 owner 隔离线程焦点与真正前台。
    let mut other = uia::ProcessFixture::start_foreground();
    assert_ne!(other.pid(), std::process::id());
    {
        // 父线程的立即检查不能证明子线程已完成激活；只由 Child 本地激活一次并确认。
        let _other_foreground = unsafe { fixture::guard_foreground(other.hwnd) };
        assert_eq!(
            unsafe { GetForegroundWindow() },
            window.hwnd,
            "foreground fixture lost parent before child request"
        );
        other.foreground();
        unsafe {
            let _ = SetFocus(Some(window.hwnd));
        }
        assert_eq!(unsafe { GetFocus() }, window.hwnd);
        assert_eq!(
            unsafe { GetForegroundWindow() },
            other.hwnd,
            "thread focus must not grant foreground"
        );
        assert!(!window.javascript_focus_allowed());
        let epoch = window.javascript_focus_epoch();
        window
            .apply_javascript_focus(
                &[Request {
                    target: "field".into(),
                    focus: true,
                }],
                epoch,
            )
            .unwrap();
        assert_eq!(window.javascript_focus_epoch(), epoch);
        assert_eq!(unsafe { GetFocus() }, window.hwnd);
        assert_eq!(unsafe { GetForegroundWindow() }, other.hwnd);
        assert!(!window.has_pending_edit());
        // AllowSetForegroundWindow 的下一次授权会替换前一接收方；必须由当前前台 Child
        // 明确归还父进程许可，不能拿父线程 SetFocus 或 AttachThreadInput 冒充前台权限。
        other.return_foreground();
    }
    let _restored_foreground = unsafe { fixture::focus_foreground(window.hwnd) };
    // 后续普通空请求帧不能恢复已拒绝的 focus，且不能生成编辑候选。
    window
        .replace(
            Scene::parse(
                br#"{"id":"root","type":"column","children":[{"id":"field","type":"input"}]}"#,
            )
            .unwrap(),
        )
        .unwrap();
    window
        .apply_javascript_focus(&[], window.javascript_focus_epoch())
        .unwrap();
    assert_eq!(unsafe { GetFocus() }, window.hwnd);
    assert!(!window.has_pending_edit());
    other.close();
    window.close().unwrap();
    assert!(window.take_error().is_none());
}
