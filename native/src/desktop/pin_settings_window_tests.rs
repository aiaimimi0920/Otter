//! Actual settings HWNDs through geometry/opacity/unified history; use only test-owned windows without global input injection.
use super::*;
use crate::desktop::{pin_settings as settings, pin_settings_history as history, tool_panel};
use windows::Win32::UI::Input::KeyboardAndMouse::*;
#[path = "device_settings_menu_tests.rs"]
mod connections;
#[path = "pin_settings_move_tests.rs"]
mod movement;
#[path = "pin_settings_projection_tests.rs"]
mod projection;
#[path = "pin_settings_stability_tests.rs"]
mod stability;
#[path = "pin_settings_text_window_tests.rs"]
mod text;
unsafe fn open(f: &Fixture) -> HWND {
    let _ = ShowWindow(f.owner.0, SW_SHOWNOACTIVATE);
    crate::desktop::edit_mode::toolbar_mode(f.owner.0, &f.state, true).unwrap();
    let _ = ShowWindow(f.panel.0, SW_SHOWNOACTIVATE);
    execute(f.owner.0, &f.state, catalog::SETTINGS as usize, false);
    let window = settings::window(&f.state).unwrap();
    assert!(IsWindowVisible(window).as_bool());
    window
}
pub(super) unsafe fn field(window: HWND, id: u16) -> HWND {
    let mut target = (i32::from(id) * 4, HWND::default());
    unsafe extern "system" fn find(hwnd: HWND, lp: LPARAM) -> windows::core::BOOL {
        let target = &mut *(lp.0 as *mut (i32, HWND));
        if GetDlgCtrlID(hwnd) == target.0 {
            target.1 = hwnd;
        }
        windows::core::BOOL::from(target.1.is_invalid())
    }
    let _ = EnumChildWindows(
        Some(window),
        Some(find),
        LPARAM(&mut target as *mut _ as isize),
    );
    assert!(!target.1.is_invalid(), "missing property field {id}");
    target.1
}
unsafe fn enter(f: &Fixture, child: HWND, value: &str) {
    SetWindowTextW(child, &HSTRING::from(value)).unwrap();
    assert!(tool_panel::translate(
        &f.state,
        &MSG {
            hwnd: child,
            message: WM_KEYDOWN,
            wParam: WPARAM(VK_RETURN.0 as usize),
            ..Default::default()
        }
    ));
}
unsafe fn dispatch(f: &Fixture) {
    for _ in 0..32 {
        let mut message = MSG::default();
        assert!(PeekMessageW(
            &mut message,
            Some(f.owner.0),
            super::super::super::EXECUTE,
            super::super::super::EXECUTE,
            PM_REMOVE
        )
        .as_bool());
        let setting = message.lParam.0 == neuro_mini_ui::PANEL;
        DispatchMessageW(&message);
        if setting {
            return;
        }
    }
    panic!("setting notification was not delivered");
}
#[test]
fn content_border_radius_recolor_and_undo_go_through_actual_settings_controls() {
    unsafe {
        let f = Fixture::new();
        let window = open(&f);
        let default = f.state.canvas.borrow().frame_style;
        let click = || {
            SendMessageW(field(window, settings::BORDER), BM_CLICK, None, None);
            dispatch(&f);
        };
        click();
        let red = f.state.canvas.borrow().frame_style;
        assert_eq!(red.border().unwrap().0, 4.0);
        enter(&f, field(window, settings::RADIUS), "24");
        dispatch(&f);
        assert_eq!(f.state.canvas.borrow().frame_style.radius(), 24.0);
        let revision = f.state.canvas.borrow().editor.revision();
        enter(&f, field(window, settings::RADIUS), "24");
        dispatch(&f);
        assert_eq!(f.state.canvas.borrow().editor.revision(), revision);
        f.state.edit_palette.borrow_mut().active = neuro_mini_ui::Rgba::rgb(0, 0, 255);
        click();
        assert_eq!(
            f.state.canvas.borrow().frame_style.border().unwrap(),
            (4., crate::surface_color::Color([0., 0., 1., 1.]))
        );
        click();
        assert_eq!(f.state.canvas.borrow().frame_style.border(), None);
        for _ in 0..4 {
            assert!(history::step(f.owner.0, &f.state, false).unwrap());
        }
        assert_eq!(f.state.canvas.borrow().frame_style, default);
        assert!(!history::step(f.owner.0, &f.state, false).unwrap());
        assert!(history::step(f.owner.0, &f.state, true).unwrap());
        assert_eq!(f.state.canvas.borrow().frame_style, red);
        assert!(f.state.error.borrow().is_none());
    }
}
#[test]
fn settings_width_and_opacity_apply_real_geometry_and_undo_with_annotations_in_one_order() {
    unsafe {
        let f = Fixture::new();
        let window = open(&f);
        let before = history::frame(f.owner.0).unwrap();
        let dpi = GetDpiForWindow(f.owner.0).max(96);
        enter(&f, field(window, settings::WIDTH), "640");
        dispatch(&f);
        let after = crate::desktop::pin_settings_geometry::width(before, 640, dpi).unwrap();
        assert_eq!(history::frame(f.owner.0).unwrap(), after);
        let mut height = [0; 16];
        let n = GetWindowTextW(field(window, settings::HEIGHT), &mut height) as usize;
        assert_eq!(
            String::from_utf16_lossy(&height[..n]),
            (f64::from(after.height) * 96.0 / f64::from(dpi))
                .round()
                .to_string()
        );
        enter(&f, field(window, settings::OPACITY), "50");
        dispatch(&f);
        assert_eq!(f.state.canvas.borrow().normal_opacity, 0.5);
        {
            let mut c = f.state.canvas.borrow_mut();
            c.editor
                .begin(
                    crate::editor::Tool::Brush,
                    crate::viewport::Point { x: 1.0, y: 1.0 },
                )
                .unwrap();
            assert!(c.editor.commit());
        }
        for _ in 0..2 {
            crate::desktop::context_menu::execute(
                f.owner.0,
                &f.state,
                crate::desktop::context_menu::UNDO,
            )
            .unwrap();
        }
        assert!(f.state.canvas.borrow().editor.elements().is_empty());
        assert_eq!(f.state.canvas.borrow().normal_opacity, 1.0);
        assert_eq!(history::frame(f.owner.0).unwrap(), after);
        crate::desktop::context_menu::execute(
            f.owner.0,
            &f.state,
            crate::desktop::context_menu::UNDO,
        )
        .unwrap();
        assert_eq!(history::frame(f.owner.0).unwrap(), before);
        for _ in 0..3 {
            crate::desktop::context_menu::execute(
                f.owner.0,
                &f.state,
                crate::desktop::context_menu::REDO,
            )
            .unwrap();
        }
        assert_eq!(history::frame(f.owner.0).unwrap(), after);
        assert_eq!(f.state.canvas.borrow().window_opacity, 0.5);
        assert_eq!(f.state.canvas.borrow().editor.elements().len(), 1);
        assert!(f.state.error.borrow().is_none());
    }
}
#[test]
fn settings_follow_pin_close_on_tool_change_and_reject_old_edit_without_disabling_other_tools() {
    unsafe {
        let f = Fixture::new();
        let window = open(&f);
        let mut rect = RECT::default();
        GetWindowRect(window, &mut rect).unwrap();
        let mut pin = RECT::default();
        GetWindowRect(f.owner.0, &mut pin).unwrap();
        assert_eq!(rect.right, pin.left);
        assert_eq!(rect.top, pin.top);
        SetWindowPos(
            f.owner.0,
            None,
            500,
            600,
            0,
            0,
            SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
        )
        .unwrap();
        GetWindowRect(window, &mut rect).unwrap();
        GetWindowRect(f.owner.0, &mut pin).unwrap();
        let work = crate::desktop::tool_panel::position::work_area(&pin).unwrap();
        let expected = crate::desktop::pin_settings_geometry::place(
            pin,
            work,
            (rect.right - rect.left, rect.bottom - rect.top),
        );
        assert_eq!(
            rect, expected,
            "settings must follow within the actual work area"
        );
        enter(&f, field(window, settings::OPACITY), "30");
        f.state
            .canvas
            .borrow_mut()
            .select_tool(crate::editor::Tool::Brush);
        dispatch(&f);
        assert_eq!(f.state.canvas.borrow().window_opacity, 1.0);
        assert!(!settings::is_open(&f.state));
        assert!(!IsWindowVisible(window).as_bool());
        assert!(control_ids().iter().all(|id| IsWindowEnabled(
            GetDlgItem(Some(f.panel.0), *id).unwrap()
        )
        .as_bool()
            || *id == catalog::OCR as i32
            || *id == catalog::TRANSLATE as i32
            || *id == 0x1007
            || *id == 0x1008
            || *id == catalog::RASTER as i32));
    }
}
#[test]
fn setting_opacity_undo_does_not_restore_unrelated_window_position_and_fit_shrinks_only() {
    unsafe {
        let f = Fixture::new();
        let window = open(&f);
        let before = history::frame(f.owner.0).unwrap();
        SendMessageW(field(window, settings::FIT), BM_CLICK, None, None);
        dispatch(&f);
        let fitted = history::frame(f.owner.0).unwrap();
        assert_eq!((fitted.x, fitted.y), (before.x, before.y));
        assert_eq!(fitted.width, fitted.height);
        assert!(fitted.width <= before.width && fitted.height <= before.height);
        enter(&f, field(window, settings::OPACITY), "30");
        dispatch(&f);
        SetWindowPos(
            f.owner.0,
            None,
            500,
            300,
            0,
            0,
            SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
        )
        .unwrap();
        assert!(history::step(f.owner.0, &f.state, false).unwrap());
        let moved = history::frame(f.owner.0).unwrap();
        assert_eq!((moved.x, moved.y), (500, 300));
        assert_eq!(f.state.canvas.borrow().window_opacity, 1.0);
    }
}
#[test]
fn lazy_settings_inherit_capture_exclusion_and_ctrl_e_from_input_closes_all_editing() {
    unsafe {
        let f = Fixture::new();
        crate::desktop::capture_protection::toggle(f.owner.0, &f.state);
        let window = open(&f);
        let mut affinity = 0;
        GetWindowDisplayAffinity(window, &mut affinity).unwrap();
        assert_eq!(affinity, WDA_EXCLUDEFROMCAPTURE.0);
        crate::desktop::capture_protection::toggle(f.owner.0, &f.state);
        GetWindowDisplayAffinity(window, &mut affinity).unwrap();
        assert_eq!(affinity, WDA_NONE.0);
        let mut old = [0; 256];
        GetKeyboardState(&mut old).unwrap();
        let mut keys = [0; 256];
        keys[VK_CONTROL.0 as usize] = 0x80;
        SetKeyboardState(&keys).unwrap();
        let handled = tool_panel::translate(
            &f.state,
            &MSG {
                hwnd: field(window, settings::WIDTH),
                message: WM_KEYDOWN,
                wParam: WPARAM(0x45),
                ..Default::default()
            },
        );
        SetKeyboardState(&old).unwrap();
        assert!(handled);
        assert!(!IsWindowVisible(window).as_bool());
        assert!(!IsWindowVisible(f.panel.0).as_bool());
        assert_eq!(f.state.canvas.borrow().tool, crate::editor::Tool::Pan);
        drop(f);
        assert!(!IsWindow(Some(window)).as_bool());
    }
}

