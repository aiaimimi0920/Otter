//! Real admission/command dispatch on hidden HWNDs; inject only final selection callbacks, without desktop reads/system hotkey registration.
use super::*;
use crate::ui_text::{self, DisplayLanguage};
use std::{cell::Cell, rc::Rc};
struct LanguageReset(DisplayLanguage);
impl Drop for LanguageReset {
    fn drop(&mut self) {
        ui_text::set_language(self.0);
    }
}

unsafe fn window() -> crate::desktop::Window {
    crate::desktop::Window(
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("STATIC"),
            w!("hotkey fixture"),
            WS_POPUP,
            -32000,
            -32000,
            1,
            1,
            None,
            None,
            None,
            None,
        )
        .unwrap(),
    )
}
unsafe fn trigger(hwnd: HWND, state: &State, start: impl FnOnce(usize)) {
    dispatch_with(
        hwnd,
        state,
        WM_HOTKEY,
        WPARAM(LIVE_ID as usize),
        LPARAM(((LIVE_KEY << 16) | MOD_CONTROL.0) as isize),
        start,
    );
}
#[test]
fn live_hotkey_dispatches_live_only_and_pending_request_prevents_a_second_selection() {
    unsafe {
        let window = window();
        let state = State::new(None, false).unwrap();
        update(&state, true, |_, _| Ok(())).unwrap();
        let stamp = state.canvas.borrow().editor.document_stamp();
        let mut count = 0;
        for _ in 0..3 {
            trigger(window.0, &state, |command| {
                assert_eq!(command, crate::desktop::capture_input::live::COMMAND);
                count += 1;
                *state.live_request.borrow_mut() =
                    Some(crate::desktop::capture_input::live::Request::Region(RECT {
                        left: -100,
                        top: 10,
                        right: -50,
                        bottom: 60,
                    }));
            });
        }
        assert_eq!(count, 1);
        assert!(stamp.matches(&state.canvas.borrow().editor));
        assert!(state.new_pin.borrow().is_none());
        assert!(!state.capture_active.get() && !state.file_dialog.get());
        state.live_request.borrow_mut().take(); // After cancellation/settlement, the next keystroke may select again.
        trigger(window.0, &state, |command| {
            assert_eq!(command, crate::desktop::capture_input::live::COMMAND);
            count += 1;
        });
        assert_eq!(count, 2);
        update(&state, false, |_, _| Ok(())).unwrap();
    }
}
#[test]
fn live_selection_rejects_missing_registration_modal_disabled_closed_and_borrowed_state() {
    unsafe {
        let window = window();
        let state = State::new(None, false).unwrap();
        update(&state, true, |_, _| Ok(())).unwrap();
        for flag in [&state.capture_active, &state.file_dialog] {
            flag.set(true);
            trigger(window.0, &state, |_| panic!("busy selection"));
            flag.set(false);
        }
        for flag in [&state.global_capture, &state.global_live_capture] {
            flag.set(false);
            trigger(window.0, &state, |_| panic!("unregistered selection"));
            flag.set(true);
        }
        let other = Rc::new(Cell::new(true));
        state.modals.register(&other);
        trigger(window.0, &state, |_| panic!("another document owns modal"));
        other.set(false);
        let _ = EnableWindow(window.0, false);
        trigger(window.0, &state, |_| panic!("disabled owner"));
        let _ = EnableWindow(window.0, true);
        {
            let _canvas = state.canvas.borrow_mut();
            trigger(window.0, &state, |_| panic!("borrowed canvas"));
        }
        {
            let _pending = state.live_request.borrow_mut();
            trigger(window.0, &state, |_| panic!("borrowed request"));
        }
        DestroyWindow(window.0).unwrap();
        trigger(window.0, &state, |_| panic!("closed owner"));
        update(&state, false, |_, _| Ok(())).unwrap();
    }
}
#[test]
fn menu_reports_each_independent_secondary_degradation_on_every_document() {
    let _language = LanguageReset(ui_text::current_language());
    let owner = State::new(None, false).unwrap();
    let mut other = State::new(None, false).unwrap();
    other.hotkey_enabled = owner.hotkey_enabled.clone();
    other.live_hotkey_enabled = owner.live_hotkey_enabled.clone();
    other.long_hotkey_enabled = owner.long_hotkey_enabled.clone();
    for (language, normal, prefix, suffix) in [
        (
            DisplayLanguage::English,
            "Global capture / Live / scrolling capture",
            "Global capture (",
            " unavailable)",
        ),
        (
            DisplayLanguage::SimplifiedChinese,
            "全局截图 / Live / 长截图录制",
            "全局截图快捷键（",
            " 不可用）",
        ),
    ] {
        ui_text::set_language(language);
        let normal = format!("{normal}\tCtrl+1 / Ctrl+2 / Ctrl+3");
        for (mask, missing) in [
            None,
            Some("Ctrl+2"),
            Some("Ctrl+3"),
            Some("Ctrl+2 / Ctrl+3"),
        ]
        .into_iter()
        .enumerate()
        {
            update(&owner, true, |key, _| {
                if (key == Key::Live && mask & 1 != 0) || (key == Key::Long && mask & 2 != 0) {
                    Err(windows::core::Error::new(E_FAIL, "occupied"))
                } else {
                    Ok(())
                }
            })
            .unwrap();
            // Fluent isolates the shortcut argument; assert full text instead of stripping it.
            let expected = missing.map_or_else(
                || normal.clone(),
                |shortcuts| {
                    format!("{prefix}\u{2068}{shortcuts}\u{2069}{suffix}\tCtrl+1 / Ctrl+2 / Ctrl+3")
                },
            );
            assert_eq!(label(&owner), expected, "{language:?}, mask {mask}");
            assert_eq!(label(&other), expected, "{language:?}, mask {mask}");
            update(&other, false, |_, _| panic!("non-owner release")).unwrap();
            assert_eq!(label(&owner), expected);
            assert_eq!(label(&other), expected);
            update(&owner, false, |_, _| Ok(())).unwrap();
            assert_eq!(label(&owner), normal);
            assert_eq!(label(&other), normal);
        }
    }
}
