//! 真实隐藏 Application 的 scope 分配/提交/恢复；沿原检查点与保存事务，不联网。
use super::*;
use crate::session_projection_scope::{Proof, Scope, FIELD};

fn token(bytes: &[u8]) -> Scope {
    crate::session_document::decode(bytes)
        .unwrap()
        .projection_scope
        .unwrap()
}
unsafe fn import(app: &mut Application, bytes: Vec<u8>) {
    app.request_restore(
        bytes,
        None,
        crate::session_placement::Space {
            origin: (0, 0),
            scale: 1.0,
        },
    )
    .unwrap();
    let end = Instant::now() + Duration::from_secs(10);
    loop {
        app.process_restore(false).unwrap();
        if app.restore_job.is_none() {
            break;
        }
        assert!(Instant::now() < end, "scope restore deadline");
        std::thread::yield_now();
    }
}
#[test]
fn generation_and_staging_do_not_grant_scope_but_commit_and_restart_do() {
    unsafe {
        run(|instance| {
            let directory = Directory::new();
            let mut app = new_app(instance);
            app.install_workspace_checkpoint(Some(Profile::open(&directory.0).unwrap()), true)
                .unwrap();
            assert!(app.projection_sender_scope().is_none());
            let original = app.restored_session.as_ref().unwrap().original.clone();
            let (receipt, data) = app.capture_session_snapshot().unwrap();
            let generated = app
                .restored_session
                .as_ref()
                .unwrap()
                .projection_scope
                .unwrap();
            let candidate = data.encode(&AtomicBool::new(false)).unwrap();
            assert_eq!(token(&candidate.bytes), generated);
            assert!(receipt.matches(&app));
            assert!(app.projection_sender_scope().is_none());
            assert!(!directory.path().exists());
            app.start_checkpoint().unwrap();
            ready(&app);
            assert!(
                app.projection_sender_scope().is_none(),
                "staged bytes are not a commit"
            );
            app.poll_checkpoint().unwrap();
            assert_eq!(app.projection_sender_scope(), Some(generated));
            assert!(original.same_storage(&app.restored_session.as_ref().unwrap().original));
            let first = std::fs::read(directory.path()).unwrap();
            app.documents[0].state.canvas.borrow_mut().normal_opacity = 0.5;
            settle(&mut app);
            let second = std::fs::read(directory.path()).unwrap();
            assert_ne!(first, second);
            assert_eq!(
                token(&second),
                generated,
                "memory original did not contain the newly committed field"
            );
            drop(app);
            let mut reopened = new_app(instance);
            reopened
                .install_workspace_checkpoint(Some(Profile::open(&directory.0).unwrap()), true)
                .unwrap();
            assert_eq!(reopened.projection_sender_scope(), Some(generated));
            assert!(reopened
                .restored_session
                .as_ref()
                .unwrap()
                .saved_path
                .is_none());
            assert_eq!(std::fs::read(directory.path()).unwrap(), second);
        });
    }
}
#[test]
fn legacy_workspaces_reusing_path_and_unit_do_not_share_scope() {
    unsafe {
        run(|instance| {
            let directory = Directory::new();
            let mut app = new_app(instance);
            app.install_workspace_checkpoint(Some(Profile::open(&directory.0).unwrap()), true)
                .unwrap();
            let legacy = br#"{"stickers":[{"id":"reused-unit","x":0,"y":0,"w":4,"h":4}],"links":[],"future":{"keep":true}}"#.to_vec();
            import(&mut app, legacy.clone());
            assert!(app.projection_sender_scope().is_none());
            settle(&mut app);
            let a = app.projection_sender_scope().unwrap();
            let a_bytes = std::fs::read(directory.path()).unwrap();
            import(&mut app, legacy);
            assert!(
                app.projection_sender_scope().is_none(),
                "same fixed path cannot lend its old scope"
            );
            settle(&mut app);
            let b = app.projection_sender_scope().unwrap();
            assert_ne!(a, b);
            let saved: serde_json::Value =
                serde_json::from_slice(&std::fs::read(directory.path()).unwrap()).unwrap();
            assert_eq!(saved["future"]["keep"], true);
            assert_eq!(saved["stickers"][0]["id"], "reused-unit");
            import(&mut app, a_bytes);
            assert!(
                app.projection_sender_scope().is_none(),
                "imported token is not this profile's commit"
            );
            settle(&mut app);
            assert_eq!(
                app.projection_sender_scope(),
                Some(a),
                "opening the same logical workspace preserves its identity"
            );
        });
    }
}
#[test]
fn legacy_default_restore_is_migrated_once_without_a_user_edit() {
    unsafe {
        run(|instance| {
            let directory = Directory::new();
            let legacy = br#"{"stickers":[],"links":[],"extension":true,"otterNativeCoordinateSpace":{"version":1,"origin":[0,0],"scale":1}}"#;
            std::fs::write(directory.path(), legacy).unwrap();
            let mut app = new_app(instance);
            app.install_workspace_checkpoint(Some(Profile::open(&directory.0).unwrap()), true)
                .unwrap();
            assert!(app.projection_sender_scope().is_none());
            settle(&mut app);
            let generated = app.projection_sender_scope().unwrap();
            let first = std::fs::read(directory.path()).unwrap();
            assert_eq!(token(&first), generated);
            assert_eq!(
                serde_json::from_slice::<serde_json::Value>(&first).unwrap()["extension"],
                true
            );
            settle(&mut app);
            assert_eq!(
                std::fs::read(directory.path()).unwrap(),
                first,
                "no recurring migration/write"
            );
        });
    }
}
#[test]
fn imported_file_baseline_and_save_as_keep_the_same_logical_scope() {
    unsafe {
        run(|instance| {
            let directory = Directory::new();
            let manual = directory.0.join("manual.json");
            let copy = directory.0.join("copy.json");
            let legacy = br#"{"stickers":[],"links":[],"extension":{"keep":true}}"#;
            std::fs::write(&manual, legacy).unwrap();
            let mut app = new_app(instance);
            app.request_restore_input(
                crate::desktop::session_job::Input::File(manual.clone()),
                None,
                Some(crate::session_placement::Space {
                    origin: (0, 0),
                    scale: 1.0,
                }),
            )
            .unwrap();
            let end = Instant::now() + Duration::from_secs(10);
            while app.restore_job.is_some() {
                app.process_restore(false).unwrap();
                assert!(Instant::now() < end);
                std::thread::yield_now();
            }
            let original = app.restored_session.as_ref().unwrap().original.clone();
            app.install_workspace_checkpoint(Some(Profile::open(&directory.0).unwrap()), true)
                .unwrap();
            settle(&mut app);
            let generated = app.projection_sender_scope().unwrap();
            assert_eq!(std::fs::read(&manual).unwrap(), legacy);
            assert!(original.same_storage(&app.restored_session.as_ref().unwrap().original));
            app.request_save_existing().unwrap();
            ready(&app);
            app.take_save_candidate()
                .unwrap()
                .unwrap()
                .commit_ready(&mut app, &AtomicBool::new(false))
                .unwrap();
            assert_eq!(token(&std::fs::read(&manual).unwrap()), generated);
            assert!(
                app.projection_sender_scope().is_none(),
                "new Source awaits canonical checkpoint"
            );
            settle(&mut app);
            crate::desktop::application::frozen_menu::reference::smoke::save(
                &mut app,
                Some(copy.clone()),
            )
            .unwrap();
            assert_eq!(token(&std::fs::read(&copy).unwrap()), generated);
            assert_eq!(token(&std::fs::read(&manual).unwrap()), generated);
            assert!(app.projection_sender_scope().is_none());
            settle(&mut app);
            assert_eq!(app.projection_sender_scope(), Some(generated));
        });
    }
}
#[test]
fn stale_cancelled_and_conflicted_checkpoint_never_grant_or_refresh_scope() {
    unsafe {
        run(|instance| {
            let directory = Directory::new();
            let mut app = new_app(instance);
            app.install_workspace_checkpoint(Some(Profile::open(&directory.0).unwrap()), true)
                .unwrap();
            app.start_checkpoint().unwrap();
            ready(&app);
            app.documents[0].state.canvas.borrow_mut().normal_opacity = 0.5;
            app.poll_checkpoint().unwrap();
            assert!(!directory.path().exists());
            assert!(app.projection_sender_scope().is_none());
            app.start_checkpoint().unwrap();
            ready(&app);
            app.cancel_save_candidate();
            assert!(app.poll_checkpoint().is_err());
            assert!(app.projection_sender_scope().is_none());
            assert!(!directory.path().exists());
            settle(&mut app);
            assert!(app.projection_sender_scope().is_some());
            std::fs::write(directory.path(), b"external sentinel").unwrap();
            app.documents[0].state.canvas.borrow_mut().normal_opacity = 0.6;
            let end = Instant::now() + Duration::from_secs(10);
            while !app.workspace_checkpoint.as_ref().unwrap().stopped {
                app.pending_workspace_save_with(|_, _, _, _| {});
                assert!(Instant::now() < end);
                std::thread::sleep(Duration::from_millis(1));
            }
            assert!(app.projection_sender_scope().is_none());
            assert_eq!(
                std::fs::read(directory.path()).unwrap(),
                b"external sentinel"
            );
        });
    }
}
#[test]
fn candidate_scope_proof_cannot_be_reused_for_different_bytes() {
    unsafe {
        run(|instance| {
            let directory = Directory::new();
            let mut app = new_app(instance);
            app.request_save_new(directory.0.join("manual.json"))
                .unwrap();
            ready(&app);
            let mut prepared = app.take_save_candidate().unwrap().unwrap();
            let source = prepared.candidate.bytes.clone();
            assert!(prepared
                .checkpoint_scope
                .as_ref()
                .unwrap()
                .is_for(&source.clone().into()));
            prepared.candidate.bytes = source.to_vec().into();
            assert!(prepared
                .commit_ready(&mut app, &AtomicBool::new(false))
                .is_err());
            assert!(!directory.0.join("manual.json").exists());
            assert!(app.projection_sender_scope().is_none());
        });
    }
}
#[test]
fn scope_change_invalidates_receipt_and_cannot_overwrite_an_existing_token() {
    unsafe {
        run(|instance| {
            let mut app = new_app(instance);
            let (receipt, data) = app.capture_session_snapshot().unwrap();
            let candidate = data.encode(&AtomicBool::new(false)).unwrap();
            let original_scope = token(&candidate.bytes);
            app.restored_session.as_mut().unwrap().projection_scope =
                Some(Scope::generate().unwrap());
            assert!(!receipt.matches(&app));
            app.restored_session
                .as_mut()
                .unwrap()
                .edit_original(candidate.bytes.into());
            let (_, data) = app.capture_session_snapshot().unwrap();
            assert!(data.encode(&AtomicBool::new(false)).is_err());
            assert_ne!(
                app.restored_session.as_ref().unwrap().projection_scope,
                Some(original_scope)
            );
            let raw = app.restored_session.as_ref().unwrap().original.clone();
            assert!(Proof::capture(&raw).is_some());
            assert_eq!(
                serde_json::from_slice::<serde_json::Value>(&raw).unwrap()[FIELD],
                original_scope.to_string()
            );
        });
    }
}

