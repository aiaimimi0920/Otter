//! 注入环境和文件读取器，不访问真实用户发现文件。
use super::*;
use std::{cell::Cell, io::Cursor};
fn config(values: &[(&str, &str)]) -> Config {
    Config::from_lookup(|key| {
        values
            .iter()
            .find(|(name, _)| key == OsStr::new(name))
            .map(|(_, value)| OsString::from(value))
    })
}
fn raw() -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({"schemaVersion":1,"appId":"loom","displayName":"Loom","version":"1",
        "pid":1,"startedAt":1,"transport":{"type":"http","baseUrl":"http://127.0.0.1:80"},"capabilities":["brain.plan"],
        "hookBridge":{"protocol":"loom.local-bridge.v1","instanceId":"d2f4f738-09d9-4f46-829e-262381e63387",
        "endpoint":"wss://127.0.0.1:43123/","certificateDerBase64":"YWJj",
        "certificateSha256":"ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad","authToken":"a1".repeat(32)}})).unwrap()
}
#[test]
fn canonical_disable_and_empty_override_never_revive_legacy() {
    for value in ["0", "false", ""] {
        let cfg = config(&[
            ("OTTER_ENABLE_LOOM_OTTER", value),
            ("HOOK_ENABLE_LOOM_HOOK", "1"),
        ]);
        let result = cfg.load_with::<Cursor<Vec<u8>>>(|_| panic!("disabled must not open files"));
        assert!(matches!(result, Err(Error::Disabled)));
    }
    let cfg = config(&[
        ("LOOM_OTTER_WS_URL", ""),
        ("LOOM_HOOK_WS_URL", "wss://127.0.0.1:1/"),
    ]);
    assert!(cfg.load_with(|_| Ok(Cursor::new(raw()))).is_ok());
    assert!(config(&[("HOOK_ENABLE_LOOM_HOOK", "yes")]).enabled);
    assert!(config(&[]).enabled);
}
#[test]
fn ordered_paths_and_first_found_manifest_match_main() {
    let cfg = config(&[
        ("LOOM_MANIFEST_PATH", "first.json"),
        ("LOOM_MANIFEST_DIR", "second"),
        ("LOOM_CAPABILITY_MANIFEST_DIR", "third"),
        ("NEURO_CAPABILITIES_DIR", "fourth"),
        ("APPDATA", "profile"),
    ]);
    assert_eq!(
        cfg.paths,
        vec![
            PathBuf::from("first.json"),
            PathBuf::from("second/loom.json"),
            PathBuf::from("third/loom.json"),
            PathBuf::from("fourth/loom.json"),
            PathBuf::from("profile/Neuro/capabilities/loom.json"),
            PathBuf::from(".runtime/neuro/capabilities/loom.json")
        ]
    );
    let mut seen = Vec::new();
    let identity = cfg
        .load_with(|path| {
            seen.push(path.to_owned());
            if seen.len() == 2 {
                Ok(Cursor::new(raw()))
            } else {
                Err(io::ErrorKind::NotFound.into())
            }
        })
        .unwrap();
    assert_eq!(seen, cfg.paths[..2]);
    assert_eq!(identity.endpoint(), "wss://127.0.0.1:43123/");
}
#[test]
fn failure_does_not_fall_back_and_errors_do_not_echo_input() {
    let cfg = config(&[("LOOM_MANIFEST_PATH", "first.json")]);
    let calls = Cell::new(0);
    let result = cfg.load_with::<Cursor<Vec<u8>>>(|_| {
        calls.set(calls.get() + 1);
        Err(io::ErrorKind::PermissionDenied.into())
    });
    assert!(matches!(result, Err(Error::Read)));
    assert_eq!(calls.get(), 1);
    let result = cfg.load_with(|_| Ok(Cursor::new(b"secret malformed input")));
    assert!(matches!(
        result,
        Err(Error::Manifest(loom_discovery_manifest::Error::Parse))
    ));
    assert!(!format!("{result:?}").contains("secret"));
    assert!(matches!(
        cfg.load_with::<Cursor<Vec<u8>>>(|_| Err(io::ErrorKind::NotFound.into())),
        Err(Error::NotFound)
    ));
    let cfg = config(&[("LOOM_OTTER_WS_URL", "wss://127.0.0.1:1/")]);
    assert!(matches!(
        cfg.load_with(|_| Ok(Cursor::new(raw()))),
        Err(Error::Endpoint)
    ));
}
#[test]
fn reader_is_capped_before_parsing() {
    struct Count<'a>(&'a Cell<usize>);
    impl Read for Count<'_> {
        fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            buffer.fill(b' ');
            self.0.set(self.0.get() + buffer.len());
            Ok(buffer.len())
        }
    }
    let bytes = Cell::new(0);
    let result = config(&[]).load_with(|_| Ok(Count(&bytes)));
    assert!(matches!(
        result,
        Err(Error::Manifest(loom_discovery_manifest::Error::TooLarge))
    ));
    assert_eq!(bytes.get(), loom_discovery_manifest::MAX_BYTES + 1);
}
