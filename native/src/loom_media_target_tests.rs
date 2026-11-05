use super::*;
use crate::{loom_discovery::Config, loom_discovery_manifest};
use std::{ffi::OsString, io};

fn manifest(origin: &str, auth: &str, token: &str) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({
        "schemaVersion":1,"appId":"loom","displayName":"Loom","version":"1",
        "pid":1,"startedAt":1,"capabilities":["brain.plan"],
        "transport":{"type":"http","baseUrl":origin,"auth":auth,"authToken":token}
    }))
    .unwrap()
}

#[test]
fn local_media_reuses_manifest_auth_without_requiring_hook_bridge() {
    for host in ["127.0.0.1", "localhost", "[::1]"] {
        for scheme in ["http", "https"] {
            for auth in ["none", "bearer"] {
                let bytes = manifest(&format!("{scheme}://{host}:1234"), auth, " fixture-bearer ");
                let target = loom_discovery_manifest::parse_media(&bytes).unwrap();
                let prepared = target
                    .local_viewer("live:test", (3, 4), RootCertStore::empty())
                    .unwrap();
                assert!(!prepared.device);
                assert!(prepared
                    .url
                    .query_pairs()
                    .any(|(k, v)| k == "deviceId" && v == "device-000-local"));
                assert_eq!(
                    prepared
                        .request
                        .headers()
                        .get("authorization")
                        .map(|v| v.to_str().unwrap()),
                    (auth == "bearer").then_some("Bearer fixture-bearer")
                );
                assert!(!format!("{:?}", prepared.request).contains("fixture-bearer"));
                assert!(prepared
                    .request
                    .headers()
                    .get("x-loom-device-nonce")
                    .is_none());
            }
        }
    }
}

#[test]
fn remote_manifest_bearer_cannot_be_used_as_local_or_device_authority() {
    let bytes = manifest("https://example.com", "bearer", "remote-secret");
    let target = loom_discovery_manifest::parse_media(&bytes).unwrap();
    assert_eq!(target.origin(), "https://example.com");
    assert!(matches!(
        target.local_viewer("live:test", (0, 0), RootCertStore::empty()),
        Err(Error::RemoteAuthorizationRequired)
    ));
    assert!(
        loom_discovery_manifest::parse_media(&manifest("http://example.com", "none", "")).is_err()
    );
    assert!(
        loom_discovery_manifest::parse_media(&manifest("http://localhost", "bearer", " ")).is_err()
    );
    assert!(loom_discovery_manifest::parse_media(&vec![
        b' ';
        loom_discovery_manifest::MAX_BYTES + 1
    ])
    .is_err());
}

#[test]
fn media_discovery_uses_same_paths_but_not_bridge_switch_or_override() {
    let config = Config::from_lookup(|key| match key.to_str().unwrap() {
        "OTTER_ENABLE_LOOM_OTTER" => Some(OsString::from("false")),
        "LOOM_MANIFEST_PATH" => Some(OsString::from("first.json")),
        "LOOM_MANIFEST_DIR" => Some(OsString::from("second")),
        "LOOM_OTTER_WS_URL" => Some(OsString::from("wss://wrong.example")),
        _ => None,
    });
    let mut seen = Vec::new();
    let bytes = manifest("http://localhost:1234", "none", "");
    let target = config
        .load_media_with(|path| {
            seen.push(path.to_path_buf());
            if seen.len() == 1 {
                Err(io::ErrorKind::NotFound.into())
            } else {
                Ok(io::Cursor::new(&bytes))
            }
        })
        .unwrap();
    assert_eq!(
        seen,
        vec![
            std::path::PathBuf::from("first.json"),
            std::path::PathBuf::from("second").join("loom.json")
        ]
    );
    assert_eq!(target.origin(), "http://localhost:1234");
    assert!(matches!(
        config.load_with(|_| Ok(io::Cursor::new(&bytes))),
        Err(crate::loom_discovery::Error::Disabled)
    ));
}

#[test]
fn malformed_or_unreadable_first_manifest_never_falls_back() {
    let config = Config::from_lookup(|key| {
        (key == "LOOM_MANIFEST_PATH").then(|| OsString::from("first.json"))
    });
    for malformed in [true, false] {
        let mut calls = 0;
        let result = config.load_media_with(|_| {
            calls += 1;
            if malformed {
                Ok(io::Cursor::new(b"{}"))
            } else {
                Err(io::ErrorKind::PermissionDenied.into())
            }
        });
        assert!(result.is_err());
        assert_eq!(calls, 1);
    }
}
