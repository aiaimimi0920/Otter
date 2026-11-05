//! 真实应用 owner、线程句柄和共享窗口状态；缺失的独立测试 profile 不产生身份或 HTTP。
use super::*;
use crate::desktop::{document_window::DocumentWindow, Apartment};
use std::{os::windows::io::AsRawHandle, path::PathBuf};
use windows::Win32::System::{LibraryLoader::GetModuleHandleW, Threading::WaitForSingleObject};
#[path = "application_projection_inbox_tests.rs"]
mod projection_inbox;
#[path = "application_projection_policy_ui_tests.rs"]
mod projection_policy;

struct Class(HINSTANCE);
impl Drop for Class {
    fn drop(&mut self) {
        unsafe {
            let _ = UnregisterClassW(w!("Otter.Native.Workspace"), Some(self.0));
        }
    }
}
unsafe fn application() -> (Class, Application) {
    let instance: HINSTANCE = GetModuleHandleW(None).unwrap().into();
    assert_ne!(
        RegisterClassW(&WNDCLASSW {
            hInstance: instance,
            lpszClassName: w!("Otter.Native.Workspace"),
            lpfnWndProc: Some(crate::desktop::procedure::window_proc),
            ..Default::default()
        }),
        0
    );
    let document =
        DocumentWindow::create(instance, State::new(None, false).unwrap(), false).unwrap();
    (Class(instance), Application::new(instance, document))
}
fn task(root: &std::path::Path) -> Task {
    Task::Configured {
        config: Config::new("https://127.0.0.1:1", root, "Authorization test").unwrap(),
        action: Action::Renew,
    }
}
fn missing_profile() -> PathBuf {
    let root = PathBuf::from("C:/Users/Public/nas_home/AI/GameEditor/linshi").join(format!(
        "otter-auth-ui-missing-{:?}",
        unsafe { windows::Win32::System::Com::CoCreateGuid() }.unwrap()
    ));
    assert!(!root.exists());
    root
}
#[test]
fn completed_worker_updates_shared_status_without_creating_identity_or_retaining_handle() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let (_class, mut app) = application();
        let state = app.bind_window_state(State::new(None, false).unwrap());
        assert!(std::rc::Rc::ptr_eq(
            &state.authorization,
            &app.authorization
        ));
        let root = missing_profile();
        app.authorization.borrow_mut().origin = Some("https://127.0.0.1:1".into());
        app.authorization.borrow_mut().busy = true;
        app.authorization_owner.start(task(&root)).unwrap();
        {
            let handle = app.authorization_owner.wait_handles().next().unwrap();
            assert_eq!(
                WaitForSingleObject(HANDLE(handle.as_raw_handle()), 5000),
                WAIT_OBJECT_0
            );
        }
        app.process_authorization();
        assert!(!state.authorization.borrow().busy);
        assert!(state
            .authorization
            .borrow()
            .status
            .starts_with("授权请求失败："));
        assert!(app.authorization_owner.finished());
        assert_eq!(app.authorization_owner.wait_handles().count(), 0);
        assert!(!root.exists());
        app.shutdown();
        assert!(state.authorization.borrow().closed);
    }
}
#[test]
fn selection_discards_old_completion_and_shutdown_clears_queued_request_and_reaps_task() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let (_class, mut app) = application();
        let root = missing_profile();
        app.authorization.borrow_mut().origin = Some("https://127.0.0.1:1".into());
        app.authorization_owner.start(task(&root)).unwrap();
        app.authorization.borrow_mut().request =
            Some(Request::Select("https://selected.example".into()));
        app.process_authorization();
        assert_eq!(
            app.authorization.borrow().status,
            "已选择服务，尚未获取授权会话"
        );
        app.authorization_owner.start(task(&root)).unwrap();
        app.authorization.borrow_mut().request = Some(Request::Start(Action::RequestPairing));
        app.shutdown();
        assert!(app.authorization_owner.finished());
        assert!(app.authorization.borrow().request.is_none());
        assert!(app.authorization.borrow().closed);
        assert!(!root.exists());
    }
}

#[test]
fn discovery_requires_live_authorization_and_service_changes_clear_catalog() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let (_class, mut app) = application();
        let origin = "https://127.0.0.1:1";
        app.authorization.borrow_mut().origin = Some(origin.into());
        app.authorization.borrow_mut().request = Some(Request::Discover);
        app.process_authorization();
        assert!(!app.remote_sessions.discovering());
        assert!(!app.authorization.borrow().catalog_busy);
        assert!(app
            .authorization
            .borrow()
            .status
            .starts_with("请先获取有效授权："));
        for request in [
            Request::Cancel,
            Request::Select("https://changed.example".into()),
        ] {
            let mut view = app.authorization.borrow_mut();
            view.catalog = Some(crate::live_relay_catalog_runtime::Catalog {
                origin: origin.into(),
                entries: Vec::new(),
            });
            view.catalog_busy = true;
            view.request = Some(request);
            drop(view);
            app.process_authorization();
            assert!(app.authorization.borrow().catalog.is_none());
            assert!(!app.authorization.borrow().catalog_busy);
        }
        // 后台租约撤销可能正常返回 Stopped；不能让 UI 永远停留在 busy。
        app.authorization.borrow_mut().catalog_busy = true;
        app.poll_remote_live(false).unwrap();
        assert!(!app.authorization.borrow().catalog_busy);
        app.shutdown();
        assert!(app.authorization.borrow().catalog.is_none());
    }
}
