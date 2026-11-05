//! 隐藏 Application 的 sender metadata 持久接缝；原日志与原保存单槽不被候选或暂存退休。
use super::*;
use crate::projection_delivery::sender::binding::{checkpoint::Index, tests as bindings};
use crate::projection_delivery::sender::Intent;
unsafe fn load(app: &mut Application, bytes: Vec<u8>) {
    app.request_restore(
        bytes,
        None,
        crate::session_placement::Space {
            origin: (0, 0),
            scale: 1.0,
        },
    )
    .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while app.restore_job.is_some() {
        app.process_restore(false).unwrap();
        assert!(Instant::now() < deadline, "sender restore deadline");
        std::thread::yield_now();
    }
}
unsafe fn publish_metadata(
    app: &mut Application,
    intent: &Intent,
) -> crate::projection_delivery::sender::binding::Binding {
    let original = app.restored_session.as_ref().unwrap().original.clone();
    let candidate = bindings::candidate(&original, intent, 1, intent.digest());
    assert!(candidate.expected().same_storage(&original));
    assert!(candidate.index().is_for(candidate.source()));
    assert!(!app.projection_sender_inclusion(candidate.binding()));
    app.restored_session
        .as_mut()
        .unwrap()
        .edit_original(candidate.source().clone());
    app.saved_state = None;
    candidate.binding().clone()
}
#[test]
fn candidate_and_staging_are_not_canonical_but_commit_and_restart_include_exact_binding() {
    unsafe {
        run(|instance| {
            let directory = Directory::new();
            let scope = "a".repeat(64);
            let (sender_files, _store, intent) =
                bindings::signed("unit-1", &scope, "https://loom.example");
            let journal = std::fs::read(sender_files.path()).unwrap();
            let mut app = new_app(instance);
            app.install_workspace_checkpoint(Some(Profile::open(&directory.0).unwrap()), true)
                .unwrap();
            load(&mut app, bindings::source("unit-1", &scope).to_vec());
            settle(&mut app);
            let expected = publish_metadata(&mut app, &intent);
            assert!(!app.projection_sender_inclusion(&expected));
            app.start_checkpoint().unwrap();
            ready(&app);
            assert!(!app.projection_sender_inclusion(&expected));
            app.poll_checkpoint().unwrap();
            assert!(app.projection_sender_inclusion(&expected));
            let canonical = std::fs::read(directory.path()).unwrap();
            let index = Index::capture(&canonical.clone().into()).unwrap();
            assert!(index.includes(&expected));
            assert_eq!(std::fs::read(sender_files.path()).unwrap(), journal);
            drop(app);
            let mut restored = new_app(instance);
            restored
                .install_workspace_checkpoint(Some(Profile::open(&directory.0).unwrap()), true)
                .unwrap();
            assert!(restored.projection_sender_inclusion(&expected));
            assert!(restored
                .restored_session
                .as_ref()
                .unwrap()
                .saved_path
                .is_none());
            assert_eq!(std::fs::read(directory.path()).unwrap(), canonical);
            assert_eq!(std::fs::read(sender_files.path()).unwrap(), journal);
        });
    }
}
#[test]
fn replacing_workspace_or_source_and_stopped_checkpoint_cannot_borrow_old_inclusion() {
    unsafe {
        run(|instance| {
            let directory = Directory::new();
            let scope = "a".repeat(64);
            let (_sender_files, _store, intent) =
                bindings::signed("unit-1", &scope, "https://loom.example");
            let mut app = new_app(instance);
            app.install_workspace_checkpoint(Some(Profile::open(&directory.0).unwrap()), true)
                .unwrap();
            load(&mut app, bindings::source("unit-1", &scope).to_vec());
            settle(&mut app);
            let expected = publish_metadata(&mut app, &intent);
            settle(&mut app);
            assert!(app.projection_sender_inclusion(&expected));
            app.workspace_checkpoint.as_mut().unwrap().stopped = true;
            assert!(!app.projection_sender_inclusion(&expected));
            app.workspace_checkpoint.as_mut().unwrap().stopped = false;
            let canonical = std::fs::read(directory.path()).unwrap();
            load(&mut app, canonical);
            assert!(
                !app.projection_sender_inclusion(&expected),
                "same token/unit import is not commit"
            );
            settle(&mut app);
            assert!(app.projection_sender_inclusion(&expected));
            let source = app.restored_session.as_ref().unwrap().original.clone();
            app.restored_session
                .as_mut()
                .unwrap()
                .edit_original(source.to_vec().into());
            assert!(
                !app.projection_sender_inclusion(&expected),
                "equal bytes are not same Source allocation"
            );
            settle(&mut app);
            assert!(app.projection_sender_inclusion(&expected));
        });
    }
}
#[test]
fn metadata_edit_keeps_manual_disk_baseline_and_ctrl_s_preserves_sender_and_unknown_fields() {
    unsafe {
        run(|instance| {
            let directory = Directory::new();
            let scope = "a".repeat(64);
            let (sender_files, _store, intent) =
                bindings::signed("unit-1", &scope, "https://loom.example");
            let manual = directory.0.join("manual.json");
            let bytes = bindings::source("unit-1", &scope);
            std::fs::write(&manual, &bytes).unwrap();
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
            let deadline = Instant::now() + Duration::from_secs(10);
            while app.restore_job.is_some() {
                app.process_restore(false).unwrap();
                assert!(Instant::now() < deadline);
                std::thread::yield_now();
            }
            let expected = publish_metadata(&mut app, &intent);
            let record = app.restored_session.as_ref().unwrap();
            assert_eq!(
                record.disk_original.as_ref().unwrap().as_ref(),
                bytes.as_ref()
            );
            assert_eq!(std::fs::read(&manual).unwrap(), bytes.as_ref());
            let journal = std::fs::read(sender_files.path()).unwrap();
            app.request_save_existing().unwrap();
            let prepared = loop {
                if let Some(prepared) = app.take_save_candidate().unwrap() {
                    break prepared;
                }
                assert!(Instant::now() < deadline);
                std::thread::yield_now();
            };
            prepared
                .commit_ready(&mut app, &AtomicBool::new(false))
                .unwrap();
            let saved = std::fs::read(&manual).unwrap();
            assert!(Index::capture(&saved.clone().into())
                .unwrap()
                .includes(&expected));
            let value: serde_json::Value = serde_json::from_slice(&saved).unwrap();
            assert_eq!(value["futureRoot"]["keep"], true);
            assert_eq!(value["stickers"][0]["future"]["keep"], true);
            assert!(
                !app.projection_sender_inclusion(&expected),
                "manual save is not canonical checkpoint"
            );
            assert_eq!(std::fs::read(sender_files.path()).unwrap(), journal);
        });
    }
}
