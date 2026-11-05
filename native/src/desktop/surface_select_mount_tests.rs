//! 真实 Surface→COMBOBOX→WM_COMMAND→共享 FIFO，不直接调用 notify 或发送网络。
use super::*;
#[path = "surface_select_color_tests.rs"]
mod color_tests;
#[path = "surface_select_navigation_tests.rs"]
mod navigation;
#[path = "surface_select_style_tests.rs"]
mod style_tests;
use crate::surface_live_binding::Binding;
use windows::Win32::UI::Input::KeyboardAndMouse::{VK_DOWN, VK_UP};
fn binding() -> Binding {
    Binding::from_mount(br#"{"instance":{"descriptor":{"instanceId":"instance:test","generation":1},"attachments":{"attachment:test":{"descriptor":{"attachmentId":"attachment:test","instanceId":"instance:test","hookNodeId":"node:test","deviceId":"viewer:test"},"lifecycle":"active","lifecycleRevision":1}}}}"#, "https://localhost:1234", "viewer:test", "node:test").unwrap()
}
fn scene(value: &str, action: &str) -> Scene {
    Scene::parse(&serde_json::to_vec(&serde_json::json!({"id":"s","type":"select","props":{"value":value,"options":[{"value":"a","label":"First"},{"value":"b","label":"第二🙂"}]},"events":{"change":action}})).unwrap()).unwrap()
}
fn mounted() -> (SurfaceWindow, Binding, HWND) {
    let window = SurfaceWindow::new(scene("a", "choose"), 240, 200).unwrap();
    let binding = binding();
    window.bind_edits(&binding).unwrap();
    window.set_input_policy(true, true).unwrap();
    let hwnd = unsafe {
        FindWindowExW(
            Some(window.dialog_owner().unwrap()),
            None,
            w!("COMBOBOX"),
            None,
        )
        .unwrap()
    };
    (window, binding, hwnd)
}
fn key(hwnd: HWND, down: bool) {
    unsafe {
        SendMessageW(
            hwnd,
            WM_KEYDOWN,
            Some(WPARAM(if down { VK_DOWN.0 } else { VK_UP.0 } as usize)),
            None,
        );
    }
}
fn index(hwnd: HWND) -> isize {
    unsafe { SendMessageW(hwnd, CB_GETCURSEL, None, None).0 }
}
#[test]
fn surface_select_mount_real_key_emits_change_and_cancel_requeues_original_snapshot() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let (window, binding, hwnd) = mounted();
    assert_eq!(index(hwnd), 0);
    assert!(!window.has_pending_edit());
    key(hwnd, true);
    assert_eq!(index(hwnd), 1);
    assert!(window.has_pending_edit());
    let events = window.prepare_edit(&binding, 7).unwrap();
    assert_eq!(events.len(), 1);
    let wire: serde_json::Value = serde_json::from_slice(events[0].bytes()).unwrap();
    assert_eq!(wire["payload"], serde_json::json!({"value":"b"}));
    assert_eq!(wire["event"], "change");
    assert_eq!(wire["class"], "commit");
    assert_eq!(wire["baseRevision"], 7);
    window.cancel_edits(&events);
    key(hwnd, false);
    let first = window.prepare_edit(&binding, 8).unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(first[0].bytes()).unwrap()["payload"]["value"],
        "b"
    );
    window.settle_edits(&first).unwrap();
    let second = window.prepare_edit(&binding, 8).unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(second[0].bytes()).unwrap()["payload"]["value"],
        "a"
    );
    window.settle_edits(&second).unwrap();
    assert!(!window.has_pending_edit());
    assert!(window.take_error().is_none());
    window.close().unwrap();
    assert!(!unsafe { IsWindow(Some(hwnd)) }.as_bool());
}
#[test]
fn surface_select_mount_authority_sync_and_new_contract_drop_old_intents() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let (window, binding, hwnd) = mounted();
    key(hwnd, true);
    window.replace(scene("a", "choose")).unwrap();
    assert_eq!(index(hwnd), 1); // 未变权威值不冲掉本地选择。
    window.replace(scene("b", "choose")).unwrap();
    assert_eq!(index(hwnd), 1);
    window.replace(scene("a", "new_action")).unwrap();
    assert!(!window.has_pending_edit());
    assert!(window.prepare_edit(&binding, 2).unwrap().is_empty());
    let next = unsafe {
        FindWindowExW(
            Some(window.dialog_owner().unwrap()),
            None,
            w!("COMBOBOX"),
            None,
        )
        .unwrap()
    };
    assert_eq!(index(next), 0);
    key(next, true);
    let events = window.prepare_edit(&binding, 2).unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(events[0].bytes()).unwrap()["action"],
        "new_action"
    );
    window.settle_edits(&events).unwrap();
    window.set_input_policy(false, false).unwrap();
    key(next, false);
    assert!(!window.has_pending_edit());
    assert!(window.take_error().is_none());
}
#[test]
fn surface_select_mount_queue_pressure_restores_last_accepted_selection() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let (window, binding, hwnd) = mounted();
    for i in 0..32 {
        key(hwnd, i % 2 == 0);
    }
    assert_eq!(index(hwnd), 0);
    key(hwnd, true);
    assert_eq!(index(hwnd), 0);
    assert!(window.take_error().is_some());
    for i in 0..32 {
        let events = window.prepare_edit(&binding, 3).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(events[0].bytes()).unwrap()["payload"]
                ["value"],
            if i % 2 == 0 { "b" } else { "a" }
        );
        window.settle_edits(&events).unwrap();
    }
    assert!(!window.has_pending_edit());
}

