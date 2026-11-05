//! 发送写前链路只使用隔离身份/PNG/目录；不触真实 profile，不联网，不占前台。
use super::*;
use serde_json::{json, Value};
#[path = "projection_sender_lifetime_tests.rs"]
mod lifetime;
#[path = "projection_sender_storage_tests.rs"]
mod storage;
const ROOT: &str = "C:/Users/Public/nas_home/AI/GameEditor/linshi/otter-sender-intent-tests";
const ORIGIN: &str = "https://loom.example";
pub(crate) struct Directory(pub(crate) PathBuf);
impl Directory {
    pub(crate) fn new() -> Self {
        std::fs::create_dir_all(ROOT).unwrap();
        let root = Path::new(ROOT).join(format!("{:?}", unsafe {
            windows::Win32::System::Com::CoCreateGuid().unwrap()
        }));
        std::fs::create_dir(&root).unwrap();
        Self(root)
    }
    pub(crate) fn path(&self) -> PathBuf {
        self.0.join("projection-sender.json")
    }
    pub(crate) fn identity(&self) -> PathBuf {
        self.0.join("identity")
    }
    pub(crate) fn install_identity(&self) {
        let store = crate::device_identity_store::Store::new(&self.identity()).unwrap();
        let mut loaded = store.load_or_create().unwrap();
        store.bind_and_save(&mut loaded, "device-1").unwrap();
    }
    fn store(&self) -> Store {
        Store::open(&self.0, &AtomicBool::new(false)).unwrap()
    }
}
pub(crate) fn selection(index: usize) -> Selection {
    Selection::new(
        &"a".repeat(64),
        ORIGIN,
        "unit-1",
        &format!("target-{index}"),
    )
    .unwrap()
}
pub(crate) fn frame() -> Frame {
    Frame {
        png: crate::projection_delivery::accept::tests::png().into(),
        width: 2,
        height: 1,
        kind: Kind::Sticker,
    }
}
pub(crate) fn prepare(
    store: &Store,
    directory: &Directory,
    selection: &Selection,
) -> Result<Intent, Error> {
    store.prepare(
        Preparation {
            selection,
            frame: frame(),
            identity_root: &directory.identity(),
            device: "device-1",
            now_ms: 1_000,
        },
        &AtomicBool::new(false),
    )
}
pub(crate) fn setup() -> (Directory, Store) {
    let directory = Directory::new();
    directory.install_identity();
    let store = directory.store();
    (directory, store)
}
#[test]
fn signed_intent_survives_restart_without_resigning_or_replacing_changed_frame() {
    use base64::{
        engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD},
        Engine,
    };
    let (directory, store) = setup();
    let intent = prepare(&store, &directory, &selection(1)).unwrap();
    let bytes = std::fs::read(directory.path()).unwrap();
    let value: Value = serde_json::from_slice(&bytes).unwrap();
    let envelope = &value["entries"][0]["envelope"];
    assert_eq!(envelope["expiresAtMs"], 271_000);
    assert_eq!(envelope["source"]["revision"], 1);
    assert_eq!(envelope["source"]["deviceId"], "device-1");
    assert_eq!(intent.dimensions(), (2, 1));
    assert_eq!(envelope["content"]["digest"], intent.digest());
    assert!(!bytes.starts_with(&[239, 187, 191]));
    let message = [
        envelope["protocol"].as_str().unwrap(),
        envelope["projectionId"].as_str().unwrap(),
        ORIGIN,
        "device-1",
        envelope["source"]["sessionId"].as_str().unwrap(),
        "unit-1",
        "1",
        "sticker",
        intent.digest(),
        "271000",
        envelope["nonce"].as_str().unwrap(),
    ]
    .join("\n");
    let loaded = crate::device_identity_store::Store::new(&directory.identity())
        .unwrap()
        .load_existing()
        .unwrap();
    let key = STANDARD
        .decode(loaded.identity().public_key().unwrap())
        .unwrap();
    let signature = URL_SAFE_NO_PAD
        .decode(envelope["signature"]["value"].as_str().unwrap())
        .unwrap();
    ring::signature::UnparsedPublicKey::new(&ring::signature::ED25519, key)
        .verify(message.as_bytes(), &signature)
        .unwrap();
    let saved_envelope = intent.envelope().clone();
    let debug = format!("{intent:?}");
    for forbidden in [
        envelope["nonce"].as_str().unwrap(),
        envelope["signature"]["value"].as_str().unwrap(),
        "imageBase64",
        "unit-1",
    ] {
        assert!(!debug.contains(forbidden));
    }
    drop((store, intent));
    let reopened = directory.store();
    let retry = reopened
        .prepare(
            Preparation {
                selection: &selection(1),
                frame: Frame {
                    png: vec![1].into(),
                    width: 0,
                    height: 0,
                    kind: Kind::Art,
                },
                identity_root: &directory.identity(),
                device: "device-1",
                now_ms: 999_999,
            },
            &AtomicBool::new(false),
        )
        .unwrap();
    assert_eq!(retry.envelope(), &saved_envelope);
    assert_eq!(std::fs::read(directory.path()).unwrap(), bytes);
    for forbidden in [
        "privateKey",
        "fixture-secret",
        "deviceToken",
        "projection-receive",
        "identity_root",
    ] {
        assert!(!std::str::from_utf8(&bytes).unwrap().contains(forbidden));
    }
    for name in [
        "projection-receive.json",
        "projection-receive.lock",
        "projection-receive-policy.json",
        "workspace.json",
    ] {
        assert!(!directory.0.join(name).exists());
    }
}
#[test]
fn missing_corrupt_unbound_or_wrong_identity_does_not_generate_or_repair_it() {
    for input in [None, Some(b"broken".as_slice())] {
        let directory = Directory::new();
        let identity = directory.identity();
        std::fs::create_dir(&identity).unwrap();
        if let Some(bytes) = input {
            std::fs::write(identity.join("device-identity.json"), bytes).unwrap();
        }
        let store = directory.store();
        assert_eq!(
            prepare(&store, &directory, &selection(1)).err(),
            Some(Error::Identity)
        );
        assert!(!directory.path().exists());
        assert_eq!(
            std::fs::read(identity.join("device-identity.json"))
                .ok()
                .as_deref(),
            input
        );
    }
    let directory = Directory::new();
    let identity_store = crate::device_identity_store::Store::new(&directory.identity()).unwrap();
    let mut loaded = identity_store.load_or_create().unwrap();
    let store = directory.store();
    assert_eq!(
        prepare(&store, &directory, &selection(1)).err(),
        Some(Error::Identity)
    );
    identity_store
        .bind_and_save(&mut loaded, "other-device")
        .unwrap();
    assert_eq!(
        prepare(&store, &directory, &selection(1)).err(),
        Some(Error::Identity)
    );
    assert!(!directory.path().exists());
}
#[test]
fn changed_key_or_same_device_with_different_private_key_cannot_reuse_old_signature() {
    let (directory, store) = setup();
    let first = prepare(&store, &directory, &selection(1)).unwrap();
    let before = std::fs::read(directory.path()).unwrap();
    let other = Directory::new();
    other.install_identity();
    assert_eq!(
        store
            .prepare(
                Preparation {
                    selection: &selection(1),
                    frame: frame(),
                    identity_root: &other.identity(),
                    device: "device-1",
                    now_ms: 1_000
                },
                &AtomicBool::new(false)
            )
            .err(),
        Some(Error::Identity)
    );
    assert_eq!(std::fs::read(directory.path()).unwrap(), before);
    let next = prepare(&store, &directory, &selection(2)).unwrap();
    assert_ne!(first.envelope(), next.envelope());
    let scoped = Selection::new(&"b".repeat(64), ORIGIN, "unit-1", "target-1").unwrap();
    let other_scope = prepare(&store, &directory, &scoped).unwrap();
    assert_ne!(first.envelope(), other_scope.envelope());
}
#[test]
fn malformed_selection_and_unimplemented_offline_route_are_rejected() {
    for value in [
        Selection {
            scope: "bad".into(),
            ..selection(1)
        },
        Selection {
            origin: "http://loom.example".into(),
            ..selection(1)
        },
        Selection {
            origin: "https://loom.example/path".into(),
            ..selection(1)
        },
        Selection {
            unit: "a\nb".into(),
            ..selection(1)
        },
        Selection {
            target: "peer-target:abc".into(),
            ..selection(1)
        },
        Selection {
            target: "x".repeat(161),
            ..selection(1)
        },
    ] {
        assert_eq!(value.validate(), Err(Error::Format));
    }
}
#[test]
fn invalid_png_dimensions_decode_limits_and_clock_never_publish_an_intent() {
    let (directory, store) = setup();
    for frame in [
        Frame {
            png: b"not png".to_vec().into(),
            ..frame()
        },
        Frame {
            width: 1,
            ..frame()
        },
        Frame {
            width: 0,
            ..frame()
        },
        Frame {
            width: 8193,
            ..frame()
        },
        Frame {
            width: 8192,
            height: 8192,
            ..frame()
        },
        Frame {
            png: vec![0; super::super::accept::MAX_PNG + 1].into(),
            ..frame()
        },
    ] {
        assert_eq!(
            store
                .prepare(
                    Preparation {
                        selection: &selection(1),
                        frame,
                        identity_root: &directory.identity(),
                        device: "device-1",
                        now_ms: 1_000
                    },
                    &AtomicBool::new(false)
                )
                .err(),
            Some(Error::Snapshot)
        );
        assert!(!directory.path().exists());
    }
    assert!(store
        .prepare(
            Preparation {
                selection: &selection(1),
                frame: frame(),
                identity_root: &directory.identity(),
                device: "device-1",
                now_ms: u64::MAX
            },
            &AtomicBool::new(false)
        )
        .is_err());
    assert!(!directory.path().exists());
}
#[test]
fn pre_cancelled_operation_does_not_create_lock_identity_or_document() {
    let directory = Directory::new();
    assert_eq!(
        Store::open(&directory.0, &AtomicBool::new(true)).err(),
        Some(Error::Stopped)
    );
    assert_eq!(std::fs::read_dir(&directory.0).unwrap().count(), 0);
    let store = directory.store();
    assert_eq!(
        store
            .prepare(
                Preparation {
                    selection: &selection(1),
                    frame: frame(),
                    identity_root: &directory.identity(),
                    device: "device-1",
                    now_ms: 1_000
                },
                &AtomicBool::new(true)
            )
            .err(),
        Some(Error::Stopped)
    );
    assert!(!directory.identity().exists());
    assert!(!directory.path().exists());
}
