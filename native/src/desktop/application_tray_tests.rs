//! 常驻图标的主循环接续；使用隐藏 Application，注入通知，不操作系统托盘。
use super::*;
use crate::desktop::{tray, Apartment};
use std::cell::Cell;

struct Class {
    instance: HINSTANCE,
    owned: bool,
}
impl Class {
    unsafe fn new() -> Self {
        let instance = HINSTANCE(
            windows::Win32::System::LibraryLoader::GetModuleHandleW(None)
                .unwrap()
                .0,
        );
        let owned = RegisterClassW(&WNDCLASSW {
            hInstance: instance,
            lpszClassName: w!("Otter.Native.Workspace"),
            lpfnWndProc: Some(crate::desktop::procedure::window_proc),
            ..Default::default()
        }) != 0;
        assert!(owned || GetLastError() == ERROR_CLASS_ALREADY_EXISTS);
        Self { instance, owned }
    }
    unsafe fn app(&self) -> Application {
        let document =
            DocumentWindow::create(self.instance, State::new(None, false).unwrap(), false).unwrap();
        Application::new(self.instance, document)
    }
}
impl Drop for Class {
    fn drop(&mut self) {
        if self.owned {
            unsafe {
                assert!(
                    UnregisterClassW(w!("Otter.Native.Workspace"), Some(self.instance)).is_ok()
                );
            }
        }
    }
}

#[test]
fn initial_loop_installs_one_resident_and_closed_owner_transfers_to_a_live_document() {
    unsafe {
        let _apartment = Apartment::new().unwrap();
        let class = Class::new();
        let mut app = class.app();
        let calls = std::cell::RefCell::new(Vec::new());
        let install = |doc: &DocumentWindow| {
            calls.borrow_mut().push(doc.window.0);
            tray::start_with(&doc.state, || Ok(()))
        };
        let first = app.documents[0].window.0;
        assert!(!tray::is_resident(&app.documents[0].state));
        app.ensure_tray_with(install);
        assert!(tray::is_resident(&app.documents[0].state));
        assert!(!tray::is_hidden(&app.documents[0].state));
        app.ensure_tray_with(install);
        app.add_window(false).unwrap();
        app.ensure_tray_with(install);
        assert_eq!(*calls.borrow(), [first]);
        let second = app.documents[1].window.0;
        DestroyWindow(first).unwrap();
        // 即使 collect_closed 尚未释放旧 State，也不能把已销毁的 resident 当成可用。
        app.ensure_tray_with(install);
        app.collect_closed();
        app.ensure_tray_with(install);
        assert_eq!(*calls.borrow(), [first, second]);
        assert!(tray::is_resident(&app.documents[0].state));
    }
}

#[test]
fn whole_owner_replacement_reinstalls_once_without_requiring_a_hide_action() {
    unsafe {
        let _apartment = Apartment::new().unwrap();
        let class = Class::new();
        let mut app = class.app();
        app.ensure_tray_with(|doc| tray::start_with(&doc.state, || Ok(())));
        let previous = app.documents[0].window.0;
        let next = app
            .prepare_window(State::new(None, false).unwrap(), false)
            .unwrap();
        let current = next.window.0;
        let previous = (previous, std::mem::replace(&mut app.documents[0], next));
        drop(previous.1);
        assert!(!IsWindow(Some(previous.0)).as_bool());
        let calls = Cell::new(0);
        app.ensure_tray_with(|doc| {
            assert_eq!(doc.window.0, current);
            calls.set(calls.get() + 1);
            tray::start_with(&doc.state, || Ok(()))
        });
        app.ensure_tray_with(|_| panic!("Replacement must not duplicate resident icons"));
        assert_eq!(calls.get(), 1);
    }
}

#[test]
fn unavailable_shell_is_not_retried_on_every_loop_or_hidden_as_a_side_effect() {
    unsafe {
        let _apartment = Apartment::new().unwrap();
        let class = Class::new();
        let app = class.app();
        let hwnd = app.documents[0].window.0;
        let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        app.ensure_tray_with(|doc| {
            tray::start_with(&doc.state, || {
                Err(Error::new(E_FAIL, "Injected unavailable Shell"))
            })
        });
        app.ensure_tray_with(|_| panic!("No unbounded loop retry"));
        assert!(IsWindowVisible(hwnd).as_bool());
        assert!(!tray::is_hidden(&app.documents[0].state));
    }
}

#[test]
fn deleting_last_pin_recreates_hidden_transient_host_but_explicit_exit_does_not() {
    unsafe {
        let _apartment = Apartment::new().unwrap();
        let class = Class::new();
        let mut app = class.app();
        let old_unit = app.documents[0].state.unit_id.clone();
        DestroyWindow(app.documents[0].window.0).unwrap();
        app.resume_empty_with(|doc| tray::start_with(&doc.state, || Ok(())))
            .unwrap();
        assert_eq!(app.documents.len(), 1);
        let doc = &app.documents[0];
        assert!(doc.is_alive() && tray::is_resident(&doc.state));
        assert!(!IsWindowVisible(doc.window.0).as_bool());
        assert!(!IsWindowVisible(doc.panel.0).as_bool());
        assert!(doc.state.canvas.borrow().image.is_none());
        assert!(!app.workspace.contains(&old_unit));
        assert_eq!(
            app.restored_session
                .as_ref()
                .unwrap()
                .transient_host
                .as_ref(),
            Some(&doc.state.unit_id)
        );
        app.resume_empty_with(|_| panic!("must not create another resident"))
            .unwrap();
        app.exit_committed = true;
        DestroyWindow(app.documents[0].window.0).unwrap();
        app.resume_empty_with(|_| panic!("explicit exit must not restart"))
            .unwrap();
        assert!(app.documents.is_empty());
    }
}
