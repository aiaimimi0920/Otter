//! Real HWND wiring for editing tools, system moves, and toolbar visibility; no global input or user-window operations.
use super::*;
use crate::{editor::Tool, viewport::Point};
use windows::Win32::{Graphics::Gdi::ClientToScreen, UI::Input::KeyboardAndMouse::*};

struct Keys([u8; 256]);
impl Keys {
    unsafe fn held(held: &[VIRTUAL_KEY]) -> Self {
        let mut old = [0; 256];
        GetKeyboardState(&mut old).unwrap();
        let mut keys = [0; 256];
        for k in held {
            keys[k.0 as usize] = 0x80;
        }
        SetKeyboardState(&keys).unwrap();
        Self(old)
    }
}
impl Drop for Keys {
    fn drop(&mut self) {
        unsafe { SetKeyboardState(&self.0).unwrap() };
    }
}
unsafe fn image_point(f: &Fixture, x: f32, y: f32) -> LPARAM {
    let c = f.state.canvas.borrow();
    let factor = GetDpiForWindow(f.owner.0).max(96) as f32 / 96.0;
    let mut p = POINT {
        x: ((c.viewport.origin.x + x * c.viewport.scale) * factor).round() as i32,
        y: ((c.viewport.origin.y + y * c.viewport.scale) * factor).round() as i32,
    };
    assert!(ClientToScreen(f.owner.0, &mut p).as_bool());
    LPARAM((p.x as u16 as isize) | ((p.y as u16 as isize) << 16))
}
fn add_line(c: &mut crate::desktop::canvas::Canvas) {
    c.editor
        .begin(Tool::Line, Point { x: 4.0, y: 4.0 })
        .unwrap();
    c.editor.extend(Point { x: 20.0, y: 20.0 }).unwrap();
    assert!(c.editor.commit());
}
#[test]
fn select_background_uses_native_window_drag_but_annotation_and_shift_stay_client_input() {
    unsafe {
        let _keys = Keys::held(&[]);
        let f = Fixture::new();
        let _ = ShowWindow(f.owner.0, SW_SHOWNOACTIVATE);
        crate::desktop::edit_mode::toolbar_mode(f.owner.0, &f.state, true).unwrap();
        {
            add_line(&mut f.state.canvas.borrow_mut());
        }
        let blank = image_point(&f, 50.0, 50.0);
        let hit = |lp| SendMessageW(f.owner.0, WM_NCHITTEST, None, Some(lp)).0;
        assert_eq!(hit(blank), HTCAPTION as isize);
        assert_eq!(hit(image_point(&f, 10.0, 10.0)), HTCLIENT as isize);
        {
            let _shift = Keys::held(&[VK_SHIFT]);
            assert_eq!(hit(blank), HTCLIENT as isize);
        }
        let revision = f.state.canvas.borrow().editor.revision();
        f.state.canvas.borrow_mut().selected = Some((0, revision));
        assert_eq!(hit(blank), HTCLIENT as isize);
        f.state.canvas.borrow_mut().selected = None;
        crate::desktop::edit_mode::execute(f.owner.0, &f.state, false).unwrap();
        assert_eq!(hit(blank), HTCAPTION as isize);
        assert_eq!(hit(image_point(&f, 10.0, 10.0)), HTCAPTION as isize);
    }
}
#[test]
fn empty_select_drag_uses_the_same_complete_frame_and_does_not_pan_the_image() {
    unsafe {
        let _keys = Keys::held(&[]);
        let f = Fixture::new();
        let _ = ShowWindow(f.owner.0, SW_SHOWNOACTIVATE);
        crate::desktop::edit_mode::toolbar_mode(f.owner.0, &f.state, true).unwrap();
        let viewport = f.state.canvas.borrow().viewport;
        let stamp = f.state.canvas.borrow().editor.document_stamp();
        SendMessageW(f.owner.0, WM_ENTERSIZEMOVE, None, None);
        assert!(crate::desktop::pin_window::preview::active(&f.state));
        SetWindowPos(
            f.owner.0,
            None,
            350,
            450,
            0,
            0,
            SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
        )
        .unwrap();
        SendMessageW(f.owner.0, WM_EXITSIZEMOVE, None, None);
        assert_eq!(f.state.canvas.borrow().tool, Tool::Select);
        assert_eq!(f.state.canvas.borrow().viewport, viewport);
        assert!(stamp.matches(&f.state.canvas.borrow().editor));
        assert!(!crate::desktop::pin_window::animation::busy(&f.state));
        assert!(f.state.error.borrow().is_none());
    }
}
#[test]
fn ctrl_e_opens_select_and_closes_into_pin_drag_while_cancelling_uncommitted_drawing() {
    unsafe {
        let _keys = Keys::held(&[VK_CONTROL]);
        let f = Fixture::new();
        let key = || SendMessageW(f.owner.0, WM_KEYDOWN, Some(WPARAM(0x45)), None);
        key();
        assert_eq!(f.state.canvas.borrow().tool, Tool::Select);
        assert!(IsWindowVisible(f.panel.0).as_bool());
        let stamp = f.state.canvas.borrow().editor.document_stamp();
        {
            let mut c = f.state.canvas.borrow_mut();
            c.select_tool(Tool::Rectangle);
            c.editor
                .begin(Tool::Rectangle, Point { x: 1.0, y: 1.0 })
                .unwrap();
            c.editor.extend(Point { x: 30.0, y: 30.0 }).unwrap();
        }
        SetCapture(f.owner.0);
        key();
        let c = f.state.canvas.borrow();
        assert_eq!(c.tool, Tool::Pan);
        assert!(!c.crop_mode && c.selected_indices().is_empty() && !c.blocks_wheel());
        assert!(stamp.matches(&c.editor));
        drop(c);
        assert!(GetCapture().is_invalid());
        assert!(!IsWindowVisible(f.panel.0).as_bool());
        assert_eq!(
            SendMessageW(f.owner.0, WM_NCHITTEST, None, None).0,
            HTCAPTION as isize
        );
    }
}
#[test]
fn editing_escape_cancels_a_transform_without_clearing_selection_or_closing_the_pin() {
    unsafe {
        let _keys = Keys::held(&[]);
        let f = Fixture::new();
        crate::desktop::edit_mode::toolbar_mode(f.owner.0, &f.state, true).unwrap();
        let _ = ShowWindow(f.panel.0, SW_SHOWNOACTIVATE);
        {
            let mut c = f.state.canvas.borrow_mut();
            add_line(&mut c);
            c.selected = Some((0, c.editor.revision()));
            assert!(c.begin_rotation_at(Point { x: 100.0, y: 100.0 }));
        }
        let stamp = f.state.canvas.borrow().editor.document_stamp();
        SendMessageW(
            f.owner.0,
            WM_KEYDOWN,
            Some(WPARAM(VK_ESCAPE.0 as usize)),
            None,
        );
        assert_eq!(f.state.canvas.borrow().selected_index(), Some(0));
        assert!(!f.state.canvas.borrow().blocks_wheel());
        assert!(IsWindow(Some(f.owner.0)).as_bool() && IsWindowVisible(f.panel.0).as_bool());
        assert!(stamp.matches(&f.state.canvas.borrow().editor));
    }
}

