//! Real application owners, thread handles, and shared window state; a missing isolated test profile creates neither an identity nor HTTP traffic.
use super::*;
use crate::desktop::{document_window::DocumentWindow, Apartment};
use std::{os::windows::io::AsRawHandle, path::PathBuf};
use windows::Win32::System::{LibraryLoader::GetModuleHandleW, Threading::WaitForSingleObject};
#[path = "application_projection_inbox_tests.rs"]
mod projection_inbox;
#[path = "application_projection_policy_ui_tests.rs"]
mod projection_policy;

unsafe fn application() -> Application {
    let instance = crate::desktop::workspace_window_fixture::register();
    let document =
        DocumentWindow::create(instance, State::new(None, false).unwrap(), false).unwrap();
    Application::new(instance, document)
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
        let mut app = application();
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
        let mut app = application();
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
        let mut app = application();
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
                device_id: "device".into(),
                entries: Vec::new(),
            });
            view.catalog_busy = true;
            view.request = Some(request);
            drop(view);
            app.process_authorization();
            assert!(app.authorization.borrow().catalog.is_none());
            assert!(!app.authorization.borrow().catalog_busy);
        }
        // Background lease revocation may normally return Stopped; never leave the UI permanently busy.
        app.authorization.borrow_mut().catalog_busy = true;
        app.poll_remote_live(false).unwrap();
        assert!(!app.authorization.borrow().catalog_busy);
        app.shutdown();
        assert!(app.authorization.borrow().catalog.is_none());
    }
}

#[test]
fn headless_remote_poll_reports_all_completed_viewers_and_settles_surface_status() {
    use crate::{
        desktop::composition::relay::session::Session,
        device_authorization_lease::Grant,
        device_session_client::tests::session,
        live_relay_control_runtime::{Runtime, Target},
    };
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let mut app = application();
        let profile = std::env::temp_dir();
        let origins = ["https://127.0.0.1:1", "https://127.0.0.1:2"];
        for origin in origins {
            // A valid test record that is guaranteed expired under the actual wall clock; the thread touches neither files nor the network.
            let mut grant = Grant::new(session(origin, 1));
            let runtime = Runtime::start_configured(
                Config::new(origin, &profile, "Remote settlement test").unwrap(),
                grant.lease(origin, 0).unwrap(),
                Target {
                    session: "live:test".into(),
                    surface: "surface:test".into(),
                    attachment: "attach:test".into(),
                    expected: None,
                    selected: None,
                },
                |_| panic!("expired fixture must not reach the consumer"),
            )
            .unwrap();
            let handle = runtime.wait_handle().unwrap();
            assert_eq!(
                WaitForSingleObject(HANDLE(handle.as_raw_handle()), 5000),
                windows::Win32::Foundation::WAIT_OBJECT_0
            );
            app.remote_sessions
                .insert_for_test(origin.into(), Session::from_runtime(runtime));
        }
        {
            let mut view = app.authorization.borrow_mut();
            view.mount_busy = true;
            view.restoring = true;
            view.recovery_status = "pend".into();
        }
        let error = app.poll_remote_live(false).unwrap_err();
        assert_eq!(error.code(), E_FAIL);
        let message = error.to_string();
        for origin in origins {
            assert!(message.contains(origin), "{message}");
        }
        assert_eq!(
            message.matches("Preparation(Credential)").count(),
            2,
            "{message}"
        );
        let view = app.authorization.borrow();
        assert!(!view.mount_busy && !view.restoring && view.recovery_status.is_empty());
        assert!(view.live.watching.is_empty());
        drop(view);
        assert_eq!(app.remote_sessions.wait_handles().count(), 0);
        app.poll_remote_live(false).unwrap();
        app.shutdown();
    }
}
