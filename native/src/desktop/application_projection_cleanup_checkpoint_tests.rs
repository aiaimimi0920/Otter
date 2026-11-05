//! 原 checkpoint 删除资格/启动 Opening 竞态/最后 Unit/普通退出；只用自有隐藏 HWND。
use super::*;
use crate::desktop::application::projection::profile::Owner as ProjectionProfile;
use std::sync::Arc;

unsafe fn persist_delete(app: &mut Application) {
    let end = Instant::now() + Duration::from_secs(10);
    while app.projection_cleanup.pending_local() {
        app.process_authorization();
        assert!(Instant::now() < end, "delete marker completion deadline");
        std::thread::yield_now();
    }
    settle(app);
}
#[test]
fn projection_cleanup_opening_profile_does_not_lose_deleted_restored_binding() {
    unsafe {
        run(|instance| {
            let (directory, store, ticket, _, mut app) = fixture(instance);
            settle(&mut app);
            let unit = UnitId::parse(ticket.unit()).unwrap();
            app.shutdown();
            drop((app, store, ticket));
            let mut app = new_app(instance);
            app.install_workspace_checkpoint(Some(Profile::open(&directory.0).unwrap()), false)
                .unwrap();
            assert!(matches!(
                app.projection_profile,
                ProjectionProfile::Opening(_)
            ));
            DestroyWindow(app.source_window(&unit).unwrap()).unwrap();
            app.collect_closed();
            assert!(app.projection_cleanup.pending_local());
            app.start_checkpoint().unwrap();
            assert!(app.save_job.is_none());
            assert!(
                crate::session_document::decode(&std::fs::read(directory.path()).unwrap())
                    .unwrap()
                    .workspace
                    .contains(&unit)
            );
            persist_delete(&mut app);
            assert!(
                !crate::session_document::decode(&std::fs::read(directory.path()).unwrap())
                    .unwrap()
                    .workspace
                    .contains(&unit)
            );
            assert!(app.projection_profile.entries().unwrap()[0].cleanup_pending());
            app.shutdown();
            drop(app);
            assert!(Store::open(&directory.0).unwrap().pending().unwrap()[0].cleanup_pending());
        })
    }
}
#[test]
fn projection_cleanup_last_restored_unit_marks_before_empty_checkpoint_even_while_opening() {
    unsafe {
        run(|instance| {
            let (directory, store, ticket, _, mut app) = fixture(instance);
            let unit = UnitId::parse(ticket.unit()).unwrap();
            DestroyWindow(app.documents[0].window.0).unwrap();
            app.collect_closed();
            settle(&mut app);
            app.shutdown();
            drop((app, store, ticket));
            let mut app = new_app(instance);
            app.install_workspace_checkpoint(Some(Profile::open(&directory.0).unwrap()), false)
                .unwrap();
            assert_eq!(app.documents.len(), 1);
            DestroyWindow(app.source_window(&unit).unwrap()).unwrap();
            assert!(drain_messages()); // 与 run_loop 相同：先消费真实 WM_QUIT，再走终态收尾。
            app.finish_deleted_workspace().unwrap();
            assert!(
                crate::session_document::decode(&std::fs::read(directory.path()).unwrap())
                    .unwrap()
                    .workspace
                    .units()
                    .is_empty()
            );
            let journal: serde_json::Value = serde_json::from_slice(
                &std::fs::read(directory.0.join("projection-receive.json")).unwrap(),
            )
            .unwrap();
            assert_eq!(journal["entries"][0]["cleanup"], true);
            app.shutdown();
        })
    }
}
#[test]
fn projection_cleanup_same_workspace_source_edit_keeps_delete_authority_but_new_workspace_does_not()
{
    unsafe {
        run(|instance| {
            for replace in [false, true] {
                let (directory, store, ticket, _, mut app) = fixture(instance);
                settle(&mut app);
                app.projection_profile = ProjectionProfile::Ready {
                    store: store.clone(),
                    entries: vec![Arc::new(ticket.retry())],
                    policy: Default::default(),
                };
                let unit = UnitId::parse(ticket.unit()).unwrap();
                let original = app.restored_session.as_ref().unwrap().original.to_vec();
                app.restored_session
                    .as_mut()
                    .unwrap()
                    .edit_original(original.into());
                if replace {
                    app.workspace = crate::workspace::Workspace::with_initial(unit.clone());
                }
                DestroyWindow(app.source_window(&unit).unwrap()).unwrap();
                app.collect_closed();
                assert_eq!(app.projection_cleanup.pending_local(), !replace);
                if !replace {
                    persist_delete(&mut app);
                }
                assert_eq!(store.pending().unwrap()[0].cleanup_pending(), !replace);
                app.shutdown();
                assert!(directory.path().exists());
            }
        })
    }
}
#[test]
fn projection_cleanup_ordinary_exit_does_not_mark_stable_binding() {
    unsafe {
        run(|instance| {
            let (directory, store, ticket, _, mut app) = fixture(instance);
            settle(&mut app);
            app.projection_profile = ProjectionProfile::Ready {
                store: store.clone(),
                entries: vec![Arc::new(ticket.retry())],
                policy: Default::default(),
            };
            let before = std::fs::read(directory.0.join("projection-receive.json")).unwrap();
            app.checkpoint_exiting();
            for doc in &app.documents {
                DestroyWindow(doc.window.0).unwrap();
            }
            app.collect_closed();
            assert!(!app.projection_cleanup.pending_local());
            app.shutdown();
            assert_eq!(
                std::fs::read(directory.0.join("projection-receive.json")).unwrap(),
                before
            );
        })
    }
}
