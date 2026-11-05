//! 复用真实 TLS 夹具，配对落盘只在固定 linshi 根下的唯一目录进行。
use super::*;
use crate::{
    device_identity_store::{Loaded, Store},
    device_pairing,
};
use std::path::PathBuf;

pub(crate) struct Files(PathBuf);
impl Files {
    pub(crate) fn root(&self) -> &std::path::Path {
        &self.0
    }
    pub(crate) fn store(&self) -> Store {
        Store::new(&self.0).unwrap()
    }
    pub(crate) fn new() -> (Self, Store, Loaded) {
        let root = PathBuf::from(
            "C:/Users/Public/nas_home/AI/GameEditor/linshi/otter-native-pairing-tests",
        );
        std::fs::create_dir_all(&root).unwrap();
        let root = root.join(format!(
            "{:?}",
            unsafe { windows::Win32::System::Com::CoCreateGuid() }.unwrap()
        ));
        std::fs::create_dir(&root).unwrap();
        let store = Store::new(&root).unwrap();
        let mut loaded = store.load_or_create().unwrap();
        store.bind_and_save(&mut loaded, "device-old").unwrap();
        (Self(root), store, loaded)
    }
    fn bytes(&self) -> Vec<u8> {
        std::fs::read(self.0.join("device-identity.json")).unwrap()
    }
}
impl Drop for Files {
    fn drop(&mut self) {
        if std::thread::panicking() {
            eprintln!("device-pairing-failure-evidence: {}", self.0.display());
            return;
        }
        // 不递归清理；仅删除此测试独占单层目录中的文件。
        if let Ok(entries) = std::fs::read_dir(&self.0) {
            for entry in entries.flatten() {
                let _ = std::fs::remove_file(entry.path());
            }
        }
        let _ = std::fs::remove_dir(&self.0);
    }
}
fn registry_peer(
    listener: TcpListener,
    config: Arc<ServerConfig>,
    status: u16,
    body: Vec<u8>,
    public: String,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let mut stream =
            StreamOwned::new(ServerConnection::new(config).unwrap(), accept(&listener));
        let (head, request) = request(&mut stream);
        assert!(head.starts_with("POST /v1/devices/requests HTTP/1.1"));
        assert_eq!(
            request,
            serde_json::json!({"name":"配对测试", "kind":"computer",
            "address":"hook://配对测试", "publicKey":public})
        );
        reply(&mut stream, status, &body);
        // 此操作不能继续申请 challenge/session；请求已声明 Connection: close。
        assert!(stream.read(&mut [0]).map_or(true, |n| n == 0));
    })
}

#[test]
fn actual_registration_saves_matching_pending_or_existing_id_without_changing_keys() {
    let (_files, store, mut loaded) = Files::new();
    let public = loaded.identity().public_key().unwrap();
    for (field, expected_changed) in [("pending", true), ("devices", false)] {
        let (listener, client, config) = fixture();
        let body = serde_json::to_vec(
            &serde_json::json!({field:[{"id":"device-new","publicKey":public}]}),
        )
        .unwrap();
        let peer = registry_peer(listener, config, 200, body, public.clone());
        assert_eq!(
            device_pairing::register_and_save(
                &client,
                &store,
                &mut loaded,
                "配对测试",
                &AtomicBool::new(false),
                |_| {}
            )
            .unwrap(),
            expected_changed
        );
        let saved = store.load_existing().unwrap();
        assert_eq!(saved.identity().device_id(), Some("device-new"));
        assert_eq!(saved.identity().public_key().unwrap(), public);
        assert_eq!(loaded.identity().device_id(), Some("device-new"));
        peer.join().unwrap();
    }
}

#[test]
fn actual_failed_or_unmatched_registration_preserves_file_and_loaded_identity() {
    let (files, store, mut loaded) = Files::new();
    let before = files.bytes();
    for (status, body, expected) in [
        (403, b"denied".as_slice(), device_pairing::Error::Http(403)),
        (
            200,
            br#"{"pending":[{"id":"other","publicKey":"not-ours"}]}"#,
            device_pairing::Error::Missing,
        ),
    ] {
        let (listener, client, config) = fixture();
        let peer = registry_peer(
            listener,
            config,
            status,
            body.to_vec(),
            loaded.identity().public_key().unwrap(),
        );
        assert_eq!(
            device_pairing::register_and_save(
                &client,
                &store,
                &mut loaded,
                "配对测试",
                &AtomicBool::new(false),
                |_| {}
            ),
            Err(expected)
        );
        assert_eq!(files.bytes(), before);
        assert_eq!(loaded.identity().device_id(), Some("device-old"));
        peer.join().unwrap();
    }
}

#[test]
fn accepted_registration_with_stale_local_snapshot_reports_partial_failure() {
    let (_files, store, mut loaded) = Files::new();
    let mut concurrent = store.load_existing().unwrap();
    let public = loaded.identity().public_key().unwrap();
    let (listener, client, config) = fixture();
    let body = serde_json::to_vec(
        &serde_json::json!({"pending":[{"id":"device-new","publicKey":public}]}),
    )
    .unwrap();
    let peer = registry_peer(listener, config, 200, body, public);
    let result = device_pairing::register_and_save(
        &client,
        &store,
        &mut loaded,
        "配对测试",
        &AtomicBool::new(false),
        |_| {
            store
                .bind_and_save(&mut concurrent, "concurrent-id")
                .unwrap();
        },
    );
    assert_eq!(
        result,
        Err(device_pairing::Error::Store(
            crate::device_identity_store::Error::Changed
        ))
    );
    assert_eq!(loaded.identity().device_id(), Some("device-old"));
    assert_eq!(
        store.load_existing().unwrap().identity().device_id(),
        Some("concurrent-id")
    );
    peer.join().unwrap();
}

#[test]
fn cancelled_registration_does_not_connect_or_change_identity() {
    let (files, store, mut loaded) = Files::new();
    let (listener, client, _) = fixture();
    let before = files.bytes();
    assert_eq!(
        device_pairing::register_and_save(
            &client,
            &store,
            &mut loaded,
            "配对测试",
            &AtomicBool::new(true),
            |_| panic!("cancelled request connected")
        ),
        Err(device_pairing::Error::Stopped)
    );
    assert!(matches!(listener.accept(), Err(e) if e.kind() == io::ErrorKind::WouldBlock));
    assert_eq!(files.bytes(), before);
}
