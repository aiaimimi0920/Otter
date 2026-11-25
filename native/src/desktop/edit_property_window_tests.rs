//! Wiring from real property HWNDs through toolbar notifications to document transactions, plus stale-target/capture-protection lifecycles.
use super::*;
use crate::desktop::{edit_properties, tool_panel};
use crate::editor::Tool;

unsafe fn property(f: &Fixture, id: u16) -> HWND {
    GetDlgItem(
        Some(neuro_mini_ui::property_window(f.panel.0).unwrap()),
        id as i32 * 4,
    )
    .unwrap()
}
unsafe fn enter(f: &Fixture, child: HWND, value: &str) {
    let popup = neuro_mini_ui::property_window(f.panel.0).unwrap();
    assert!(
        IsWindow(Some(child)).as_bool(),
        "retired EDIT before {value}"
    );
    assert!(
        IsWindowVisible(popup).as_bool(),
        "hidden property popup before {value}: tool={:?} panelVisible={}",
        f.state.canvas.borrow().tool,
        IsWindowVisible(f.panel.0).as_bool()
    );
    assert!(
        neuro_mini_ui::property_contains(f.panel.0, child),
        "foreign EDIT before {value}"
    );
    SetWindowTextW(child, &windows::core::HSTRING::from(value)).unwrap();
    assert!(
        tool_panel::translate(
            &f.state,
            &MSG {
                hwnd: child,
                message: WM_KEYDOWN,
                wParam: WPARAM(0x0d),
                ..Default::default()
            }
        ),
        "property Enter not translated: {value}"
    );
}
unsafe fn dispatch_property(f: &Fixture) {
    let mut message = MSG::default();
    loop {
        assert!(PeekMessageW(
            &mut message,
            Some(f.owner.0),
            super::super::super::EXECUTE,
            super::super::super::EXECUTE,
            PM_REMOVE
        )
        .as_bool());
        let property = message.lParam.0 == neuro_mini_ui::PROPERTY;
        DispatchMessageW(&message);
        if property {
            break;
        }
    }
}
#[test]
fn property_enter_reaches_model_and_stale_queued_edit_cannot_change_next_tool() {
    unsafe {
        let f = Fixture::new();
        f.state.canvas.borrow_mut().select_tool(Tool::Rectangle);
        refresh(&f.state);
        let _ = ShowWindow(f.owner.0, SW_SHOWNOACTIVATE);
        let _ = ShowWindow(f.panel.0, SW_SHOWNOACTIVATE);
        let stamp = f.state.canvas.borrow().editor.document_stamp();
        let child = property(&f, edit_properties::WIDTH);
        enter(&f, child, "20");
        dispatch_property(&f);
        assert_eq!(f.state.canvas.borrow().editor.style().width(), 20.0);
        assert!(stamp.matches(&f.state.canvas.borrow().editor));
        enter(&f, child, "40");
        f.state.canvas.borrow_mut().select_tool(Tool::Brush);
        let brush = f.state.canvas.borrow().editor.style();
        dispatch_property(&f);
        assert_eq!(f.state.canvas.borrow().editor.style(), brush);
        let prop = neuro_mini_ui::property_window(f.panel.0).unwrap();
        f.state.canvas.borrow_mut().select_tool(Tool::Pan);
        refresh(&f.state);
        assert!(!IsWindowVisible(prop).as_bool());
        let _ = ShowWindow(f.panel.0, SW_HIDE);
        let _ = ShowWindow(f.panel.0, SW_SHOWNOACTIVATE);
        assert!(!IsWindowVisible(prop).as_bool());
    }
}
#[test]
fn lazy_property_popup_inherits_capture_protection_and_tracks_later_changes() {
    unsafe {
        let f = Fixture::new();
        assert!(neuro_mini_ui::property_window(f.panel.0).is_none());
        crate::desktop::capture_protection::toggle(f.owner.0, &f.state);
        assert_eq!(f.state.capture_protection.get(), [true; 5]);
        f.state.canvas.borrow_mut().select_tool(Tool::Text);
        refresh(&f.state);
        let prop = neuro_mini_ui::property_window(f.panel.0).unwrap();
        let mut affinity = 0;
        GetWindowDisplayAffinity(prop, &mut affinity).unwrap();
        assert_eq!(affinity, WDA_EXCLUDEFROMCAPTURE.0);
        let font = property(&f, edit_properties::FONT);
        let mut rect = RECT::default();
        GetWindowRect(font, &mut rect).unwrap();
        assert_eq!(
            rect.right - rect.left,
            (110 * GetDpiForWindow(font).max(96) / 96) as i32
        );
        crate::desktop::capture_protection::toggle(f.owner.0, &f.state);
        GetWindowDisplayAffinity(prop, &mut affinity).unwrap();
        assert_eq!(affinity, WDA_NONE.0);
        drop(f);
        assert!(!IsWindow(Some(prop)).as_bool());
    }
}

