//! 真实回环 TLS 资源获取与消费身份门；不下载外部图片，不操作用户文件。
use super::*;
use crate::live_relay_http::{MountedSurface, ResourceError};
use sha2::{Digest, Sha256};
#[path = "surface_javascript_resource_tests.rs"]
mod javascript;
#[path = "surface_resource_runtime_tests.rs"]
mod runtime;
fn clock() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}
fn image(origin: &str, bytes: &[u8]) -> (MountedSurface, String) {
    let digest = format!("{:x}", Sha256::digest(bytes));
    let id = format!("sha256:{digest}");
    let snapshot = serde_json::json!({"protocolVersion":"loom.surface.v1",
        "instanceId":"surface:test/part","attachmentId":"attachment:1","revision":3,
        "resourceLeases":[{"leaseId":"lease:fixture","resource":{
            "resourceId":id,"kind":"image","mime":"image/png","size":bytes.len()},
            "transport":{"kind":"loom_resource","path":format!("/v1/surfaces/resources/{digest}")},
            "expiresAtMs":clock()+60_000}]});
    (
        MountedSurface {
            binding: binding(origin),
            snapshot: serde_json::value::to_raw_value(&snapshot).unwrap(),
        },
        id,
    )
}
fn mutate(mount: &mut MountedSurface, edit: impl FnOnce(&mut serde_json::Value)) {
    let mut value: serde_json::Value = serde_json::from_str(mount.snapshot.get()).unwrap();
    edit(&mut value);
    mount.snapshot = serde_json::value::to_raw_value(&value).unwrap();
}
#[test]
fn image_resource_https_auth_hash_and_current_mount_gate() {
    let (listener, client, config) = fixture();
    let (mut mount, id) = image(&client.origin, b"verified encoded bytes");
    let session = session(&client.origin, u64::MAX);
    let path = format!("GET /v1/surfaces/resources/{} ", &id[7..]);
    let peer = std::thread::spawn(move || {
        let mut stream =
            StreamOwned::new(ServerConnection::new(config).unwrap(), accept(&listener));
        let (head, _) = request(&mut stream);
        assert!(head.starts_with(&path));
        let head = head.to_ascii_lowercase();
        for field in [
            "authorization: device fixture-secret",
            "x-loom-device-nonce:",
            "x-loom-surface-lease: lease:fixture",
            "accept: application/octet-stream",
        ] {
            assert!(head.contains(field), "{field}");
        }
        reply(&mut stream, 200, b"verified encoded bytes");
    });
    let cancel = Cancel::new();
    let resource = Control::new(&client, &session, &cancel)
        .fetch_image_resource(mount.image_resource(&id, clock()).unwrap())
        .unwrap();
    peer.join().unwrap();
    assert_eq!(
        resource.bytes(&mount, clock(), &cancel).unwrap(),
        b"verified encoded bytes"
    );
    assert_eq!(resource.mime(), "image/png");
    assert!(matches!(
        resource.bytes(&mount, clock() + 3_600_000, &cancel),
        Err(ResourceError::Expired)
    ));
    mutate(&mut mount, |s| s["revision"] = 4.into());
    assert!(matches!(
        resource.bytes(&mount, clock(), &cancel),
        Err(ResourceError::Identity)
    ));
    mutate(&mut mount, |s| {
        s["revision"] = 3.into();
        s["resourceLeases"][0]["leaseId"] = "replacement".into();
    });
    assert!(matches!(
        resource.bytes(&mount, clock(), &cancel),
        Err(ResourceError::Identity)
    ));
}
#[test]
fn image_resource_rejects_invalid_contracts_before_network() {
    let (_listener, client, _config) = fixture();
    for (pointer, value, expected) in [
        (
            "/resourceLeases/0/leaseId",
            serde_json::json!("bad\r\nheader"),
            ResourceError::Input,
        ),
        (
            "/resourceLeases/0/resource/size",
            serde_json::json!(0),
            ResourceError::Limit,
        ),
        (
            "/resourceLeases/0/resource/size",
            serde_json::json!(16 * 1024 * 1024 + 1),
            ResourceError::Limit,
        ),
        (
            "/resourceLeases/0/expiresAtMs",
            serde_json::json!(0),
            ResourceError::Expired,
        ),
        (
            "/resourceLeases/0/expiresAtMs",
            serde_json::json!(clock() + 7_200_000),
            ResourceError::Expired,
        ),
        (
            "/resourceLeases/0/transport/path",
            serde_json::json!("https://evil.invalid/x"),
            ResourceError::Input,
        ),
        (
            "/resourceLeases/0/transport/path",
            serde_json::json!("/v1/surfaces/resources/../x"),
            ResourceError::Input,
        ),
        (
            "/resourceLeases/0/transport/kind",
            serde_json::json!("shared_memory"),
            ResourceError::Unsupported,
        ),
        (
            "/resourceLeases/0/resource/mime",
            serde_json::json!("application/x-neuro-rgba8"),
            ResourceError::Input,
        ),
        (
            "/attachmentId",
            serde_json::json!("foreign"),
            ResourceError::Identity,
        ),
    ] {
        let (mut mount, id) = image(&client.origin, b"data");
        mutate(&mut mount, |s| *s.pointer_mut(pointer).unwrap() = value);
        assert!(
            matches!(mount.image_resource(&id, clock()), Err(e) if e == expected),
            "{pointer}"
        );
    }
    let (mut mount, id) = image(&client.origin, b"data");
    mutate(&mut mount, |s| {
        let duplicate = s["resourceLeases"][0].clone();
        s["resourceLeases"].as_array_mut().unwrap().push(duplicate);
    });
    assert!(matches!(
        mount.image_resource(&id, clock()),
        Err(ResourceError::Identity)
    ));
}
#[test]
fn image_resource_rejects_corruption_size_redirect_and_unauthorized() {
    for (status, body, expected) in [
        (200, b"evil".as_slice(), ResourceError::Digest),
        (200, b"x".as_slice(), ResourceError::Limit),
        (
            200,
            b"extra".as_slice(),
            ResourceError::Http(LiveError::Http(crate::device_session_client::Error::Limit)),
        ),
        (
            302,
            b"".as_slice(),
            ResourceError::Http(LiveError::Status(302)),
        ),
        (
            401,
            b"".as_slice(),
            ResourceError::Http(LiveError::AuthorizationRequired),
        ),
    ] {
        let (listener, client, config) = fixture();
        let (mount, id) = image(&client.origin, b"data");
        let session = session(&client.origin, u64::MAX);
        let peer = std::thread::spawn(move || {
            let mut stream =
                StreamOwned::new(ServerConnection::new(config).unwrap(), accept(&listener));
            request(&mut stream);
            reply(&mut stream, status, body);
        });
        let cancel = Cancel::new();
        let result = Control::new(&client, &session, &cancel)
            .fetch_image_resource(mount.image_resource(&id, clock()).unwrap());
        assert!(matches!(result, Err(e) if e == expected), "status {status}");
        peer.join().unwrap();
    }
}
#[test]
fn image_resource_checks_origin_and_cancellation_before_connect() {
    let (_listener, client, _config) = fixture();
    let session = session(&client.origin, u64::MAX);
    let cancel = Cancel::new();
    let control = Control::new(&client, &session, &cancel);
    let (mount, id) = image("https://foreign.invalid", b"data");
    assert!(matches!(
        control.fetch_image_resource(mount.image_resource(&id, clock()).unwrap()),
        Err(ResourceError::Identity)
    ));
    let (mount, id) = image(&client.origin, b"data");
    cancel.cancel();
    assert!(matches!(
        control.fetch_image_resource(mount.image_resource(&id, clock()).unwrap()),
        Err(ResourceError::Http(LiveError::Http(
            crate::device_session_client::Error::Stopped
        )))
    ));
}
#[test]
fn image_resource_raw_rgba_digest_and_budget_above_json_stream_ceiling() {
    let (listener, client, config) = fixture();
    let bytes = vec![0x7f; 9 * 1024 * 1024];
    let (mut mount, id) = image(&client.origin, &bytes);
    mutate(&mut mount, |s| {
        let r = &mut s["resourceLeases"][0]["resource"];
        r["mime"] = "application/x-neuro-rgba8".into();
        r["width"] = 1536.into();
        r["height"] = 1536.into();
    });
    let session = session(&client.origin, u64::MAX);
    let peer = std::thread::spawn(move || {
        let mut stream =
            StreamOwned::new(ServerConnection::new(config).unwrap(), accept(&listener));
        request(&mut stream);
        reply(&mut stream, 200, &bytes);
    });
    let cancel = Cancel::new();
    let resource = Control::new(&client, &session, &cancel)
        .fetch_image_resource(mount.image_resource(&id, clock()).unwrap())
        .unwrap();
    assert_eq!(
        resource.bytes(&mount, clock(), &cancel).unwrap().len(),
        9 * 1024 * 1024
    );
    assert!(resource
        .bytes(&mount, clock(), &cancel)
        .unwrap()
        .iter()
        .all(|b| *b == 0x7f));
    peer.join().unwrap();
    cancel.cancel();
    assert!(matches!(
        resource.bytes(&mount, clock(), &cancel),
        Err(ResourceError::Http(LiveError::Http(
            crate::device_session_client::Error::Stopped
        )))
    ));
}
