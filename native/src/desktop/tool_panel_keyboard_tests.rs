//! 自有隐藏宿主/真实面板的键盘接线；在隔离 desktop 运行，不代表普通前台人工键盘验收。
use super::*;

struct Keys([u8; 256]);
impl Keys {
    unsafe fn held(held: &[VIRTUAL_KEY]) -> Self {
        let mut original = [0; 256];
        GetKeyboardState(&mut original).unwrap();
        let mut keys = [0; 256];
        for key in held {
            keys[key.0 as usize] = 0x80;
        }
        SetKeyboardState(&keys).unwrap();
        Self(original)
    }
}
impl Drop for Keys {
    fn drop(&mut self) {
        unsafe { SetKeyboardState(&self.0).unwrap() };
    }
}
struct Fixture {
    panel: Window,
    owner: Window,
    state: Box<State>,
}
impl Fixture {
    unsafe fn new() -> Self {
        let instance = HINSTANCE(GetModuleHandleW(None).unwrap().0);
        let class = w!("Otter.Native.ToolbarKeyboardFixture");
        let registered = RegisterClassW(&WNDCLASSW {
            hInstance: instance,
            lpszClassName: class,
            lpfnWndProc: Some(crate::desktop::procedure::window_proc),
            ..Default::default()
        });
        assert!(registered != 0 || GetLastError() == ERROR_CLASS_ALREADY_EXISTS);
        let image = crate::image_decode::Image {
            width: 4,
            height: 4,
            pixels: [10, 20, 30, 255].repeat(16).into(),
        };
        let state = Box::new(State::new(Some(image), true).unwrap());
        let owner = Window(
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                class,
                w!("toolbar keyboard fixture"),
                WS_POPUP,
                0,
                0,
                320,
                240,
                None,
                None,
                Some(instance),
                Some((&*state as *const State).cast()),
            )
            .unwrap(),
        );
        let panel = create(owner.0, &state, false).unwrap();
        Self {
            panel,
            owner,
            state,
        }
    }
    unsafe fn key(&self, key: usize, repeat: bool) {
        SendMessageW(
            self.owner.0,
            WM_KEYDOWN,
            Some(WPARAM(key)),
            Some(LPARAM(if repeat { 1 << 30 } else { 0 })),
        );
    }
    unsafe fn visible(&self) -> bool {
        IsWindowVisible(self.panel.0).as_bool()
    }
    unsafe fn message(&self, target: HWND, key: usize, repeat: bool) -> MSG {
        MSG {
            hwnd: target,
            message: WM_KEYDOWN,
            wParam: WPARAM(key),
            lParam: LPARAM(if repeat { 1 << 30 } else { 0 }),
            ..Default::default()
        }
    }
}

#[test]
fn initial_native_titles_are_preserved_before_panel_is_shown() {
    unsafe {
        let fixture = Fixture::new();
        for (hwnd, expected) in [
            (fixture.owner.0, "toolbar keyboard fixture"),
            (fixture.panel.0, "Otter · 工具（Ctrl+E / F4）"),
        ] {
            let mut title = [0; 64];
            let length = GetWindowTextW(hwnd, &mut title) as usize;
            assert_eq!(String::from_utf16_lossy(&title[..length]), expected);
        }
        assert!(!fixture.visible());
    }
}

#[test]
fn ctrl_e_is_exact_toolbar_chord_and_does_not_claim_tab_art_or_clean_view() {
    assert!(toggles(WM_KEYDOWN, 0x45, true, false, false));
    for (control, shift, alt) in [
        (false, false, false),
        (true, true, false),
        (true, false, true),
        (true, true, true),
    ] {
        assert!(!toggles(WM_KEYDOWN, 0x45, control, shift, alt));
    }
    for key in [VK_TAB.0 as usize, 0x31, 0x34, 0x48] {
        assert!(!toggles(WM_KEYDOWN, key, true, false, false));
    }
    assert!(!toggles(WM_KEYUP, 0x45, true, false, false));
    assert!(!toggles(WM_SYSKEYDOWN, 0x45, true, false, false));
    assert!(toggles(WM_KEYDOWN, VK_F4.0 as usize, false, false, false));
    assert!(!toggles(
        WM_SYSKEYDOWN,
        VK_F4.0 as usize,
        false,
        false,
        true
    ));
}

#[test]
fn real_host_ctrl_e_materializes_once_and_reuses_panel_without_changing_document() {
    unsafe {
        let _keys = Keys::held(&[VK_CONTROL]);
        let fixture = Fixture::new();
        let before = fixture.state.canvas.borrow().editor.document_stamp();
        let unit = fixture.state.unit_id.clone();
        let tool = fixture.state.canvas.borrow().tool;
        assert!(!fixture.visible());
        assert!(!content::ready(fixture.panel.0));
        fixture.key(0x45, false);
        assert!(fixture.visible());
        let controls: Vec<_> = entries()
            .map(|(id, _)| GetDlgItem(Some(fixture.panel.0), id as i32).unwrap())
            .collect();
        fixture.key(0x45, true);
        assert!(fixture.visible());
        fixture.key(0x45, false);
        assert!(!fixture.visible());
        fixture.key(0x45, true);
        assert!(!fixture.visible());
        fixture.key(0x45, false);
        assert!(fixture.visible());
        assert_eq!(
            controls,
            entries()
                .map(|(id, _)| GetDlgItem(Some(fixture.panel.0), id as i32).unwrap())
                .collect::<Vec<_>>()
        );
        assert!(before.matches(&fixture.state.canvas.borrow().editor));
        assert_eq!(fixture.state.unit_id, unit);
        assert_eq!(fixture.state.canvas.borrow().tool, tool);
        assert_eq!(
            &fixture.state.canvas.borrow().image.as_ref().unwrap().pixels[..4],
            &[10, 20, 30, 255]
        );
        let panel = fixture.panel.0;
        drop(fixture);
        assert!(!IsWindow(Some(panel)).as_bool());
        assert!(controls
            .into_iter()
            .all(|hwnd| !IsWindow(Some(hwnd)).as_bool()));
    }
}

