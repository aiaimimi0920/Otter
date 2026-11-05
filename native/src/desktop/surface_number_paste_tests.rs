//! 使用注入快照经过真实 HWND 粘贴事务，不读写用户剪贴板。
use super::*;
use crate::desktop::surface_edit_messages::number_paste::paste_with;
fn snapshot(text: &str) -> Option<Vec<u16>> {
    Some(text.encode_utf16().collect())
}
fn select_all(edit: HWND) {
    unsafe {
        SendMessageW(edit, EM_SETSEL, Some(WPARAM(0)), Some(LPARAM(-1)));
    }
}
#[test]
fn surface_number_paste_valid_snapshot_is_single_input_and_native_undoable() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = mount(serde_json::json!("1"));
    let edit = child(&window);
    select_all(edit);
    let mut reads = 0;
    unsafe {
        paste_with(edit, || {
            reads += 1;
            snapshot("3.25")
        });
    }
    assert_eq!(reads, 1);
    assert_eq!(text(edit), "3.25");
    let (_, wire) = event(&window);
    assert_eq!(wire["payload"]["value"], 3.25);
    assert!(!window.has_pending_edit());
    assert_ne!(unsafe { SendMessageW(edit, EM_CANUNDO, None, None) }.0, 0);
    unsafe {
        SendMessageW(edit, WM_UNDO, None, None);
    }
    assert_eq!(text(edit), "1");
    select_all(edit);
    unsafe {
        paste_with(edit, || snapshot(""));
    }
    assert_eq!(text(edit), "");
    assert!(window.take_error().is_none());
}
#[test]
fn surface_number_paste_invalid_busy_and_malformed_snapshot_preserve_prior_undo() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = mount(serde_json::json!("1"));
    let edit = child(&window);
    type_text(edit, "2.5");
    select_all(edit);
    for value in [
        None,
        snapshot("bad"),
        Some(vec![0xd800]),
        Some(vec![49, 0, 50]),
        Some(vec![49; 65537]),
    ] {
        unsafe {
            paste_with(edit, || value);
        }
        assert_eq!(text(edit), "2.5");
        assert_ne!(unsafe { SendMessageW(edit, EM_CANUNDO, None, None) }.0, 0);
    }
    unsafe {
        SendMessageW(edit, WM_UNDO, None, None);
    }
    assert_eq!(text(edit), "1");
    select_all(edit);
    unsafe {
        paste_with(edit, || snapshot("4"));
    }
    assert_eq!(text(edit), "4"); // 每条拒绝路径都释放临时 owner。
    assert!(window.take_error().is_none());
}
#[test]
fn surface_number_paste_reentrant_edit_and_selection_invalidate_old_snapshot() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = mount(serde_json::json!("12"));
    let edit = child(&window);
    select_all(edit);
    unsafe {
        paste_with(edit, || {
            type_text(edit, "7");
            snapshot("9")
        });
    }
    assert_eq!(text(edit), "7");
    select_all(edit);
    unsafe {
        paste_with(edit, || {
            SendMessageW(edit, EM_SETSEL, Some(WPARAM(0)), Some(LPARAM(0)));
            snapshot("9")
        });
    }
    assert_eq!(text(edit), "7");
    select_all(edit);
    unsafe {
        paste_with(edit, || {
            paste_with(edit, || panic!("nested clipboard read"));
            snapshot("8")
        });
    }
    assert_eq!(text(edit), "8");
    assert!(window.take_error().is_none());
}
#[test]
fn surface_number_paste_rechecks_edit_policy_and_composition_after_read() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = mount(serde_json::json!("1"));
    let edit = child(&window);
    select_all(edit);
    unsafe {
        paste_with(edit, || {
            window.set_input_policy(false, false).unwrap();
            window.set_input_policy(true, true).unwrap();
            snapshot("9")
        });
    }
    assert_eq!(text(edit), "1");
    unsafe {
        paste_with(edit, || {
            SendMessageW(edit, WM_IME_STARTCOMPOSITION, None, None);
            SendMessageW(edit, WM_IME_ENDCOMPOSITION, None, None);
            snapshot("9")
        });
    }
    assert_eq!(text(edit), "1");
    window
        .state
        .run(|_| {
            unsafe {
                paste_with(edit, || panic!("busy Host clipboard read"));
            }
            Ok(())
        })
        .unwrap();
    window.set_input_policy(false, false).unwrap();
    unsafe {
        paste_with(edit, || panic!("revoked clipboard read"));
    }
    window.set_input_policy(true, false).unwrap();
    select_all(edit);
    unsafe {
        paste_with(edit, || snapshot("3"));
    }
    assert_eq!(text(edit), "3");
    assert!(window.prepare_edit(&binding(), 1).is_err());
    unsafe {
        SendMessageW(edit, EM_SETREADONLY, Some(WPARAM(1)), None);
        paste_with(edit, || panic!("readonly clipboard read"));
    }
    let plain = super::super::mount();
    unsafe {
        paste_with(child(&plain), || panic!("non-Number clipboard read"));
    }
    assert!(window.take_error().is_none());
}
#[test]
fn surface_number_paste_destroyed_window_cannot_receive_delayed_snapshot() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = mount(serde_json::json!("1"));
    let edit = child(&window);
    unsafe {
        paste_with(edit, || {
            window.close().unwrap();
            snapshot("9")
        });
    }
    assert!(!unsafe { IsWindow(Some(edit)) }.as_bool());
}
