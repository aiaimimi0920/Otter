//! 真实 HWND 的生产挂载/通知/准备路径；不读取剪贴板、不注入用户桌面输入。
use super::*;
#[path = "surface_edit_action_tests.rs"]
mod actions;
#[path = "surface_edit_alignment_mount_tests.rs"]
mod alignment;
#[path = "surface_edit_border_tests.rs"]
mod border;
#[path = "surface_edit_font_tests.rs"]
mod fonts;
#[path = "surface_edit_limit_tests.rs"]
mod limits;
#[path = "surface_edit_navigation_tests.rs"]
mod navigation;
#[path = "surface_number_edit_tests.rs"]
mod number;
#[path = "surface_edit_order_tests.rs"]
mod order;
#[path = "surface_window_session_end_tests.rs"]
mod session_end;
#[path = "surface_textarea_box_tests.rs"]
mod textarea_box;
#[path = "surface_edit_values_tests.rs"]
mod values;
use crate::surface_live_binding::Binding;
use windows::{
    core::HSTRING,
    Win32::UI::{Controls::*, Input::KeyboardAndMouse::*},
};
fn binding() -> Binding {
    Binding::from_mount(br#"{"instance":{"descriptor":{"instanceId":"instance:test","generation":1},"attachments":{"attachment:test":{"descriptor":{"attachmentId":"attachment:test","instanceId":"instance:test","hookNodeId":"node:test","deviceId":"viewer:test"},"lifecycle":"active","lifecycleRevision":1}}}}"#, "https://localhost:1234", "viewer:test", "node:test").unwrap()
}
fn form(value: &str, disabled: bool) -> Scene {
    Scene::parse(
        serde_json::json!({"id":"field","type":"input","props":{"value":value,"disabled":disabled},
        "layout":{"width":180,"height":32},"events":{"input":"typing","change":"commit"}})
        .to_string()
        .as_bytes(),
    )
    .unwrap()
}
fn mount() -> SurfaceWindow {
    let window = SurfaceWindow::new(form("source", false), 240, 160).unwrap();
    window.bind_edits(&binding()).unwrap();
    window.set_input_policy(true, true).unwrap();
    window
}
fn child(window: &SurfaceWindow) -> HWND {
    unsafe { FindWindowExW(Some(window.hwnd), None, windows::core::w!("EDIT"), None).unwrap() }
}
fn text(hwnd: HWND) -> String {
    let mut buffer = [0u16; 256];
    let count = unsafe { GetWindowTextW(hwnd, &mut buffer) };
    String::from_utf16(&buffer[..count as usize]).unwrap()
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
fn surface_edit_mount_real_notification_and_ack_preserve_newer_text() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = mount();
    let edit = child(&window);
    assert_eq!(text(edit), "source");
    type_text(edit, "中文🙂");
    assert!(window.has_pending_edit());
    let first = window.prepare_edit(&binding(), 1).unwrap();
    let wire: serde_json::Value = serde_json::from_slice(first[0].bytes()).unwrap();
    assert_eq!(wire["payload"]["value"], "中文🙂");
    assert_eq!(wire["event"], "input");
    window.set_submission_enabled(false).unwrap();
    assert!(unsafe { IsWindowEnabled(edit) }.as_bool());
    type_text(edit, "newer");
    assert!(window.prepare_edit(&binding(), 1).is_err());
    window.replace(form("remote", false)).unwrap();
    window.settle_edits(&first).unwrap();
    assert_eq!(child(&window), edit);
    assert_eq!(text(edit), "newer");
    window.set_submission_enabled(true).unwrap();
    let second = window.prepare_edit(&binding(), 2).unwrap();
    window.replace(form("accepted", false)).unwrap();
    window.settle_edits(&second).unwrap();
    assert_eq!(text(edit), "accepted");
    assert!(!window.has_pending_edit());
    assert!(window.take_error().is_none());
    window.close().unwrap();
    assert!(!unsafe { IsWindow(Some(edit)) }.as_bool());
}
#[test]
fn surface_edit_mount_composition_messages_and_pause_recovery() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = mount();
    let edit = child(&window);
    unsafe {
        SendMessageW(edit, WM_IME_STARTCOMPOSITION, None, None);
    }
    type_text(edit, "composing");
    assert!(!window.has_pending_edit());
    window.replace(form("remote", false)).unwrap();
    assert_eq!(text(edit), "composing");
    unsafe {
        SendMessageW(edit, WM_IME_ENDCOMPOSITION, None, None);
    }
    assert!(window.has_pending_edit());
    let event = window.prepare_edit(&binding(), 2).unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(event[0].bytes()).unwrap()["payload"]["value"],
        "composing"
    );
    window.cancel_edits(&event);
    unsafe {
        SendMessageW(edit, WM_IME_STARTCOMPOSITION, None, None);
    }
    window.set_input_policy(false, false).unwrap();
    unsafe {
        SendMessageW(edit, WM_IME_ENDCOMPOSITION, None, None);
    }
    window.set_input_policy(true, true).unwrap();
    type_text(edit, "after modal");
    assert!(window.has_pending_edit());
    assert_eq!(window.prepare_edit(&binding(), 2).unwrap().len(), 1);
    assert!(window.take_error().is_none());
}
#[test]
fn surface_edit_mount_cancel_restores_change_phase_and_programmatic_sync_is_silent() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = mount();
    let edit = child(&window);
    window.replace(form("remote", false)).unwrap();
    assert!(!window.has_pending_edit());
    type_text(edit, "local");
    let _input = window.prepare_edit(&binding(), 2).unwrap();
    unsafe {
        SendMessageW(
            window.hwnd,
            WM_COMMAND,
            Some(WPARAM((EN_KILLFOCUS as usize) << 16)),
            Some(LPARAM(edit.0 as isize)),
        );
    }
    let events = window.prepare_edit(&binding(), 2).unwrap();
    assert_eq!(events.len(), 1);
    window.cancel_edits(&events);
    let retry = window.prepare_edit(&binding(), 2).unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(retry[0].bytes()).unwrap()["event"],
        "change"
    );
    assert!(window.take_error().is_none());
}
#[test]
fn surface_edit_mount_disabled_fields_and_removed_nodes_release_controls() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = SurfaceWindow::new(form("disabled", true), 240, 160).unwrap();
    window.bind_edits(&binding()).unwrap();
    window.set_input_policy(true, true).unwrap();
    let edit = child(&window);
    assert!(!unsafe { IsWindowEnabled(edit) }.as_bool());
    window.replace(form("enabled", false)).unwrap();
    let edit = child(&window);
    assert!(unsafe { IsWindowEnabled(edit) }.as_bool());
    assert_eq!(text(edit), "enabled");
    window
        .replace(Scene::parse(br#"{"id":"root","type":"view"}"#).unwrap())
        .unwrap();
    assert!(!unsafe { IsWindow(Some(edit)) }.as_bool());
    assert!(!window.has_pending_edit());
}
#[test]
fn surface_edit_mount_uses_layout_and_ancestor_clip() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let scene = Scene::parse(br#"{"id":"root","type":"view","layout":{"width":100,"height":40,"overflowX":"hidden","overflowY":"hidden"},"children":[{"id":"field","type":"textarea","props":{"rows":4},"layout":{"width":180,"height":80}}]}"#).unwrap();
    let window = SurfaceWindow::new(scene, 240, 160).unwrap();
    window.bind_edits(&binding()).unwrap();
    let edit = child(&window);
    let region = unsafe { CreateRectRgn(0, 0, 0, 0) };
    unsafe {
        assert_ne!(GetWindowRgn(edit, region), GDI_REGION_TYPE(0));
        let mut rect = RECT::default();
        GetRgnBox(region, &mut rect);
        let dpi = windows::Win32::UI::HiDpi::GetDpiForWindow(window.hwnd) as f32 / 96.;
        assert!(rect.right <= (100. * dpi).ceil() as i32);
        assert!(rect.bottom <= (40. * dpi).ceil() as i32);
        let _ = DeleteObject(region.into());
    }
    assert!(window.take_error().is_none());
}