#[test]
fn actual_select_pointer_modifiers_transform_annotations_without_switching_mode_or_moving_pin() {
    use windows::Win32::Graphics::Gdi::ScreenToClient;
    unsafe {
        for (keys, rotate) in [(&[VK_CONTROL][..], true), (&[VK_MENU][..], false)] {
            let _keys = Keys::held(keys);
            let f = Fixture::new();
            crate::desktop::edit_mode::toolbar_mode(f.owner.0, &f.state, true).unwrap();
            add_line(&mut f.state.canvas.borrow_mut());
            let screen = image_point(&f, 19.0, 19.0);
            let mut point = POINT {
                x: screen.0 as i16 as i32,
                y: (screen.0 >> 16) as i16 as i32,
            };
            assert!(ScreenToClient(f.owner.0, &mut point).as_bool());
            let lp = LPARAM((point.x as u16 as isize) | ((point.y as u16 as isize) << 16));
            let mut before = RECT::default();
            GetWindowRect(f.owner.0, &mut before).unwrap();
            let stamp = f.state.canvas.borrow().editor.document_stamp();
            SendMessageW(f.owner.0, WM_LBUTTONDOWN, None, Some(lp));
            let c = f.state.canvas.borrow();
            assert_eq!(c.rotation_drag.is_some(), rotate);
            assert!(c.selection_drag.is_some());
            assert!(c.selection_enabled);
            assert_eq!(c.selected_index(), Some(0));
            assert!(stamp.matches(&c.editor));
            drop(c);
            assert_eq!(GetCapture(), f.owner.0);
            SendMessageW(f.owner.0, WM_CANCELMODE, None, None);
            assert!(!f.state.canvas.borrow().blocks_wheel());
            assert!(GetCapture().is_invalid());
            let mut after = RECT::default();
            GetWindowRect(f.owner.0, &mut after).unwrap();
            assert_eq!(after, before);
        }
    }
}

