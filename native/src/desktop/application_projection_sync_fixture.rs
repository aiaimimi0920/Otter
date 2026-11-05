//! 真实 750ms deadline/TLS/read 接缝；不点菜单或伪造完成结果，沿原 Application poll。
use super::*;
use base64::{engine::general_purpose::STANDARD, Engine};
use sha2::{Digest, Sha256};
pub(super) fn changed(origin: &str) -> serde_json::Value {
    let mut bytes = Vec::new();
    {
        let mut encoder = ::png::Encoder::new(&mut bytes, 3, 1);
        encoder.set_color(::png::ColorType::Rgba);
        encoder.set_depth(::png::BitDepth::Eight);
        encoder
            .write_header()
            .unwrap()
            .write_image_data(&[0, 0, 255, 255, 0, 0, 255, 255, 0, 0, 255, 255])
            .unwrap();
    }
    let mut value = super::super::fixture::response(origin);
    value["revision"] = 6.into();
    value["width"] = 3.into();
    value["digest"] = format!("{:x}", Sha256::digest(&bytes)).into();
    value["snapshot"] =
        serde_json::json!({"width":3,"height":1,"imageBase64":STANDARD.encode(bytes)});
    value
}
pub(super) fn peer(fixture: &mut Fixture, modes: Vec<u8>) -> mpsc::Receiver<()> {
    peer_with_renew(fixture, modes, false)
}
pub(super) fn peer_with_renew(
    fixture: &mut Fixture,
    modes: Vec<u8>,
    renew: bool,
) -> mpsc::Receiver<()> {
    let listener = fixture.listener.take().unwrap();
    let tls = fixture.tls.clone();
    let origin = fixture.origin.clone();
    let offline = fixture.offline;
    let (tx, rx) = mpsc::sync_channel(4);
    fixture.server = Some(std::thread::spawn(move || {
        if renew {
            for index in 0..2 {
                let mut stream = StreamOwned::new(
                    ServerConnection::new(tls.clone()).unwrap(),
                    accept(&listener),
                );
                let (head, _) = request(&mut stream);
                if index == 0 {
                    assert!(head.starts_with("POST /v1/device-sessions/challenges "));
                    reply(
                        &mut stream,
                        200,
                        br#"{"deviceId":"device-old","challengeId":"id","challenge":"value"}"#,
                    );
                } else {
                    assert!(head.starts_with("POST /v1/device-sessions "));
                    let now = crate::desktop::surface_windows::now_ms().unwrap();
                    reply(&mut stream, 200, &serde_json::to_vec(&serde_json::json!({"deviceId":"device-old","token":"receive-owner-token","expiresAtMs":now+60_000})).unwrap());
                }
            }
        }
        let mut revision = 5;
        for mode in modes {
            let mut stream = StreamOwned::new(
                ServerConnection::new(tls.clone()).unwrap(),
                accept(&listener),
            );
            let (head, body) = request(&mut stream);
            assert!(head.starts_with(if offline {
                "POST /v1/offline-projections/read "
            } else {
                "POST /v1/projections/read "
            }));
            assert!(head
                .to_ascii_lowercase()
                .contains("authorization: device receive-owner-token\r\n"));
            assert_eq!(body["knownRevision"], revision);
            assert_eq!(
                body["projectionId"],
                png::invitation(&origin).envelope().projection_id()
            );
            if mode == 2 || mode == 5 {
                reply(
                    &mut stream,
                    409,
                    if mode == 5 {
                        br#"{"error":{"code":"projection_peer_revoked","message":"projection_peer_revoked"}}"#
                    } else {
                        br#"{"error":{"code":"projection_unlinked","message":"projection_unlinked"}}"#
                    },
                );
            } else if mode == 3 {
                reply(
                    &mut stream,
                    503,
                    br#"{"error":{"code":"temporarily_unavailable"}}"#,
                );
            } else {
                let mut value = super::super::fixture::response(&origin);
                if matches!(mode, 1 | 4) {
                    value = changed(&origin);
                    if mode == 1 {
                        revision = 6;
                    } // mode 4 明确核拒绝提交后的旧 knownRevision。
                } else {
                    value["revision"] = revision.into();
                    value["snapshot"] = serde_json::Value::Null;
                    if revision == 6 {
                        value["digest"] = changed(&origin)["digest"].clone();
                    }
                }
                if offline {
                    value["envelope"] = crate::projection_delivery::tests::offline::entry(false)
                        ["envelope"]
                        .clone();
                    value["route"] = "offline_peer".into();
                    value["peerId"] = crate::projection_delivery::tests::offline::PEER.into();
                    value["delivery"]["targetDeviceId"] = "device-old".into();
                }
                if mode == 6 {
                    value["peerId"] = format!("loom-{}", "c".repeat(64)).into();
                }
                reply(&mut stream, 200, &serde_json::to_vec(&value).unwrap());
            }
            tx.send(()).unwrap();
        }
        listener
    }));
    rx
}
pub(super) fn cycle(fixture: &mut Fixture, received: &mpsc::Receiver<()>) {
    let end = Instant::now() + Duration::from_secs(10);
    let mut seen = false;
    loop {
        poll(fixture);
        seen |= received.try_recv().is_ok();
        if seen && !fixture.app.projection_receive.busy() {
            break;
        }
        assert!(Instant::now() < end, "continuous read deadline");
        std::thread::sleep(Duration::from_millis(1));
    }
}
pub(super) unsafe fn completed(fixture: &mut Fixture, received: &mpsc::Receiver<()>) {
    let end = Instant::now() + Duration::from_secs(10);
    while !fixture.app.projection_receive.busy() {
        poll(fixture);
        assert!(Instant::now() < end, "continuous read start deadline");
        std::thread::sleep(Duration::from_millis(1));
    }
    // 开始后不再 poll：精确停在真实 worker 已完成、UI 尚未领取的边界。
    received.recv_timeout(Duration::from_secs(5)).unwrap();
    let handle = fixture.app.projection_receive.wait_handle().unwrap();
    assert_eq!(
        WaitForSingleObject(HANDLE(handle.as_raw_handle()), 5000),
        WAIT_OBJECT_0
    );
}
