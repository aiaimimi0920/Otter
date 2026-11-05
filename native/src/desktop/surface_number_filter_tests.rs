//! 数字编辑态过滤的真实 HWND 接缝；整段替换模拟粘贴结果，不访问系统剪贴板。
use super::*;
#[test]
fn surface_number_filter_preflight_matches_native_character_repeat_semantics() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let plain = super::super::mount();
    let plain_edit = child(&plain);
    type_text(plain_edit, "");
    unsafe {
        SendMessageW(
            plain_edit,
            WM_CHAR,
            Some(WPARAM(b'.' as usize)),
            Some(LPARAM(2)),
        );
    }
    // 真实 classic EDIT 会展开低 16 位重复数，预检必须验证完整结果而非单字符。
    assert_eq!(text(plain_edit), "..");
    let window = mount(serde_json::json!("1"));
    let edit = child(&window);
    type_text(edit, "");
    unsafe {
        SendMessageW(edit, WM_CHAR, Some(WPARAM(b'.' as usize)), Some(LPARAM(2)));
    }
    assert_eq!(text(edit), "");
    assert_ne!(unsafe { SendMessageW(edit, EM_CANUNDO, None, None) }.0, 0);
    unsafe {
        SendMessageW(edit, WM_UNDO, None, None);
    }
    assert_eq!(text(edit), "1");
    unsafe {
        SendMessageW(edit, EM_SETSEL, Some(WPARAM(0)), Some(LPARAM(-1)));
        SendMessageW(edit, WM_CHAR, Some(WPARAM(b'7' as usize)), Some(LPARAM(3)));
    }
    assert_eq!(text(edit), "777");
    assert!(window.take_error().is_none());
}
#[test]
fn surface_number_filter_rejected_replacement_preserves_native_undo() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = mount(serde_json::json!("1"));
    let edit = child(&window);
    type_text(edit, "2.5");
    assert_ne!(unsafe { SendMessageW(edit, EM_CANUNDO, None, None) }.0, 0);
    type_text(edit, "1.2.3");
    assert_eq!(text(edit), "2.5");
    assert_ne!(unsafe { SendMessageW(edit, EM_CANUNDO, None, None) }.0, 0);
    unsafe {
        SendMessageW(edit, WM_UNDO, None, None);
    }
    assert_eq!(text(edit), "1");
    assert!(window.take_error().is_none());
}
#[test]
fn surface_number_filter_rejected_decimal_key_preserves_native_undo_and_selection() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = mount(serde_json::json!("1"));
    let edit = child(&window);
    type_text(edit, "2.5");
    unsafe {
        SendMessageW(edit, EM_SETSEL, Some(WPARAM(3)), Some(LPARAM(3)));
        SendMessageW(edit, WM_CHAR, Some(WPARAM(b'.' as usize)), None);
    }
    assert_eq!(text(edit), "2.5");
    assert_eq!(selection(edit), (3, 3));
    assert_ne!(unsafe { SendMessageW(edit, EM_CANUNDO, None, None) }.0, 0);
    unsafe {
        SendMessageW(edit, WM_UNDO, None, None);
    }
    assert_eq!(text(edit), "1");
    assert!(window.take_error().is_none());
}
#[test]
fn surface_number_filter_preflight_respects_partial_selection_and_empty_replacement() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = mount(serde_json::json!("12.5"));
    let edit = child(&window);
    unsafe {
        SendMessageW(edit, EM_SETSEL, Some(WPARAM(1)), Some(LPARAM(3)));
        let replacement = HSTRING::from("3e");
        SendMessageW(
            edit,
            EM_REPLACESEL,
            Some(WPARAM(1)),
            Some(LPARAM(replacement.as_ptr() as isize)),
        );
    }
    assert_eq!(text(edit), "13e5");
    let (_, wire) = event(&window);
    assert_eq!(wire["payload"]["value"], 1300000.);
    type_text(edit, "");
    assert_eq!(text(edit), "");
    let (_, wire) = event(&window);
    assert!(wire["payload"]["value"].is_null());
    assert!(window.take_error().is_none());
}
fn selection(edit: HWND) -> (u32, u32) {
    let mut start = 0_u32;
    let mut end = 0_u32;
    unsafe {
        SendMessageW(
            edit,
            EM_GETSEL,
            Some(WPARAM((&mut start as *mut u32) as usize)),
            Some(LPARAM((&mut end as *mut u32) as isize)),
        );
    }
    (start, end)
}
#[test]
fn surface_number_filter_keyboard_rejects_without_losing_selection_or_emitting() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = mount(serde_json::json!("12.5"));
    let edit = child(&window);
    unsafe {
        SetFocus(Some(edit)).unwrap();
        SendMessageW(edit, EM_SETSEL, Some(WPARAM(1)), Some(LPARAM(3)));
        for ch in ['x', ' ', '中', ','] {
            SendMessageW(edit, WM_CHAR, Some(WPARAM(ch as usize)), None);
        }
    }
    assert_eq!(text(edit), "12.5");
    assert_eq!(selection(edit), (1, 3));
    assert!(!window.has_pending_edit());
    assert!(window.take_error().is_none());
}
#[test]
fn surface_number_filter_replacement_rejects_whole_candidate_and_preserves_pending() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = mount(serde_json::json!("1"));
    let edit = child(&window);
    type_text(edit, "2.50");
    for invalid in ["12abc", "1.2.3", "1e2e3", "--2", "2-3", "0x12", "NaN"] {
        type_text(edit, invalid);
        assert_eq!(text(edit), "2.50");
        assert_eq!(selection(edit), (0, 4));
        assert_eq!(values(&window)["field"], "2.50");
    }
    let (_, wire) = event(&window);
    assert_eq!(wire["payload"]["value"], 2.5);
    assert!(!window.has_pending_edit());
    assert!(window.take_error().is_none());
}
#[test]
fn surface_number_filter_incomplete_exponents_remain_editable_and_numeric_events_stay_null() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = mount(serde_json::json!("1"));
    let edit = child(&window);
    for raw in ["-", ".", "-.", "1.", "e", "e-", "1e", "1e-", "1e400"] {
        type_text(edit, raw);
        assert_eq!(text(edit), raw);
        let (_, wire) = event(&window);
        assert!(wire["payload"]["value"].is_null());
    }
    type_text(edit, "1e-2");
    let (_, wire) = event(&window);
    assert_eq!(wire["payload"]["value"], 0.01);
    assert!(window.take_error().is_none());
}
#[test]
fn surface_number_filter_waits_for_composition_end_and_accepts_numeric_composition() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = mount(serde_json::json!("12"));
    let edit = child(&window);
    unsafe {
        SendMessageW(edit, WM_IME_STARTCOMPOSITION, None, None);
    }
    type_text(edit, "中文");
    assert_eq!(text(edit), "中文");
    assert!(!window.has_pending_edit());
    unsafe {
        SendMessageW(edit, WM_IME_ENDCOMPOSITION, None, None);
    }
    assert_eq!(text(edit), "12");
    assert!(!window.has_pending_edit());
    unsafe {
        SendMessageW(edit, WM_IME_STARTCOMPOSITION, None, None);
    }
    type_text(edit, "3.25");
    unsafe {
        SendMessageW(edit, WM_IME_ENDCOMPOSITION, None, None);
    }
    let (_, wire) = event(&window);
    assert_eq!(wire["payload"]["value"], 3.25);
    assert!(window.take_error().is_none());
}
