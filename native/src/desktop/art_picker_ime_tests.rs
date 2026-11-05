//! 宿主 IME 标点的窄快捷键回归；标准 EDIT 不经过宿主命令，不注入全局输入。
use super::*;

#[test]
fn ime_exclamation_requires_digit_one_and_exact_modifiers_not_char_or_processkey() {
    let test = |message, character, held: &[VIRTUAL_KEY]| {
        key_state(
            message,
            character,
            held.contains(&VIRTUAL_KEY(0x31)),
            held.contains(&VK_CONTROL),
            held.contains(&VK_SHIFT),
            held.contains(&VK_MENU),
            held.contains(&VK_LWIN) || held.contains(&VK_RWIN),
        )
    };
    for held in [
        vec![],
        vec![VK_SHIFT],
        vec![VIRTUAL_KEY(0x31)],
        vec![VK_SHIFT, VK_NUMPAD1],
        vec![VK_SHIFT, VIRTUAL_KEY(0x31), VK_CONTROL],
        vec![VK_SHIFT, VIRTUAL_KEY(0x31), VK_MENU],
        vec![VK_SHIFT, VIRTUAL_KEY(0x31), VK_LWIN],
        vec![VK_SHIFT, VIRTUAL_KEY(0x31), VK_RWIN],
    ] {
        for character in [0x21, 0xff01] {
            assert!(!test(WM_IME_CHAR, character, &held), "{held:?}");
        }
    }
    let held = &[VK_SHIFT, VIRTUAL_KEY(0x31)];
    for character in [0x21, 0xff01] {
        assert!(test(WM_IME_CHAR, character, held));
        assert!(!test(WM_CHAR, character, held));
    }
    assert!(!test(WM_KEYDOWN, VK_PROCESSKEY.0 as usize, held));
    for character in [0x31, 0xff11, 0x40] {
        assert!(!test(WM_IME_CHAR, character, held));
    }
}

#[test]
fn actual_host_never_opens_picker_for_ime_punctuation_without_real_digit_one() {
    unsafe {
        let _apartment = Apartment::new().unwrap();
        let (_, doc) = document();
        let _keys = Keys::held(&[VK_SHIFT]);
        doc.state.catalog.borrow_mut().loaded = Some(catalog(|_| {}));
        // SetKeyboardState在本环境不会设置数字键；不把人工键态伪造为正向IME验收。
        assert!(GetKeyState(0x31) >= 0);
        assert!(!key(WM_IME_CHAR, WPARAM(0xff01)));
        assert!(dispatch(
            doc.window.0,
            &doc.state,
            WM_IME_CHAR,
            WPARAM(0xff01),
            LPARAM(0)
        )
        .is_none());
        let stamp = doc.state.canvas.borrow().editor.document_stamp();
        SendMessageW(doc.window.0, WM_IME_CHAR, Some(WPARAM(0xff01)), None);
        assert!(doc.state.art_request.borrow().is_none());
        assert!(!doc.state.file_dialog.get());
        SendMessageW(
            doc.window.0,
            WM_IME_CHAR,
            Some(WPARAM(0xff01)),
            Some(LPARAM(1 << 30)),
        );
        doc.state.group_locked.set(true);
        SendMessageW(doc.window.0, WM_IME_CHAR, Some(WPARAM(0xff01)), None);
        assert!(doc.state.art_request.borrow().is_none());
        assert!(stamp.matches(&doc.state.canvas.borrow().editor));
    }
}

#[test]
fn standard_child_edit_keeps_final_character_and_never_routes_ime_to_host_picker() {
    unsafe {
        let _apartment = Apartment::new().unwrap();
        let (instance, doc) = document();
        let edit = CreateWindowExW(
            WINDOW_EX_STYLE(0),
            w!("EDIT"),
            w!(""),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP,
            0,
            0,
            100,
            24,
            Some(doc.window.0),
            None,
            Some(instance),
            None,
        )
        .unwrap();
        let _keys = Keys::held(&[VK_SHIFT]);
        SendMessageW(edit, WM_IME_CHAR, Some(WPARAM(0xff01)), None);
        assert!(!doc.state.file_dialog.get());
        assert!(doc.state.art_request.borrow().is_none());
        // 直接 SendMessage 不运行系统 IME 翻译；另验最终字符仍由标准 EDIT 接收。
        SetWindowTextW(edit, w!("")).unwrap();
        SendMessageW(edit, WM_CHAR, Some(WPARAM(0xff01)), None);
        let mut text = [0; 4];
        assert_eq!(GetWindowTextW(edit, &mut text), 1);
        assert_eq!(text[0], 0xff01);
        assert!(!doc.state.file_dialog.get());
        assert!(doc.state.art_request.borrow().is_none());
    }
}
