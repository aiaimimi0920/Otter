//! 自有 HWND/原 Application 事务；不注册热键、不改前台、不注入物理输入。
use super::*;
pub(super) unsafe fn app() -> Application {
    let instance = HINSTANCE(
        windows::Win32::System::LibraryLoader::GetModuleHandleW(None)
            .unwrap()
            .0,
    );
    let class = WNDCLASSW {
        hInstance: instance,
        lpszClassName: w!("Otter.Native.Workspace"),
        lpfnWndProc: Some(desktop::procedure::window_proc),
        ..Default::default()
    };
    let registered = RegisterClassW(&class);
    assert!(registered != 0 || GetLastError() == ERROR_CLASS_ALREADY_EXISTS);
    Application::new(
        instance,
        DocumentWindow::create(instance, State::new(None, false).unwrap(), false).unwrap(),
    )
}
pub(in crate::desktop) unsafe fn catalog() -> Rc<Catalog> {
    let app = app();
    Catalog::capture(&app.documents, &app.live_sessions, &app.workspace)
}
#[path = "workspace_chrome_window_tests.rs"]
mod window_tests;
#[test]
fn hidden_runtime_does_not_create_any_chrome_or_retain_application_through_a_cycle() {
    unsafe {
        let mut app = app();
        let binding = app.chrome.bind();
        app.sync_workspace_chrome(false);
        assert!(app.chrome.windows.is_none());
        assert!(app.chrome.idle());
        drop(app);
        assert!(binding.upgrade().is_none());
    }
}
#[test]
fn shared_link_commit_rejects_cycle_and_stale_catalog_without_deleting_old_edge() {
    unsafe {
        let mut app = app();
        app.add_window(false).unwrap();
        let catalog = Catalog::capture(&app.documents, &app.live_sessions, &app.workspace);
        let link = catalog.make_link(0, 1, 0, 0).unwrap().unwrap();
        app.apply_link_creation(desktop::link_catalog::Creation {
            catalog: catalog.clone(),
            link,
        });
        assert_eq!(app.workspace.links().len(), 1);
        let original = app.workspace.links()[0].clone();
        let stale = catalog.make_link(1, 0, 0, 0).unwrap().unwrap();
        app.apply_link_creation(desktop::link_catalog::Creation {
            catalog,
            link: stale,
        });
        assert_eq!(app.workspace.links(), std::slice::from_ref(&original));
        let fresh = Catalog::capture(&app.documents, &app.live_sessions, &app.workspace);
        let cycle = fresh.make_link(1, 0, 0, 0).unwrap().unwrap();
        app.apply_link_creation(desktop::link_catalog::Creation {
            catalog: fresh,
            link: cycle,
        });
        assert_eq!(app.workspace.links(), [original]);
        app.shutdown();
    }
}
#[test]
fn locked_target_cannot_be_connected_by_either_shared_creation_entry() {
    unsafe {
        let mut app = app();
        app.add_window(false).unwrap();
        let catalog = Catalog::capture(&app.documents, &app.live_sessions, &app.workspace);
        let link = catalog.make_link(0, 1, 0, 0).unwrap().unwrap();
        app.documents[1].state.group_locked.set(true);
        // Workspace group lock normally updates both owners; the direct State flag is also authoritative.
        *app.documents[1].state.link_creation.borrow_mut() =
            Some(desktop::link_catalog::Creation { catalog, link });
        app.pending_link_changes();
        assert!(app.workspace.links().is_empty());
        app.shutdown();
    }
}
#[test]
fn locked_source_and_hidden_target_cannot_use_the_chrome_entry() {
    unsafe {
        let mut app = app();
        app.add_window(false).unwrap();
        for source_locked in [true, false] {
            app.documents[0].state.group_locked.set(source_locked);
            app.documents[1].state.group_hidden.set(!source_locked);
            let catalog = Catalog::capture(&app.documents, &app.live_sessions, &app.workspace);
            let link = catalog.make_link(0, 1, 0, 0).unwrap().unwrap();
            app.apply_link_creation(desktop::link_catalog::Creation { catalog, link });
            assert!(app.workspace.links().is_empty());
        }
        app.shutdown();
    }
}
#[test]
fn clean_mode_controls_bound_live_but_does_not_change_an_independent_host() {
    let owner = Owner::default();
    let binding = RefCell::new(owner.bind());
    for (mode, visible) in [(Mode::All, true), (Mode::Border, true), (Mode::None, false)] {
        owner.ui.mode.set(mode);
        assert_eq!(live_frame(&binding), visible);
    }
    drop(owner);
    assert!(live_frame(&binding));
}
#[test]
fn same_error_is_not_queued_again_after_reporting() {
    let ui = Ui::default();
    ui.fail(Error::from(E_OUTOFMEMORY));
    assert_eq!(
        ui.failure.borrow_mut().take().unwrap().code(),
        E_OUTOFMEMORY
    );
    ui.reported.set(Some(E_OUTOFMEMORY));
    ui.fail(Error::from(E_OUTOFMEMORY));
    assert!(ui.failure.borrow().is_none());
    ui.fail(Error::from(E_ABORT));
    assert_eq!(ui.failure.borrow().as_ref().unwrap().code(), E_ABORT);
}
