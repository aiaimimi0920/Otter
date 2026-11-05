//! 隐藏 HWND 上实际执行 EDIT 写入/选择/替换及父子销毁，不修改系统剪贴板。
use super::*;
#[path = "surface_edit_placeholder_tests.rs"]
mod placeholder_tests;
use crate::{desktop::surface_window::SurfaceWindow, surface_scene::Scene};
#[test]
fn surface_edit_control_value_encodes_input_and_retains_pending_draft() {
    use crate::{
        surface_input_draft::Draft,
        surface_input_event::{self, Phase, Value},
        surface_live_binding::Binding,
    };
    let _apartment = crate::desktop::Apartment::new().unwrap();
    let parent = parent();
    let edit = Control::new(parent.dialog_owner().unwrap(), rect(), false, 100).unwrap();
    let node = Scene::parse(br#"{"id":"field","type":"input","events":{"input":"save"}}"#).unwrap();
    let binding = Binding::from_mount(br#"{"instance":{"descriptor":{"instanceId":"instance:test","generation":1},"attachments":{"attachment:test":{"descriptor":{"attachmentId":"attachment:test","instanceId":"instance:test","hookNodeId":"node:test","deviceId":"viewer:test"},"lifecycle":"active","lifecycleRevision":1}}}}"#, "https://localhost:1234", "viewer:test", "node:test").unwrap();
    let mut draft = Draft::new("source").unwrap();
    edit.set_text("输入🙂").unwrap();
    draft.edit(&edit.text().unwrap()).unwrap();
    let ticket = draft.begin().unwrap();
    let event = surface_input_event::encode(
        node.root(),
        &binding,
        9,
        Phase::Input,
        Value::Text(draft.value()),
    )
    .unwrap()
    .unwrap();
    let wire: serde_json::Value = serde_json::from_slice(event.bytes()).unwrap();
    assert_eq!(wire["payload"]["value"], "输入🙂");
    assert_eq!(wire["class"], "continuous");
    draft.synchronize("server result").unwrap();
    assert_eq!(draft.value(), edit.text().unwrap());
    assert!(draft.settle(ticket));
    edit.set_text(draft.value()).unwrap();
    assert_eq!(edit.text().unwrap(), "server result");
    assert!(!event.attempted());
}
use windows::Win32::UI::Controls::{EM_REPLACESEL, EM_SETSEL};
fn parent() -> SurfaceWindow {
    SurfaceWindow::new(
        Scene::parse(br#"{"id":"root","type":"view"}"#).unwrap(),
        240,
        160,
    )
    .unwrap()
}
fn rect() -> RECT {
    RECT {
        left: 4,
        top: 4,
        right: 204,
        bottom: 104,
    }
}
#[test]
fn surface_edit_control_real_unicode_selection_and_replacement() {
    let _apartment = crate::desktop::Apartment::new().unwrap();
    let parent = parent();
    let edit = Control::new(parent.dialog_owner().unwrap(), rect(), false, 100).unwrap();
    edit.set_text("中文🙂 test").unwrap();
    assert_eq!(edit.text().unwrap(), "中文🙂 test");
    unsafe {
        let hwnd = edit.hwnd().unwrap();
        SendMessageW(hwnd, EM_SETSEL, Some(WPARAM(0)), Some(LPARAM(2)));
        let replacement = HSTRING::from("输入");
        SendMessageW(
            hwnd,
            EM_REPLACESEL,
            Some(WPARAM(1)),
            Some(LPARAM(replacement.as_ptr() as isize)),
        );
        assert_eq!(edit.text().unwrap(), "输入🙂 test");
    }
    edit.place(RECT {
        left: 8,
        top: 9,
        right: 200,
        bottom: 50,
    })
    .unwrap();
    let hwnd = edit.hwnd().unwrap();
    drop(edit);
    assert!(!unsafe { IsWindow(Some(hwnd)) }.as_bool());
}
#[test]
fn surface_edit_control_parent_destroy_invalidates_owner_and_reads() {
    let _apartment = crate::desktop::Apartment::new().unwrap();
    let parent = parent();
    let edit = Control::new(parent.dialog_owner().unwrap(), rect(), true, 100).unwrap();
    edit.set_text("第一行\r\n第二行🙂").unwrap();
    assert_eq!(edit.text().unwrap(), "第一行\r\n第二行🙂");
    parent.close().unwrap();
    assert!(!edit.is_alive());
    assert!(edit.text().is_err());
    assert!(edit.set_text("closed").is_err());
}
#[test]
fn surface_edit_control_rejects_programmatic_limit_bypass_and_bad_bounds() {
    let _apartment = crate::desktop::Apartment::new().unwrap();
    let parent = parent();
    let hwnd = parent.dialog_owner().unwrap();
    let edit = Control::new(hwnd, rect(), false, 4).unwrap();
    edit.set_text("🙂🙂").unwrap();
    for value in ["🙂🙂a", "a\0b", "a\nb"] {
        assert!(edit.set_text(value).is_err());
    }
    assert_eq!(edit.text().unwrap(), "🙂🙂");
    unsafe {
        SetWindowTextW(edit.hwnd().unwrap(), &HSTRING::from("overflow")).unwrap();
    }
    assert!(edit.text().is_err());
    assert!(Control::new(hwnd, rect(), false, 0).is_err());
    assert!(Control::new(HWND::default(), rect(), false, 10).is_err());
    assert!(Control::new(
        hwnd,
        RECT {
            left: i32::MIN,
            right: i32::MAX,
            ..rect()
        },
        false,
        10
    )
    .is_err());
}
#[test]
fn surface_edit_control_byte_limit_is_independent_of_utf16_units() {
    let _apartment = crate::desktop::Apartment::new().unwrap();
    let parent = parent();
    let edit = Control::new(parent.dialog_owner().unwrap(), rect(), true, MAX_UNITS).unwrap();
    let allowed = "中".repeat(MAX_BYTES / 3);
    edit.set_text(&allowed).unwrap();
    assert_eq!(edit.text().unwrap(), allowed);
    let oversized = format!("{allowed}中");
    assert!(edit.set_text(&oversized).is_err());
    unsafe {
        SetWindowTextW(edit.hwnd().unwrap(), &HSTRING::from(oversized)).unwrap();
    }
    assert!(edit.text().is_err());
}