#[test]
fn manual_save_alone_and_readonly_recovery_do_not_lend_canonical_scope() {
    unsafe {
        run(|instance| {
            let directory = Directory::new();
            let mut app = new_app(instance);
            let manual = directory.0.join("manual.json");
            crate::desktop::application::frozen_menu::reference::smoke::save(
                &mut app,
                Some(manual.clone()),
            )
            .unwrap();
            let generated = token(&std::fs::read(&manual).unwrap());
            assert!(app.projection_sender_scope().is_none());
            app.install_workspace_checkpoint(Some(Profile::open(&directory.0).unwrap()), true)
                .unwrap();
            settle(&mut app);
            assert_eq!(app.projection_sender_scope(), Some(generated));
            let committed = std::fs::read(directory.path()).unwrap();
            drop(app);
            let recovery = directory
                .0
                .join("workspace.json.otter-recovery-00000000-0000-0000-0000-000000000001.json");
            std::fs::rename(directory.path(), &recovery).unwrap();
            let mut recovered = new_app(instance);
            recovered
                .install_workspace_checkpoint(Some(Profile::open(&directory.0).unwrap()), true)
                .unwrap();
            assert_eq!(
                recovered
                    .restored_session
                    .as_ref()
                    .unwrap()
                    .projection_scope,
                Some(generated)
            );
            assert!(recovered.workspace_checkpoint.as_ref().unwrap().stopped);
            assert!(recovered.projection_sender_scope().is_none());
            recovered
                .pending_workspace_save_with(|_, _, _, _| panic!("readonly cannot checkpoint"));
            assert!(!directory.path().exists());
            assert_eq!(std::fs::read(&recovery).unwrap(), committed);
            assert!(recovered
                .restored_session
                .as_ref()
                .unwrap()
                .saved_path
                .is_none());
        });
    }
}