#[test]
fn surface_select_closeup_deduplicates_keys_and_unchanged_source() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let (window, binding, hwnd) = mounted();
    window.show().unwrap();
    unsafe {
        SendMessageW(hwnd, CB_SHOWDROPDOWN, Some(WPARAM(1)), None);
    }
    key(hwnd, true);
    unsafe {
        SendMessageW(hwnd, CB_SHOWDROPDOWN, Some(WPARAM(0)), None);
    }
    let events = window.prepare_edit(&binding, 1).unwrap();
    assert_eq!(events.len(), 1);
    window.settle_edits(&events).unwrap();
    assert!(!window.has_pending_edit());
    window.replace(scene("b", "choose")).unwrap();
    unsafe {
        SendMessageW(hwnd, CB_SHOWDROPDOWN, Some(WPARAM(1)), None);
        SendMessageW(hwnd, CB_SHOWDROPDOWN, Some(WPARAM(0)), None);
    }
    assert!(!window.has_pending_edit());
    assert!(window.take_error().is_none());
}

#[test]
fn surface_select_mount_dpi_message_uses_window_dpi_without_change_event() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let (window, _, hwnd) = mounted();
    window.show().unwrap();
    unsafe {
        for dpi in [144_u32, 192, 96] {
            let bounds = RECT {
                left: 0,
                top: 0,
                right: 480,
                bottom: 400,
            };
            SendMessageW(
                window.dialog_owner().unwrap(),
                WM_DPICHANGED,
                Some(WPARAM((dpi | (dpi << 16)) as usize)),
                Some(LPARAM((&bounds as *const RECT) as isize)),
            );
            let font = HGDIOBJ(SendMessageW(hwnd, WM_GETFONT, None, None).0 as *mut _);
            let mut value = LOGFONTW::default();
            assert_ne!(
                GetObjectW(
                    font,
                    std::mem::size_of::<LOGFONTW>() as i32,
                    Some((&mut value as *mut LOGFONTW).cast())
                ),
                0
            );
            // 伪造 WM_DPICHANGED 不改变系统窗口 DPI；宿主从 GetDpiForWindow 取权威值。
            let actual_dpi =
                windows::Win32::UI::HiDpi::GetDpiForWindow(window.dialog_owner().unwrap());
            assert_eq!(
                value.lfHeight,
                -((16. * actual_dpi as f32 / 96.).round() as i32)
            );
            assert_eq!(index(hwnd), 0);
            assert!(!window.has_pending_edit());
            assert!(window.take_error().is_none());
        }
    }
}
