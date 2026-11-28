//! One-shot background journal owner after production-profile startup; check-mode isolation, failure-state retention, and lease release on exit.
use super::*;
use crate::desktop::application::projection::profile::Owner as ProjectionOwner;
use crate::device_authorization_config::Config;
use crate::projection_delivery::{accept::tests::invitation, journal::Store};
use std::sync::Arc;
const ORIGIN: &str = "https://loom.example";
fn seed(directory: &Directory) -> Vec<u8> {
    let store = Store::open(&directory.0).unwrap();
    drop(store.begin(&invitation(ORIGIN), "device-old").unwrap());
    std::fs::read(directory.0.join("projection-receive.json")).unwrap()
}
unsafe fn poll(app: &mut Application) {
    let end = Instant::now() + Duration::from_secs(5);
    loop {
        app.process_authorization_with_projection(Config::from_env);
        if !matches!(app.projection_profile, ProjectionOwner::Opening(_)) {
            break;
        }
        assert!(Instant::now() < end, "profile preparation deadline");
        std::thread::yield_now();
    }
}
#[test]
fn ordinary_startup_opens_shared_store_once_and_switching_services_does_not_reopen_it() {
    unsafe {
        run(|instance| {
            let directory = Directory::new();
            let bytes = seed(&directory);
            let mut app = new_app(instance);
            app.install_projection_fixture(Profile::open(&directory.0).unwrap())
                .unwrap();
            poll(&mut app);
            let entry = app.authorization.borrow().projection.recovery.entries[0].clone();
            assert_eq!(entry.device(), "device-old");
            assert!(Store::open(&directory.0).is_err());
            app.projection_profile.start(
                directory.0.join("must-not-open"),
                app.execution_wake.callback(),
            );
            assert!(!directory.0.join("must-not-open").exists());
            for origin in [ORIGIN, "https://other.example", ORIGIN] {
                app.authorization.borrow_mut().request = Some(
                    crate::desktop::device_authorization_ui::Request::Select(origin.into()),
                );
                app.process_authorization_with_projection(Config::from_env);
                assert!(Arc::ptr_eq(
                    &entry,
                    &app.authorization.borrow().projection.recovery.entries[0]
                ));
                assert_eq!(
                    std::fs::read(directory.0.join("projection-receive.json")).unwrap(),
                    bytes
                );
            }
            assert!(!app.remote_sessions.projection_pending());
            drop(entry);
            app.shutdown();
            assert!(app
                .authorization
                .borrow()
                .projection
                .recovery
                .entries
                .is_empty());
            assert!(Store::open(&directory.0).is_ok());
        });
    }
}
#[test]
fn workspace_check_and_explicit_non_profile_startups_do_not_discover_a_projection_store() {
    unsafe {
        run(|instance| {
            let directory = Directory::new();
            let mut app = new_app(instance);
            app.install_workspace_checkpoint(Some(Profile::open(&directory.0).unwrap()), true)
                .unwrap();
            app.process_authorization_with_projection(Config::from_env);
            assert!(matches!(app.projection_profile, ProjectionOwner::Disabled));
            assert!(!directory.0.join("projection-receive.lock").exists());
            assert!(!directory.0.join("projection-receive.json").exists());
            app.shutdown();
            let mut app = new_app(instance);
            app.install_workspace_checkpoint(None, false).unwrap();
            app.process_authorization_with_projection(Config::from_env);
            assert!(matches!(app.projection_profile, ProjectionOwner::Disabled));
            app.shutdown();
        });
    }
}
#[test]
fn invalid_or_busy_journal_disables_only_recovery_without_overwriting_or_retrying() {
    unsafe {
        run(|instance| {
            for busy in [false, true] {
                let directory = Directory::new();
                let lock = busy.then(|| Store::open(&directory.0).unwrap());
                let path = directory.0.join("projection-receive.json");
                std::fs::write(&path, b"{unknown-document}").unwrap();
                let mut app = new_app(instance);
                app.install_projection_fixture(Profile::open(&directory.0).unwrap())
                    .unwrap();
                poll(&mut app);
                assert_eq!(
                    app.authorization.borrow().projection.recovery.status,
                    crate::ui_text::format(
                        "app-projection-recovery-unavailable",
                        &[("error", if busy { "Lock" } else { "Format" }.into())]
                    )
                );
                assert!(app
                    .authorization
                    .borrow()
                    .projection
                    .recovery
                    .entries
                    .is_empty());
                assert!(app
                    .workspace_checkpoint
                    .as_ref()
                    .is_some_and(|owner| !owner.stopped));
                assert_eq!(std::fs::read(&path).unwrap(), b"{unknown-document}");
                drop(lock);
                app.projection_profile.start(
                    directory.0.join("must-not-open"),
                    app.execution_wake.callback(),
                );
                assert!(!directory.0.join("must-not-open").exists());
                app.shutdown();
            }
        });
    }
}
#[test]
fn journal_completion_notifies_from_worker_and_shutdown_reaps_an_unpolled_result() {
    let directory = Directory::new();
    let _bytes = seed(&directory);
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    let mut owner = ProjectionOwner::default();
    let ui = std::thread::current().id();
    owner.start(
        directory.0.clone(),
        Arc::new(move || {
            tx.send(std::thread::current().id()).unwrap();
        }),
    );
    assert_ne!(rx.recv_timeout(Duration::from_secs(5)).unwrap(), ui);
    owner.close();
    assert!(Store::open(&directory.0).is_ok());
}
