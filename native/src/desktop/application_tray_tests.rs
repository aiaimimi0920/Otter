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
fn clipboard_cascade_and_current_target_reach_real_staged_windows() {
    unsafe {
        let _apartment = Apartment::new().unwrap();
        let class = Class::new();
        let mut app = class.app();
        crate::desktop::application::paste_smoke::run(&mut app).unwrap();
        // 可见粘贴已创建附属窗口，后续失败导入仍必须完整回滚。
        assert!(app.capture_windows.borrow().len() > app.documents.len() * 2);
        crate::desktop::application::drop_smoke::run(&mut app).unwrap();
    }
}

#[test]
fn visible_paste_preserves_whole_application_smoke_lifecycle() {
    unsafe {
        let _apartment = Apartment::new().unwrap();
        let class = Class::new();
        smoke::run(class.instance, Default::default()).unwrap();
    }
}

#[test]
#[ignore = "requires a freshly created noninteractive window station with an isolated clipboard"]
fn clipboard_system_publication_preserves_source_cascade() {
    unsafe {
        let _apartment = Apartment::new().unwrap();
        let class = Class::new();
        let mut app = class.app();
        crate::desktop::application::paste_smoke::system::run(&mut app).unwrap();
    }
}

#[test]
fn external_paste_position_does_not_change_initial_scale() {
    unsafe {
        let _apartment = Apartment::new().unwrap();
        let class = Class::new();
        let mut app = class.app();
        let mut pin = crate::desktop::open_image::PreparedDocument::from_image(
            crate::desktop::image::Image {
                width: 2400,
                height: 1600,
                pixels: vec![255; 2400 * 1600 * 4].into(),
            },
            true,
        )
        .unwrap();
        pin.paste_origin = Some(POINT { x: -1000, y: 100 });
        app.add_prepared(vec![pin], false).unwrap();
        let mut rect = RECT::default();
        GetWindowRect(app.documents.last().unwrap().window.0, &mut rect).unwrap();
        assert_eq!(
            (
                rect.left,
                rect.top,
                rect.right - rect.left,
                rect.bottom - rect.top
            ),
            (-1000, 100, 1200, 800)
        );
    }
}

#[test]
fn pasted_pins_start_idle_without_changing_source_toolbar_and_can_be_edited_explicitly() {
    use crate::desktop::{open_image::PreparedDocument, open_job, paste_input};
    use crate::editor::Tool;
    use windows::Win32::UI::Input::KeyboardAndMouse::VK_F4;
    unsafe {
        let _apartment = Apartment::new().unwrap();
        let class = Class::new();
        let mut app = class.app();
        let fixture = smoke::image_fixture::ImageFile::new().unwrap();
        app.add_prepared(
            vec![PreparedDocument::open(fixture.path(), true).unwrap()],
            true,
        )
        .unwrap();
        let source_index = app.documents.len() - 1;
        let source = &app.documents[source_index];
        // 普通打开图片的自动显示规则不变；只让粘贴的新窗口保持收起。
        assert!(IsWindowVisible(source.panel.0).as_bool());
        SendMessageW(source.panel.0, WM_CLOSE, None, None);
        let key = |hwnd| SendMessageW(hwnd, WM_KEYDOWN, Some(WPARAM(VK_F4.0 as usize)), None);
        for (source_open, source_tool) in [(true, Tool::Brush), (false, Tool::Pan)] {
            let source = &app.documents[source_index];
            key(source.window.0);
            source.state.canvas.borrow_mut().select_tool(source_tool);
            let stamp = source.state.canvas.borrow().editor.document_stamp();
            paste_input::deliver_at(
                source.window.0,
                &source.state,
                paste_input::Snapshot::Files(vec![fixture.path().to_owned()]),
                true,
                POINT { x: 200, y: 200 },
            )
            .unwrap();
            open_job::finish(source.window.0, &source.state, true);
            app.pending_windows(true).unwrap();
            let pin = app.documents.last().unwrap();
            paste_smoke::verify_idle(pin).unwrap();
            key(pin.window.0);
            assert!(IsWindowVisible(pin.panel.0).as_bool());
            assert_eq!(pin.state.canvas.borrow().tool, Tool::Select);
            key(pin.window.0);
            paste_smoke::verify_idle(pin).unwrap();
            let source = &app.documents[source_index];
            assert_eq!(IsWindowVisible(source.panel.0).as_bool(), source_open);
            assert_eq!(source.state.canvas.borrow().tool, source_tool);
            assert!(stamp.matches(&source.state.canvas.borrow().editor));
        }
    }
}

#[test]
fn startup_presentation_keeps_empty_host_hidden_but_shows_actual_documents() {
    unsafe {
        let _apartment = Apartment::new().unwrap();
        let class = Class::new();
        let app = class.app();
        let doc = &app.documents[0];
        doc.show_content(SW_SHOWNORMAL);
        assert!(!IsWindowVisible(doc.window.0).as_bool());
        assert!(!IsWindowVisible(doc.panel.0).as_bool());
        app.ensure_tray_with(|doc| tray::start_with(&doc.state, || Ok(())));
        assert!(tray::is_resident(&doc.state));
        doc.state.canvas.borrow_mut().pinned = true;
        doc.show_content(SW_SHOWNOACTIVATE);
        assert!(IsWindowVisible(doc.window.0).as_bool());
        assert!(!IsWindowVisible(doc.panel.0).as_bool());
        let _ = ShowWindow(doc.window.0, SW_HIDE);
        doc.state.group_hidden.set(true);
        doc.show_content(SW_SHOWNORMAL);
        assert!(!IsWindowVisible(doc.window.0).as_bool());
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
