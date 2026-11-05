//! 真实隐藏 HWND + 实际归档替换 + 固定证书 TLS/WS 的完整成功链路，隔离于用户服务。
use super::*;
use crate::desktop::{document_window::DocumentWindow, Apartment};
use std::{
    sync::atomic::AtomicBool,
    time::{Duration, Instant},
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
struct Class(HINSTANCE);
impl Drop for Class {
    fn drop(&mut self) {
        unsafe {
            let _ = UnregisterClassW(w!("Otter.Native.Workspace"), Some(self.0));
        }
    }
}
#[test]
fn real_application_cycle_archives_before_sending_committed_revision() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
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
        let _class = Class(instance);
        let image = crate::desktop::image::Image {
            width: 2,
            height: 1,
            pixels: vec![10, 20, 30, 255, 40, 50, 60, 255].into(),
        };
        let document =
            DocumentWindow::create(instance, State::new(Some(image), false).unwrap(), false)
                .unwrap();
        let mut app = Application::new(instance, document);
        let directory = std::path::PathBuf::from("C:/Users/Public/nas_home/AI/GameEditor/linshi")
            .join(format!(
                "otter-cycle-success-{}",
                crate::desktop::document_state::new_unit_id().unwrap()
            ));
        std::fs::create_dir(&directory).unwrap();
        let images = directory.join("images");
        std::fs::create_dir(&images).unwrap();
        let root = Arc::new(Root::open(&images).unwrap());
        let path = directory.join("session.json");
        app.request_save_new(path.clone()).unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        let prepared = loop {
            if let Some(prepared) = app.take_save_candidate().unwrap() {
                break prepared;
            }
            assert!(Instant::now() < deadline, "Save worker did not finish");
            std::thread::sleep(Duration::from_millis(1));
        };
        prepared
            .commit_ready(&mut app, &AtomicBool::new(false))
            .unwrap();
        assert_eq!(app.save_status(), save_state::Status::Unchanged);
        let baseline = app.restored_session.as_ref().unwrap().original.clone();
        let revision = crate::session_document::decode(&baseline).unwrap().revision;
        let (listener, identity, config) = crate::loom_bridge_client::tests::fixture();
        listener.set_nonblocking(true).unwrap();
        let handshake = crate::loom_handshake::response(
            &serde_json::to_vec(&crate::loom_handshake::tests::fixture()).unwrap(),
        )
        .unwrap()
        .unwrap();
        app.catalog.borrow_mut().loaded = Some(Rc::new(crate::desktop::capability_job::Loaded {
            identity,
            handshake,
        }));
        let saved_path = path.clone();
        let server = std::thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(10);
            let tcp = loop {
                match listener.accept() {
                    Ok((stream, _)) => break stream,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        assert!(Instant::now() < deadline, "No workflow connection arrived");
                        std::thread::sleep(Duration::from_millis(1));
                    }
                    Err(error) => panic!("Workflow listener failed: {error}"),
                }
            };
            let stream = crate::loom_bridge_stream::BridgeStream::new(
                tcp,
                Instant::now() + Duration::from_secs(5),
            )
            .unwrap();
            let mut socket = tungstenite::accept(rustls::StreamOwned::new(
                rustls::ServerConnection::new(config).unwrap(),
                stream,
            ))
            .unwrap();
            let message = socket.read().unwrap();
            let request: serde_json::Value =
                serde_json::from_str(message.to_text().unwrap()).unwrap();
            assert_eq!(request["method"], "loom.hook.workflow.sync");
            assert_eq!(request["params"]["workflowId"], "hook-live");
            let snapshot = &request["params"]["snapshot"];
            assert_eq!(snapshot["documentRevision"], revision + 1);
            assert_eq!(snapshot["nodes"].as_array().unwrap().len(), 1);
            // 回执前查磁盘，证明网络消息来自已耐久提交的新基线。
            let disk: serde_json::Value =
                serde_json::from_slice(&std::fs::read(saved_path).unwrap()).unwrap();
            assert_eq!(disk["documentRevision"], revision + 1);
            let id = disk["stickers"][0]["id"].as_str().unwrap();
            let source = disk["stickers"][0]["src"].as_str().unwrap();
            assert!(std::path::Path::new(source).is_file());
            assert_eq!(
                disk["workflowAssetArchiveIndex"]["workflows"]["hook-live"]["nodes"][id]["src"],
                source
            );
            socket.send(tungstenite::Message::Text(serde_json::json!({"protocolVersion":"loom.hook.v1","requestId":request["params"]["requestId"],"status":"succeeded","data":{"workflowId":"hook-live"}}).to_string())).unwrap();
        });
        app.request_workflow_cycle(root.clone(), 123).unwrap();
        let deadline = Instant::now() + Duration::from_secs(12);
        let outcome = loop {
            app.process_workflow_sync();
            if let Some(outcome) = app.take_workflow_cycle() {
                break outcome;
            }
            assert!(Instant::now() < deadline, "Workflow cycle did not finish");
            std::thread::sleep(Duration::from_millis(1));
        };
        server.join().unwrap();
        assert!(outcome.current);
        let report = outcome.sync.unwrap();
        assert_eq!(report.acknowledged, 1);
        assert_eq!(report.failure, None);
        assert!(outcome.archive.unwrap().retained_recovery.is_none());
        assert!(app.take_workflow_cycle().is_none());
        assert!(app
            .restored_session
            .as_ref()
            .unwrap()
            .workflow_delivery
            .is_none());
        assert_eq!(app.save_status(), save_state::Status::Unchanged);
        assert!(!app
            .restored_session
            .as_ref()
            .unwrap()
            .original
            .same_storage(&baseline));
        assert_eq!(
            std::fs::read(&path).unwrap(),
            app.restored_session.as_ref().unwrap().original.as_ref()
        );
        drop((app, root));
        std::fs::remove_dir_all(directory).unwrap();
    }
}