#[test]
fn child_ctrl_e_closes_only_its_panel_and_repeat_does_not_reopen_it() {
    unsafe {
        let _keys = Keys::held(&[VK_CONTROL]);
        let fixture = Fixture::new();
        let other = Fixture::new();
        fixture.key(0x45, false);
        other.key(0x45, false);
        let button = GetDlgItem(Some(fixture.panel.0), 0x53).unwrap();
        let message = fixture.message(button, 0x45, false);
        assert!(!super::super::translate(&other.state, &message));
        assert!(super::super::translate(&fixture.state, &message));
        assert!(!fixture.visible());
        assert!(other.visible());
        assert!(super::super::translate(
            &fixture.state,
            &fixture.message(button, 0x45, true)
        ));
        assert!(!fixture.visible());
    }
}

#[test]
fn f4_compatibility_and_escape_keep_existing_panel_close_route() {
    unsafe {
        let _keys = Keys::held(&[]);
        let fixture = Fixture::new();
        fixture.key(VK_F4.0 as usize, false);
        assert!(fixture.visible());
        fixture.key(VK_F4.0 as usize, true);
        assert!(fixture.visible());
        assert!(super::super::translate(
            &fixture.state,
            &fixture.message(fixture.panel.0, VK_F4.0 as usize, false)
        ));
        assert!(!fixture.visible());
        fixture.key(VK_F4.0 as usize, false);
        assert!(super::super::translate(
            &fixture.state,
            &fixture.message(fixture.panel.0, VK_ESCAPE.0 as usize, false)
        ));
        assert!(!fixture.visible());
    }
}

#[test]
fn toolbar_keys_respect_capture_modal_group_and_system_session_gates() {
    unsafe {
        let _keys = Keys::held(&[VK_CONTROL]);
        let fixture = Fixture::new();
        for flag in [
            &*fixture.state.capture_active,
            &fixture.state.file_dialog,
            &fixture.state.group_locked,
            &fixture.state.group_hidden,
        ] {
            flag.set(true);
            fixture.key(0x45, false);
            assert!(!fixture.visible());
            flag.set(false);
        }
        let sibling_modal = std::rc::Rc::new(std::cell::Cell::new(true));
        fixture.state.modals.register(&sibling_modal);
        fixture.key(0x45, false);
        assert!(!fixture.visible());
        sibling_modal.set(false);
        fixture.key(0x45, false);
        assert!(fixture.visible());
        fixture.state.file_dialog.set(true);
        assert!(super::super::translate(
            &fixture.state,
            &fixture.message(fixture.panel.0, 0x45, false)
        ));
        assert!(fixture.visible());
        fixture.state.file_dialog.set(false);
        crate::desktop::session_end::with_query(
            &fixture.state.session_end,
            || true,
            || {
                assert_eq!(
                    SendMessageW(fixture.owner.0, WM_QUERYENDSESSION, None, None).0,
                    1
                );
            },
        );
        assert!(fixture.state.session_end.paused());
        assert!(super::super::translate(
            &fixture.state,
            &fixture.message(fixture.panel.0, 0x45, false)
        ));
        assert!(fixture.visible());
        fixture.key(0x45, false);
        assert!(fixture.visible());
        SendMessageW(fixture.owner.0, WM_ENDSESSION, Some(WPARAM(0)), None);
        fixture.key(0x45, false);
        assert!(!fixture.visible());
    }
}

#[test]
fn plain_e_and_modified_e_do_not_get_consumed_as_toolbar_toggle() {
    unsafe {
        let fixture = Fixture::new();
        for keys in [
            &[][..],
            &[VK_CONTROL, VK_SHIFT][..],
            &[VK_CONTROL, VK_MENU][..],
        ] {
            let _keys = Keys::held(keys);
            assert!(dispatch(
                fixture.owner.0,
                &fixture.state,
                WM_KEYDOWN,
                WPARAM(0x45),
                LPARAM(0)
            )
            .is_none());
            assert!(!fixture.visible());
        }
    }
}

#[test]
fn stale_panel_owner_identity_does_not_toggle_or_consume_foreign_child_input() {
    unsafe {
        let _keys = Keys::held(&[VK_CONTROL]);
        let fixture = Fixture::new();
        SetWindowLongPtrW(fixture.panel.0, GWLP_USERDATA, 0);
        fixture.key(0x45, false);
        assert!(!fixture.visible());
        assert!(!super::super::translate(
            &fixture.state,
            &fixture.message(fixture.panel.0, 0x45, false)
        ));
        SetWindowLongPtrW(fixture.panel.0, GWLP_USERDATA, fixture.owner.0 .0 as isize);
        fixture.key(0x45, false);
        assert!(fixture.visible());
    }
}
