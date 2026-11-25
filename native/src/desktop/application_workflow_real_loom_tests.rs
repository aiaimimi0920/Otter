//! Native save/archive and explicit resend through an isolated Loom bridge.
use super::*;
use crate::desktop::{document_window::DocumentWindow, Apartment};
use crate::session_document::workflow_snapshot::batch::dispatch::Failure;
use std::{
    fs,
    path::Path,
    sync::atomic::AtomicBool,
    time::{Duration, Instant},
};

#[test]
#[ignore = "requires an isolated Loom bridge controller under linshi"]
fn saved_workflow_survives_real_loom_bridge_restart() {
    let root = fs::canonicalize(std::env::var_os("OTTER_WORKFLOW_LOOM_FIXTURE").unwrap()).unwrap();
    let base = fs::canonicalize("C:/Users/Public/nas_home/AI/GameEditor/linshi").unwrap();
    assert!(root != base && root.starts_with(base));
    let session = root.join("session.json");
    assert!(!session.exists());
    let assets = root.join("images");
    fs::create_dir(&assets).unwrap();

    let _apartment = Apartment::new().unwrap();
    unsafe {
        let instance = crate::desktop::workspace_window_fixture::register();
        let image = crate::desktop::image::Image {
            width: 2,
            height: 1,
            pixels: vec![10, 20, 30, 255, 40, 50, 60, 255].into(),
        };
        let document =
            DocumentWindow::create(instance, State::new(Some(image), false).unwrap(), false)
                .unwrap();
        let mut app = Application::new(instance, document);
        let cancel = AtomicBool::new(false);
        app.request_save_new(session.clone()).unwrap();
        wait(|| app.take_save_candidate().unwrap())
            .commit_ready(&mut app, &cancel)
            .unwrap();
        let initial = fs::read(&session).unwrap();
        let revision = crate::session_document::decode(&initial).unwrap().revision;

        let first = catalog(&root);
        let first_endpoint = first.identity.address();
        app.catalog.borrow_mut().loaded = Some(Rc::new(first));
        fs::create_dir(root.join("initial-capability-read")).unwrap();
        wait(|| root.join("bridge-stopped").is_dir().then_some(()));

        let archive = Arc::new(Root::open(&assets).unwrap());
        app.request_workflow_cycle(archive.clone(), 123).unwrap();
        let outcome = wait(|| {
            app.process_workflow_sync();
            app.take_workflow_cycle()
        });
        assert!(outcome.current);
        assert!(outcome.archive.unwrap().retained_recovery.is_none());
        let report = outcome.sync.unwrap();
        assert_eq!(report.acknowledged, 0);
        assert_eq!(
            report.failure,
            Some(Failure::Connect(crate::loom_bridge_client::Error::Connect))
        );
        assert!(!report.may_have_reached_remote);
        assert!(app.take_workflow_delivery().is_none());
        let committed = fs::read(&session).unwrap();
        assert_eq!(
            crate::session_document::decode(&committed)
                .unwrap()
                .revision,
            revision + 1
        );
        let stored: serde_json::Value = serde_json::from_slice(&committed).unwrap();
        let id = stored["stickers"][0]["id"].as_str().unwrap();
        let source = stored["stickers"][0]["src"].as_str().unwrap();
        assert!(Path::new(source)
            .canonicalize()
            .unwrap()
            .starts_with(&assets));
        assert_eq!(
            stored["workflowAssetArchiveIndex"]["workflows"]["hook-live"]["nodes"][id]["src"],
            source
        );
        assert_eq!(app.save_status(), save_state::Status::Unchanged);
        fs::create_dir(root.join("disconnected-outcome-verified")).unwrap();

        wait(|| root.join("bridge-started").is_dir().then_some(()));
        let next = catalog(&root);
        assert_ne!(next.identity.address(), first_endpoint);
        app.catalog.borrow_mut().loaded = Some(Rc::new(next));
        for _ in 0..3 {
            app.process_workflow_sync();
            assert!(app.take_workflow_delivery().is_none());
            assert!(app.take_workflow_cycle().is_none());
        }
        fs::create_dir(root.join("fresh-capability-read")).unwrap();
        wait(|| root.join("explicit-resend").is_dir().then_some(()));

        // This is a new user-intent equivalent, not a replay of the failed worker.
        app.request_workflow_preparation(Some(archive)).unwrap();
        let candidate = wait(|| app.take_workflow_preparation().unwrap());
        app.request_workflow_sync(candidate).unwrap();
        let delivery = wait(|| {
            app.process_workflow_sync();
            app.take_workflow_delivery()
        })
        .unwrap();
        assert!(delivery.is_current(&app));
        assert_eq!(delivery.report().acknowledged, 1);
        assert_eq!(delivery.report().failure, None);
        assert_eq!(fs::read(&session).unwrap(), committed);
        assert_eq!(app.save_status(), save_state::Status::Unchanged);
        fs::write(
            root.join("otter-result.json"),
            serde_json::to_vec(&serde_json::json!({
                "nodeId": id, "committedRevision": revision + 1, "archiveRetained": true,
                "offlineAcknowledged": 0, "offlineMayHaveReachedRemote": false,
                "manualResendAcknowledged": 1, "sameCommittedBytes": true, "productExe": false
            }))
            .unwrap(),
        )
        .unwrap();
    }
}

fn catalog(root: &Path) -> crate::desktop::capability_job::Loaded {
    let manifest = root.join("manifest").join("loom.json");
    assert!(manifest.is_file());
    let config = crate::loom_discovery::Config::from_lookup(|key| {
        (key == "LOOM_MANIFEST_PATH").then(|| manifest.clone().into_os_string())
    });
    let identity = config.load().unwrap();
    let handshake =
        crate::loom_handshake_exchange::fetch(&identity, &AtomicBool::new(false)).unwrap();
    crate::desktop::capability_job::Loaded {
        identity,
        handshake,
    }
}

fn wait<T>(mut read: impl FnMut() -> Option<T>) -> T {
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        if let Some(value) = read() {
            return value;
        }
        assert!(
            Instant::now() < deadline,
            "isolated workflow fixture timed out"
        );
        std::thread::sleep(Duration::from_millis(2));
    }
}
