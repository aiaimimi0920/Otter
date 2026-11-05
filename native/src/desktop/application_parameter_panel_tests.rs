//! Application 真实文档 owner 接线；隐藏窗口，不注册热键/托盘或切换输入 desktop。
use super::*;
#[path = "application_parameter_paint_tests.rs"]
mod paint;
unsafe fn app() -> Application {
    let instance = HINSTANCE(
        windows::Win32::System::LibraryLoader::GetModuleHandleW(None)
            .unwrap()
            .0,
    );
    let registered = RegisterClassW(&WNDCLASSW {
        hInstance: instance,
        lpszClassName: w!("Otter.Native.Workspace"),
        lpfnWndProc: Some(crate::desktop::procedure::window_proc),
        ..Default::default()
    });
    assert!(registered != 0 || GetLastError() == ERROR_CLASS_ALREADY_EXISTS);
    let document =
        DocumentWindow::create(instance, State::new(None, false).unwrap(), false).unwrap();
    Application::new(instance, document)
}
#[test]
fn host_tab_creates_owned_panel_and_capture_list_tray_close_does_not_restore_it() {
    unsafe {
        let mut app = app();
        let hwnd = app.documents[0].window.0;
        crate::desktop::document_window::parameter_panel::dispatch(
            hwnd,
            &app.documents[0].state,
            WM_KEYDOWN,
            WPARAM(9),
            LPARAM(0),
        );
        assert!(app.documents[0].parameters.is_none());
        app.process_parameter_panels(false);
        let panel = app.documents[0].parameters.as_ref().unwrap().handle();
        assert!(app.capture_windows.borrow().contains(&panel));
        crate::desktop::tray::hide_with(hwnd, &app.documents[0].state, || Ok(())).unwrap();
        app.process_parameter_panels(false);
        assert!(app.documents[0].parameters.is_none());
        assert!(!app.capture_windows.borrow().contains(&panel));
        assert!(!IsWindow(Some(panel)).as_bool());
        crate::desktop::tray::restore_with(hwnd, &app.documents[0].state, |_, _| {}, |_| {});
        app.process_parameter_panels(false);
        assert!(app.documents[0].parameters.is_none());
    }
}
#[test]
fn four_panel_budget_is_shared_and_closed_slot_is_reusable_without_changing_document() {
    unsafe {
        let mut app = app();
        for _ in 0..4 {
            app.add_window(false).unwrap();
        }
        let units = app
            .documents
            .iter()
            .map(|d| d.state.unit_id.clone())
            .collect::<Vec<_>>();
        for doc in &app.documents {
            doc.state.parameters_ui.toggle.set(true);
        }
        app.process_parameter_panels(false);
        assert_eq!(
            app.documents
                .iter()
                .filter(|d| d.parameters.is_some())
                .count(),
            4
        );
        assert!(app.documents[4].parameters.is_none());
        app.documents[0].state.parameters_ui.close.set(true);
        app.documents[4].state.parameters_ui.toggle.set(true);
        app.process_parameter_panels(false);
        assert!(app.documents[0].parameters.is_none());
        assert!(app.documents[4].parameters.is_some());
        assert_eq!(
            units,
            app.documents
                .iter()
                .map(|d| d.state.unit_id.clone())
                .collect::<Vec<_>>()
        );
    }
}
