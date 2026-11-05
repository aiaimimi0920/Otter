//! 隐藏 HWND 的真实候选/磁盘提交/重开；暂存、取消、冲突与新工作区都不能借用旧包含证明。
use super::*;
#[path = "application_projection_cleanup_checkpoint_tests.rs"]
mod cleanup;
use crate::projection_delivery::{
    binding::Binding,
    journal::{Store, Ticket},
};
const ORIGIN: &str = "https://loom.example";
unsafe fn fixture(instance: HINSTANCE) -> (Directory, Store, Ticket, Binding, Application) {
    let directory = Directory::new();
    let store = Store::open(&directory.0).unwrap();
    let ticket = store.begin(&png::invitation(ORIGIN), "device-1").unwrap();
    let value = png::verified(ORIGIN).bind_intent(&ticket).unwrap();
    let expected = Binding::new(&value, &ticket).unwrap();
    let mut app = new_app(instance);
    app.install_workspace_checkpoint(Some(Profile::open(&directory.0).unwrap()), true)
        .unwrap();
    append_projection(&mut app, value, &ticket);
    (directory, store, ticket, expected, app)
}
#[test]
fn projection_checkpoint_requires_real_commit_not_append_staging_dirty_or_unit_membership() {
    unsafe {
        run(|instance| {
            let (directory, store, ticket, expected, mut app) = fixture(instance);
            let journal = std::fs::read(directory.0.join("projection-receive.json")).unwrap();
            assert!(app
                .workspace
                .units()
                .iter()
                .any(|unit| unit.as_str() == ticket.unit()));
            assert!(app.projection_checkpoint_inclusion(&ticket).is_none());
            app.start_checkpoint().unwrap();
            ready(&app);
            assert!(!directory.path().exists());
            assert!(app.projection_checkpoint_inclusion(&ticket).is_none());
            app.poll_checkpoint().unwrap();
            assert!(app.projection_checkpoint_inclusion(&ticket) == Some(&expected));
            let bytes: crate::session_source::Source =
                std::fs::read(directory.path()).unwrap().into();
            assert!(
                crate::projection_delivery::binding::checkpoint::Index::capture(&bytes)
                    .unwrap()
                    .includes(&expected)
            );
            assert_eq!(
                std::fs::read(directory.0.join("projection-receive.json")).unwrap(),
                journal
            );
            assert_eq!(store.pending().unwrap().len(), 1);
            assert!(app
                .documents
                .iter()
                .all(|doc| !IsWindowVisible(doc.window.0).as_bool()));
            app.shutdown();
        });
    }
}
#[test]
fn projection_checkpoint_cancel_stale_receipt_and_competing_destination_never_publish_inclusion() {
    unsafe {
        run(|instance| {
            for mode in 0..3 {
                let (directory, store, ticket, _expected, mut app) = fixture(instance);
                let journal = std::fs::read(directory.0.join("projection-receive.json")).unwrap();
                app.start_checkpoint().unwrap();
                ready(&app);
                match mode {
                    0 => app.cancel_save_candidate(),
                    1 => app.documents[0].state.canvas.borrow_mut().normal_opacity = 0.5,
                    2 => std::fs::write(directory.path(), b"competing writer").unwrap(),
                    _ => unreachable!(),
                }
                let result = app.poll_checkpoint();
                assert_eq!(result.is_err(), mode != 1);
                assert!(app.projection_checkpoint_inclusion(&ticket).is_none());
                if mode == 2 {
                    assert_eq!(
                        std::fs::read(directory.path()).unwrap(),
                        b"competing writer"
                    );
                } else {
                    assert!(!directory.path().exists());
                }
                assert_eq!(
                    std::fs::read(directory.0.join("projection-receive.json")).unwrap(),
                    journal
                );
                assert_eq!(store.pending().unwrap().len(), 1);
                app.shutdown();
            }
        });
    }
}
#[test]
fn projection_checkpoint_canonical_restart_reconciles_same_intent_but_not_new_same_projection_intent(
) {
    unsafe {
        run(|instance| {
            let (directory, store, ticket, expected, mut app) = fixture(instance);
            settle(&mut app);
            let original_intent = ticket.intent_id().to_owned();
            app.shutdown();
            drop((app, store, ticket));
            let store = Store::open(&directory.0).unwrap();
            let ticket = store.pending().unwrap().pop().unwrap();
            assert_eq!(ticket.intent_id(), original_intent);
            let mut restored = new_app(instance);
            restored
                .install_workspace_checkpoint(Some(Profile::open(&directory.0).unwrap()), true)
                .unwrap();
            assert!(restored.projection_checkpoint_inclusion(&ticket) == Some(&expected));
            let other = Directory::new();
            let other_store = Store::open(&other.0).unwrap();
            let replacement = other_store
                .begin(&png::invitation(ORIGIN), "device-1")
                .unwrap();
            assert!(restored
                .projection_checkpoint_inclusion(&replacement)
                .is_none());
            assert_eq!(store.pending().unwrap().len(), 1);
            restored.shutdown();
        });
    }
}
#[test]
fn projection_checkpoint_recovery_copy_is_not_a_canonical_committed_inclusion() {
    unsafe {
        run(|instance| {
            let (directory, store, ticket, _expected, mut app) = fixture(instance);
            settle(&mut app);
            app.shutdown();
            drop(app);
            let recovery = directory
                .0
                .join("workspace.json.otter-recovery-01234567-89AB-CDEF-0123-456789ABCDEF.json");
            std::fs::rename(directory.path(), &recovery).unwrap();
            let mut restored = new_app(instance);
            restored
                .install_workspace_checkpoint(Some(Profile::open(&directory.0).unwrap()), true)
                .unwrap();
            assert!(restored
                .workspace
                .units()
                .iter()
                .any(|unit| unit.as_str() == ticket.unit()));
            assert!(restored.workspace_checkpoint.as_ref().unwrap().stopped);
            assert!(restored.projection_checkpoint_inclusion(&ticket).is_none());
            assert!(!directory.path().exists());
            assert!(recovery.exists());
            assert_eq!(store.pending().unwrap().len(), 1);
            restored.shutdown();
        });
    }
}
#[test]
fn projection_checkpoint_scope_and_later_removal_clear_saved_classification_without_retiring_journal(
) {
    unsafe {
        run(|instance| {
            let (directory, store, ticket, _expected, mut app) = fixture(instance);
            settle(&mut app);
            let entry = std::sync::Arc::new(ticket.retry());
            app.projection_profile =
                crate::desktop::application::projection::profile::Owner::Ready {
                    store: store.clone(),
                    entries: vec![entry.clone()],
                    policy: Default::default(),
                };
            app.process_authorization();
            assert!(std::sync::Arc::ptr_eq(
                &app.authorization.borrow().projection.recovery.saved[0],
                &entry
            ));
            let original = app.restored_session.as_ref().unwrap().original.clone();
            app.restored_session
                .as_mut()
                .unwrap()
                .edit_original(original.to_vec().into());
            assert!(app.projection_checkpoint_inclusion(&ticket).is_none());
            app.process_authorization();
            assert!(app
                .authorization
                .borrow()
                .projection
                .recovery
                .saved
                .is_empty());
            // 相同字节的新工作区只有自己完成真实 checkpoint 后才能获得新作用域证明。
            settle(&mut app);
            assert!(app.projection_checkpoint_inclusion(&ticket).is_some());
            let unit = UnitId::parse(ticket.unit()).unwrap();
            DestroyWindow(app.source_window(&unit).unwrap()).unwrap();
            app.collect_closed();
            assert!(app.projection_checkpoint_inclusion(&ticket).is_none());
            let end = Instant::now() + Duration::from_secs(10);
            while app.projection_cleanup.pending_local() {
                app.process_authorization();
                assert!(Instant::now() < end, "deletion intent persistence deadline");
                std::thread::yield_now();
            }
            settle(&mut app);
            app.process_authorization();
            assert!(app
                .authorization
                .borrow()
                .projection
                .recovery
                .saved
                .is_empty());
            let saved = std::fs::read(directory.path()).unwrap();
            assert!(!crate::session_document::decode(&saved)
                .unwrap()
                .workspace
                .contains(&unit));
            assert_eq!(store.pending().unwrap().len(), 1);
            app.shutdown();
        });
    }
}