#[test]
fn initially_visible_strip_enables_selection_without_changing_hidden_pin_defaults() {
    unsafe {
        let _keys = Keys::held(&[]);
        let hidden = Fixture::new();
        assert!(!IsWindowVisible(hidden.panel.0).as_bool());
        assert_eq!(hidden.state.canvas.borrow().tool, Tool::Pan);
        let visible = Fixture::with_visibility(true);
        assert!(IsWindowVisible(visible.panel.0).as_bool());
        assert!(visible.state.canvas.borrow().selection_enabled);
        assert_eq!(visible.state.canvas.borrow().tool, Tool::Select);
    }
}

#[test]
fn select_button_is_a_default_on_toggle_without_a_dropdown_and_reopens_enabled() {
    unsafe {
        let _keys = Keys::held(&[]);
        let f = Fixture::new();
        {
            let _ctrl = Keys::held(&[VK_CONTROL]);
            SendMessageW(f.owner.0, WM_KEYDOWN, Some(WPARAM(0x45)), None);
        }
        assert!(IsWindowVisible(f.panel.0).as_bool());
        assert!(f.state.canvas.borrow().selection_enabled);
        assert_eq!(f.state.canvas.borrow().tool, Tool::Select);
        assert!(catalog::choices(catalog::MODE).is_empty());
        assert!(
            catalog::items().iter().any(|item| matches!(item,
            neuro_mini_ui::Item::Button(button) if button.id == catalog::MODE && !button.dropdown))
        );
        add_line(&mut f.state.canvas.borrow_mut());
        let revision = f.state.canvas.borrow().editor.revision();
        f.state.canvas.borrow_mut().selected = Some((0, revision));
        let stamp = f.state.canvas.borrow().editor.document_stamp();
        let button = GetDlgItem(Some(f.panel.0), catalog::MODE as i32).unwrap();
        let click = || {
            SendMessageW(button, BM_CLICK, None, None);
            let mut message = MSG::default();
            assert!(PeekMessageW(
                &mut message,
                Some(f.owner.0),
                super::super::super::EXECUTE,
                super::super::super::EXECUTE,
                PM_REMOVE
            )
            .as_bool());
            DispatchMessageW(&message);
        };
        click();
        assert!(!f.state.canvas.borrow().selection_enabled);
        assert!(f.state.canvas.borrow().selected_indices().is_empty());
        assert!(stamp.matches(&f.state.canvas.borrow().editor));
        let mut label = [0u16; 64];
        let length = GetWindowTextW(button, &mut label);
        assert_eq!(
            String::from_utf16_lossy(&label[..length as usize]),
            "选择（静默）"
        );
        execute(f.owner.0, &f.state, catalog::MODE as usize, true);
        assert!(!f.state.canvas.borrow().selection_enabled);
        assert!(!f.state.file_dialog.get());
        click();
        assert!(f.state.canvas.borrow().selection_enabled);
        SendMessageW(f.owner.0, WM_KEYDOWN, Some(WPARAM(0x51)), None);
        assert!(!f.state.canvas.borrow().selection_enabled);
        // Removing W/E/R mode shortcuts stops them from stealing existing tool keys.
        for (key, tool) in [
            (0x57, Tool::Eraser),
            (0x45, Tool::Ellipse),
            (0x52, Tool::Rectangle),
        ] {
            SendMessageW(f.owner.0, WM_KEYDOWN, Some(WPARAM(key)), None);
            assert_eq!(f.state.canvas.borrow().tool, tool);
        }
        {
            let _ctrl = Keys::held(&[VK_CONTROL]);
            SendMessageW(f.owner.0, WM_KEYDOWN, Some(WPARAM(0x45)), None);
            SendMessageW(f.owner.0, WM_KEYDOWN, Some(WPARAM(0x45)), None);
        }
        assert!(f.state.canvas.borrow().selection_enabled);
        assert_eq!(f.state.canvas.borrow().tool, Tool::Select);
        f.state.group_locked.set(true);
        click();
        assert!(f.state.canvas.borrow().selection_enabled);
        assert!(stamp.matches(&f.state.canvas.borrow().editor));
    }
}