#[test]
fn presentation_history_retains_single_and_multiple_annotation_selection() {
    unsafe {
        let f = Fixture::new();
        open(&f);
        {
            let mut c = f.state.canvas.borrow_mut();
            for x in [4.0, 24.0] {
                c.editor
                    .begin(
                        crate::editor::Tool::Line,
                        crate::viewport::Point { x, y: 4.0 },
                    )
                    .unwrap();
                c.editor
                    .extend(crate::viewport::Point {
                        x: x + 12.0,
                        y: 20.0,
                    })
                    .unwrap();
                assert!(c.editor.commit());
            }
        }
        for selected in [vec![1], vec![0, 1]] {
            {
                let mut c = f.state.canvas.borrow_mut();
                let revision = c.editor.revision();
                c.selected = Some((selected[0], revision));
                c.multi_selection = (selected.len() > 1).then_some((selected.clone(), revision));
            }
            assert!(history::opacity(f.owner.0, &f.state, 0.5).unwrap());
            assert_eq!(f.state.canvas.borrow().selected_indices(), selected);
            assert!(history::step(f.owner.0, &f.state, false).unwrap());
            assert_eq!(f.state.canvas.borrow().selected_indices(), selected);
            assert!(history::step(f.owner.0, &f.state, true).unwrap());
            assert_eq!(f.state.canvas.borrow().selected_indices(), selected);
            assert!(history::opacity(f.owner.0, &f.state, 1.0).unwrap());
        }
    }
}

