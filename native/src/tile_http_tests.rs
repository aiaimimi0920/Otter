//! 实际 TLS→串行 presenter/独立图片线程→原生 HWND→heartbeat；自有夹具，不接真实设备。
use super::*;
use crate::{
    device_authorization_lease::Grant,
    tile_runtime::Runtime,
    tile_wire::{self as wire, Endpoint},
};
use std::sync::atomic::{AtomicUsize, Ordering};
#[path = "tile_presenter_runtime_admission_tests.rs"]
mod admission;
fn image() -> (String, serde_json::Value) {
    use base64::Engine;
    use sha2::{Digest, Sha256};
    let bytes = [
        0, 0, 255, 255, 0, 255, 0, 255, 255, 0, 0, 255, 255, 255, 255, 255,
    ];
    let id = format!("sha256:{:x}", Sha256::digest(bytes));
    let value = serde_json::json!({"protocolVersion":wire::PROTOCOL,"resource":{"resourceId":id,"kind":"image","mime":"application/x-neuro-rgba8","size":16,"width":2,"height":2},"dataBase64":base64::engine::general_purpose::STANDARD.encode(bytes)});
    (id, value)
}
#[test]
fn exact_https_routes_device_nonce_and_image_digest_are_preserved() {
    let (listener, client, config) = fixture();
    let session = session(client.origin(), u64::MAX);
    let cancel = crate::device_request_cancel::Cancel::new();
    let endpoint = Endpoint::image("device-1", &crate::tile_tests::output());
    let advertised = endpoint.clone();
    let (id, resource) = image();
    let expected = id.clone();
    let peer = std::thread::spawn(move || {
        let mut nonces = std::collections::HashSet::new();
        for index in 0..6 {
            let mut stream = StreamOwned::new(
                ServerConnection::new(config.clone()).unwrap(),
                accept(&listener),
            );
            let (head, body) = request(&mut stream);
            let header = head.to_ascii_lowercase();
            assert!(header.contains("authorization: device fixture-secret\r\n"));
            assert!(nonces.insert(
                header
                    .lines()
                    .find_map(|s| s.strip_prefix("x-loom-device-nonce: "))
                    .unwrap()
                    .to_owned()
            ));
            let (path, response) = match index {
                0 => (
                    "GET /v1/walls/state",
                    serde_json::json!({"protocolVersion":wire::PROTOCOL,"revision":0,"endpoints":[],"layouts":[]}),
                ),
                1 => {
                    assert_eq!(body["endpoint"]["deviceId"], "device-1");
                    assert_eq!(body["baseRevision"], 0);
                    (
                        "POST /v1/walls/endpoints/register",
                        crate::tile_tests::catalog(&advertised, 2),
                    )
                }
                2 => (
                    "POST /v1/walls/connect",
                    serde_json::json!({"protocolVersion":wire::PROTOCOL,"leaseId":"lease-1","leaseTtlMs":15000}),
                ),
                3 => {
                    assert_eq!(body["resourceId"], expected);
                    assert_eq!(body["revision"], 2);
                    ("POST /v1/walls/images/read", resource.clone())
                }
                4 => {
                    assert_eq!(body["sequence"], 1);
                    assert_eq!(body["appliedRevision"], 2);
                    (
                        "POST /v1/walls/heartbeat",
                        serde_json::json!({"protocolVersion":wire::PROTOCOL,"accepted":true}),
                    )
                }
                _ => (
                    "POST /v1/walls/disconnect",
                    serde_json::json!({"protocolVersion":wire::PROTOCOL,"accepted":true}),
                ),
            };
            assert!(head.starts_with(&format!("{path} HTTP/1.1\r\n")));
            reply(&mut stream, 200, &serde_json::to_vec(&response).unwrap());
        }
    });
    unsafe {
        windows::Win32::System::Com::CoInitializeEx(
            None,
            windows::Win32::System::Com::COINIT_APARTMENTTHREADED,
        )
        .ok()
        .unwrap();
    }
    let control = crate::live_relay_http::Control::new(&client, &session, &cancel);
    assert_eq!(control.wall_state().unwrap().revision, 0);
    assert_eq!(control.wall_register(0, &endpoint).unwrap().revision, 2);
    let lease = control.wall_connect(&endpoint.endpoint_id).unwrap();
    let decoded = control
        .wall_image(&endpoint.endpoint_id, &lease.lease_id, 2, &id)
        .unwrap();
    assert_eq!(&decoded.pixels[..4], &[255, 0, 0, 255]);
    assert_eq!(decoded.width, 2);
    control
        .wall_heartbeat(
            &endpoint.endpoint_id,
            &lease.lease_id,
            1,
            Some(2),
            None,
            None,
        )
        .unwrap();
    control
        .wall_disconnect(&endpoint.endpoint_id, &lease.lease_id)
        .unwrap();
    unsafe {
        windows::Win32::System::Com::CoUninitialize();
    }
    peer.join().unwrap();
}
#[test]
fn image_integrity_and_unimplemented_formats_fail_without_allocating_or_publishing() {
    let (id, mut resource) = image();
    resource["dataBase64"] = "AAAA".into();
    assert!(
        crate::tile_image::decode(&serde_json::to_vec(&resource).unwrap(), &id, || false).is_err()
    );
    let (_, mut resource) = image();
    resource["resource"]["width"] = 16385.into();
    assert!(
        crate::tile_image::decode(&serde_json::to_vec(&resource).unwrap(), &id, || false).is_err()
    );
    let (_, mut resource) = image();
    resource["resource"]["mime"] = "image/svg+xml".into();
    assert!(
        crate::tile_image::decode(&serde_json::to_vec(&resource).unwrap(), &id, || false).is_err()
    );
}
#[test]
fn tile_png_decode_checks_real_mime_natural_dimensions_cancel_and_premultiplied_pixels() {
    use base64::Engine;
    use sha2::{Digest, Sha256};
    unsafe {
        windows::Win32::System::Com::CoInitializeEx(
            None,
            windows::Win32::System::Com::COINIT_APARTMENTTHREADED,
        )
        .ok()
        .unwrap();
    }
    struct Com;
    impl Drop for Com {
        fn drop(&mut self) {
            unsafe {
                windows::Win32::System::Com::CoUninitialize();
            }
        }
    }
    let _com = Com;
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut bytes, 2, 1);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();
        writer
            .write_image_data(&[255, 0, 0, 255, 0, 255, 0, 128])
            .unwrap();
        writer.finish().unwrap();
    }
    let id = format!("sha256:{:x}", Sha256::digest(&bytes));
    let value = serde_json::json!({"protocolVersion":wire::PROTOCOL,"resource":{"resourceId":id,"kind":"image","mime":"image/png","size":bytes.len(),"width":2,"height":1},"dataBase64":base64::engine::general_purpose::STANDARD.encode(bytes)});
    let decoded =
        crate::tile_image::decode(&serde_json::to_vec(&value).unwrap(), &id, || false).unwrap();
    assert_eq!(&*decoded.pixels, &[0, 0, 255, 255, 0, 128, 0, 128]);
    assert_eq!((decoded.width, decoded.height), (2, 1));
    assert!(crate::tile_image::decode(&serde_json::to_vec(&value).unwrap(), &id, || true).is_err());
    let mut mismatched = value.clone();
    mismatched["resource"]["width"] = 99.into();
    assert!(
        crate::tile_image::decode(&serde_json::to_vec(&mismatched).unwrap(), &id, || false)
            .is_err()
    );
    let mut wrong_mime = value;
    wrong_mime["resource"]["mime"] = "image/jpeg".into();
    assert!(
        crate::tile_image::decode(&serde_json::to_vec(&wrong_mime).unwrap(), &id, || false)
            .is_err()
    );
}
#[test]
fn window_presenter_images_freeze_black_resume_and_stop_drain_one_serial_lease() {
    let (listener, client, config) = fixture();
    let origin = client.origin().to_owned();
    let mut grant = Grant::new(session(&origin, u64::MAX));
    let lease = grant.lease(&origin, 0).unwrap();
    let images = grant.lease(&origin, 0).unwrap();
    let art = grant.lease(&origin, 0).unwrap();
    let mut output = crate::tile_tests::output();
    output.x = -30000;
    output.y = -30000;
    let endpoint = Endpoint::image("device-1", &output);
    let (id, resource) = image();
    let phase = Arc::new(AtomicUsize::new(0));
    let state = phase.clone();
    let reads = Arc::new(AtomicUsize::new(0));
    let image_reads = reads.clone();
    let peer = std::thread::spawn(move || {
        let end = Instant::now() + Duration::from_secs(25);
        let mut sequence = 0;
        let mut connects = 0;
        loop {
            assert!(Instant::now() < end, "fixture deadline");
            let mut stream = StreamOwned::new(
                ServerConnection::new(config.clone()).unwrap(),
                accept(&listener),
            );
            let (head, body) = request(&mut stream);
            let phase = state.load(Ordering::Acquire);
            let response = if head.starts_with("GET /v1/walls/state ") {
                let mut value =
                    crate::tile_tests::catalog(&endpoint, if phase >= 3 { 5 } else { 2 });
                value["layouts"][0]["placements"] = serde_json::json!([{"placementId":"p","source":{"kind":"image","id":id},"rect":{"x":0,"y":0,"width":2,"height":2},"sourceCrop":{"x":0,"y":0,"width":1,"height":1},"zIndex":0,"interactive":false}]);
                if phase == 1 || phase == 2 {
                    value["revision"] = (phase + 2).into();
                    value["presentations"] = serde_json::json!([{"wallId":"wall-1","revision":phase+2,"mode":if phase==1{"frozen"}else{"black"}}]);
                }
                value
            } else if head.starts_with("POST /v1/walls/connect ") {
                connects += 1;
                assert_eq!(connects, 1);
                serde_json::json!({"protocolVersion":wire::PROTOCOL,"leaseId":"presenter","leaseTtlMs":15000})
            } else if head.starts_with("POST /v1/walls/images/read ") {
                image_reads.fetch_add(1, Ordering::AcqRel);
                assert_eq!(body["resourceId"], id);
                resource.clone()
            } else if head.starts_with("POST /v1/walls/heartbeat ") {
                let next = body["sequence"].as_u64().unwrap();
                assert!(next > sequence);
                sequence = next;
                if phase == 0 && body["appliedRevision"] == 2 {
                    state.store(1, Ordering::Release);
                }
                if phase == 1 && body["presentation"]["outcome"] == "applied" {
                    assert_eq!(body["appliedRevision"], 2);
                    state.store(2, Ordering::Release);
                }
                if phase == 2 && body["presentation"]["outcome"] == "applied" {
                    assert!(body["appliedRevision"].is_null());
                    state.store(3, Ordering::Release);
                }
                if phase == 3 && body["appliedRevision"] == 5 {
                    state.store(4, Ordering::Release);
                }
                serde_json::json!({"protocolVersion":wire::PROTOCOL,"accepted":true})
            } else if head.starts_with("POST /v1/walls/disconnect ") {
                assert_eq!(phase, 4);
                reply(
                    &mut stream,
                    200,
                    br#"{"protocolVersion":"loom.wall.v1","accepted":true}"#,
                );
                break;
            } else {
                panic!("unexpected {}", head.lines().next().unwrap());
            };
            reply(&mut stream, 200, &serde_json::to_vec(&response).unwrap());
        }
    });
    unsafe {
        windows::Win32::System::Com::CoInitializeEx(
            None,
            windows::Win32::System::Com::COINIT_APARTMENTTHREADED,
        )
        .ok()
        .unwrap();
    }
    let runtime = Runtime::test_start(client, lease, images, art, output.clone()).unwrap();
    let window = crate::desktop::tile_test_window(runtime.shared.clone(), &output);
    let end = Instant::now() + Duration::from_secs(22);
    while phase.load(Ordering::Acquire) < 4 {
        assert!(Instant::now() < end, "{}", runtime.status());
        assert!(!runtime.finished());
        window.tick().unwrap();
        crate::desktop::tile_test_pump();
        std::thread::sleep(Duration::from_millis(20));
    }
    let pixels = runtime
        .shared
        .lock()
        .frame
        .as_ref()
        .unwrap()
        .images
        .values()
        .next()
        .unwrap()
        .pixels
        .downgrade();
    runtime.request_stop();
    while !runtime.finished() {
        crate::desktop::tile_test_pump();
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(
        reads.load(Ordering::Acquire),
        1,
        "immutable image reused across freeze/black/layout revisions"
    );
    assert!(runtime.shared.lock().frame.is_none());
    drop(window);
    drop(runtime);
    assert!(
        pixels.upgrade().is_none(),
        "window, cache, worker and candidate released their shared pixel lease"
    );
    drop(grant);
    peer.join().unwrap();
    unsafe {
        windows::Win32::System::Com::CoUninitialize();
    }
}