#[test]
fn real_glow_button_keeps_brush_and_numeric_controls_then_commits_smoothed_glow() {
    unsafe {
        let f = Fixture::new();
        f.state.canvas.borrow_mut().select_tool(Tool::Brush);
        refresh(&f.state);
        let _ = ShowWindow(f.owner.0, SW_SHOWNOACTIVATE);
        let _ = ShowWindow(f.panel.0, SW_SHOWNOACTIVATE);
        let opacity = property(&f, edit_properties::OPACITY);
        let smoothing = property(&f, edit_properties::SMOOTH);
        enter(&f, opacity, "50");
        dispatch_property(&f);
        enter(&f, smoothing, "100");
        dispatch_property(&f);
        let glow = property(&f, edit_properties::GLOW);
        SendMessageW(glow, BM_CLICK, None, None);
        dispatch_property(&f);
        assert_eq!(f.state.canvas.borrow().tool, Tool::Brush);
        assert_eq!(property(&f, edit_properties::OPACITY), opacity);
        assert_eq!(property(&f, edit_properties::SMOOTH), smoothing);
        assert!(IsWindowVisible(opacity).as_bool());
        assert!(IsWindowVisible(smoothing).as_bool());
        let mut c = f.state.canvas.borrow_mut();
        c.viewport = crate::viewport::Viewport {
            origin: Default::default(),
            scale: 1.0,
        };
        let p = |x, y| crate::viewport::Point { x, y };
        c.pointer_down(p(0., 0.));
        c.pointer_move(p(10., 10.));
        c.pointer_up(p(20., 0.));
        assert_eq!(c.editor.elements()[0].tool, Tool::Highlighter);
        assert_eq!(c.editor.elements()[0].points[1].y, 1.25);
        assert_eq!(c.editor.elements()[0].opacity, 0.5);
        drop(c);
        refresh(&f.state);
        SendMessageW(property(&f, edit_properties::GLOW), BM_CLICK, None, None);
        dispatch_property(&f);
        assert!(!f
            .state
            .canvas
            .borrow()
            .tool_profiles
            .brush_highlighter_enabled());
        assert_eq!(f.state.canvas.borrow().tool, Tool::Brush);
    }
}
#[test]
fn ratio_width_height_and_lock_use_one_context_and_survive_shape_switching() {
    unsafe {
        let f = Fixture::new();
        f.state.canvas.borrow_mut().select_tool(Tool::Ellipse);
        refresh(&f.state);
        let _ = ShowWindow(f.owner.0, SW_SHOWNOACTIVATE);
        let _ = ShowWindow(f.panel.0, SW_SHOWNOACTIVATE);
        let popup = neuro_mini_ui::property_window(f.panel.0).unwrap();
        let lock = property(&f, edit_properties::RATIO);
        let input = |part| {
            GetDlgItem(
                Some(popup),
                0x8000 + edit_properties::RATIO as i32 * 8 + part,
            )
            .unwrap()
        };
        let width = input(0);
        let height = input(1);
        enter(&f, width, "16");
        dispatch_property(&f);
        enter(&f, height, "9");
        dispatch_property(&f);
        assert_eq!(
            f.state.canvas.borrow().tool_profiles.shapes.ratio_label(),
            "16:9"
        );
        SendMessageW(lock, BM_CLICK, None, None);
        dispatch_property(&f);
        assert!(!f.state.canvas.borrow().tool_profiles.shapes.lock_aspect);
        f.state.canvas.borrow_mut().select_tool(Tool::Rectangle);
        refresh(&f.state);
        assert_eq!(input(0), width);
        assert_eq!(input(1), height);
        assert_eq!(property(&f, edit_properties::RATIO), lock);
        assert!(IsWindowVisible(property(&f, edit_properties::RADIUS)).as_bool());
        f.state.canvas.borrow_mut().select_tool(Tool::Ellipse);
        refresh(&f.state);
        assert_eq!(input(0), width);
        assert_eq!(property(&f, edit_properties::RATIO), lock);
    }
}