#[test]
fn mini_opacity_history_keeps_view_stamp_and_normal_opacity_independent() {
    unsafe {
        let f = Fixture::new();
        {
            use crate::{
                crop_geometry::Size,
                session_minified::{View, Views},
            };
            let mut c = f.state.canvas.borrow_mut();
            let view = View {
                size: Size {
                    width: 64.0,
                    height: 64.0,
                },
                offset: (0.0, 0.0),
            };
            c.minified = Some(
                crate::desktop::canvas::minified::Presentation::restored(
                    c.image.clone(),
                    Views {
                        image: view,
                        annotations: view,
                    },
                    Size {
                        width: 32.0,
                        height: 32.0,
                    },
                    &c.editor,
                )
                .unwrap(),
            );
            c.minified_opacity = 0.8;
            c.window_opacity = 0.8;
        }
        let window = open(&f);
        enter(&f, field(window, settings::OPACITY), "30");
        dispatch(&f);
        for (redo, expected) in [(None, 0.3), (Some(false), 0.8), (Some(true), 0.3)] {
            if let Some(redo) = redo {
                assert!(history::step(f.owner.0, &f.state, redo).unwrap());
            }
            let c = f.state.canvas.borrow();
            assert!(c.minified.as_ref().unwrap().matches(&c.editor));
            assert_eq!(c.window_opacity, expected);
            assert_eq!(c.minified_opacity, expected);
            assert_eq!(c.normal_opacity, 1.0);
        }
    }
}
