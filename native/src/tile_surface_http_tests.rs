//! 原TLS夹具驱动真实Views owner；未知open不重放、迟到open关闭、资源逐步完整后才交帧。
use super::*;
use crate::{device_authorization_lease::Grant, tile_assets::Job, tile_surface_owner::Views};
use serde_json::{json, Value};
fn job() -> (Arc<crate::tile_shared::Shared>, Job) {
    let (shared, frame) = crate::tile_tests::shared_frame();
    (
        shared,
        Job {
            generation: 1,
            key: frame.key,
            layout: frame.layout,
            size: frame.size,
        },
    )
}
fn state(job: &Job) -> Value {
    json!({"protocolVersion":"loom.wall.v1","view":{"binding":crate::tile_surface_wire::Binding::from(&job.key),"instanceId":"art","attachmentId":"ephemeral"},"width":64,"height":64,"generation":1,"sequence":0,
        "snapshot":{"protocolVersion":"loom.surface.v1","instanceId":"art","attachmentId":"ephemeral","artId":"fixture","artVersion":"1","revision":1,"runtime":"declarative","scene":{"id":"root","type":"view"},"resources":[],"resourceLeases":[]},"preview":null,"result":null,"confirmations":[],"pending":[],"failure":null})
}
fn connection(
    listener: &std::net::TcpListener,
    config: &Arc<ServerConfig>,
) -> StreamOwned<ServerConnection, BridgeStream> {
    StreamOwned::new(
        ServerConnection::new(config.clone()).unwrap(),
        accept(listener),
    )
}
#[test]
fn surface_response_budgets_require_exact_device_post_and_do_not_expand_ordinary_routes() {
    let (listener, client, _config) = fixture();
    let stop = AtomicBool::new(false);
    for (method, path, device, limit) in [
        ("POST", "/v1/walls/surfaces/open", false, 10 * 1024 * 1024),
        ("GET", "/v1/walls/surfaces/open", true, 10 * 1024 * 1024),
        ("POST", "/v1/walls/surfaces/open/", true, 10 * 1024 * 1024),
        (
            "POST",
            "/v1/walls/surfaces/state?alias=1",
            true,
            10 * 1024 * 1024,
        ),
        (
            "POST",
            "/v1/walls/surfaces/state",
            true,
            10 * 1024 * 1024 + 1,
        ),
        ("POST", "/v1/walls/surfaces/image", false, 24 * 1024 * 1024),
        (
            "POST",
            "/v1/walls/surfaces/image",
            true,
            24 * 1024 * 1024 + 1,
        ),
        (
            "POST",
            "/v1/walls/surfaces/close",
            true,
            8 * 1024 * 1024 + 1,
        ),
        ("POST", "/v1/ordinary", true, 8 * 1024 * 1024 + 1),
    ] {
        let result = client.exchange(
            Request {
                method,
                path,
                body: &[],
                limit,
                device: device.then_some(("fixture-secret", "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")),
            },
            &stop,
            &mut |_| panic!("rejected before any TCP connection"),
            Instant::now() + Duration::from_secs(1),
        );
        assert_eq!(result.err(), Some(Error::Request), "{method} {path}");
    }
    assert!(listener.accept().is_err());
}
#[test]
fn malformed_successful_open_is_unknown_creation_and_cannot_be_implicitly_retried() {
    let (listener, client, config) = fixture();
    let mut grant = Grant::new(session(client.origin(), u64::MAX));
    let lease = grant.lease(client.origin(), 0).unwrap();
    let (shared, job) = job();
    let peer = std::thread::spawn(move || {
        let mut stream = connection(&listener, &config);
        let (head, body) = request(&mut stream);
        assert!(head.starts_with("POST /v1/walls/surfaces/open "));
        assert!(head
            .to_ascii_lowercase()
            .contains("authorization: device fixture-secret\r\n"));
        assert_eq!(body["instanceId"], "art");
        reply(&mut stream, 200, b"{}");
        drop(stream);
        std::thread::sleep(Duration::from_millis(100));
        assert!(
            listener.accept().is_err(),
            "no speculative reopen or guessed close"
        );
    });
    let mut views = Views::new(&client, &lease);
    assert_eq!(
        views.poll(&["art".into()], &shared, &job, 0),
        Err("Art视图校验失败")
    );
    assert!(shared.lock().stopped);
    assert!(views.frames().is_empty());
    views.poll(&["art".into()], &shared, &job, 0).unwrap();
    drop(views);
    peer.join().unwrap();
}
#[test]
fn open_finishing_after_freeze_closes_only_the_verified_view_and_never_publishes_it() {
    let (listener, client, config) = fixture();
    let mut grant = Grant::new(session(client.origin(), u64::MAX));
    let lease = grant.lease(client.origin(), 0).unwrap();
    let (shared, job) = job();
    let response = state(&job);
    let held = shared.clone();
    let peer = std::thread::spawn(move || {
        let mut stream = connection(&listener, &config);
        assert!(request(&mut stream)
            .0
            .starts_with("POST /v1/walls/surfaces/open "));
        held.lock().frozen = true;
        reply(&mut stream, 200, &serde_json::to_vec(&response).unwrap());
        drop(stream);
        let mut stream = connection(&listener, &config);
        let (head, body) = request(&mut stream);
        assert!(head.starts_with("POST /v1/walls/surfaces/close "));
        assert_eq!(body["view"], response["view"]);
        reply(
            &mut stream,
            200,
            br#"{"protocolVersion":"loom.wall.v1","accepted":true}"#,
        );
    });
    let mut views = Views::new(&client, &lease);
    views.poll(&["art".into()], &shared, &job, 0).unwrap();
    assert!(views.frames().is_empty() && !shared.lock().stopped);
    drop(views);
    peer.join().unwrap();
}
#[test]
fn each_poll_downloads_at_most_one_resource_and_width_only_mismatch_is_not_ignored() {
    use base64::{engine::general_purpose::STANDARD, Engine};
    use sha2::{Digest, Sha256};
    for mismatch in [false, true] {
        let (listener, client, config) = fixture();
        let mut grant = Grant::new(session(client.origin(), u64::MAX));
        let lease = grant.lease(client.origin(), 0).unwrap();
        let (shared, job) = job();
        let mut value = state(&job);
        let resources: Vec<_> = [[255u8,0,0,255].repeat(2), [0u8,255,0,255].repeat(2)].iter().map(|raw| {
            let id = format!("sha256:{:x}", Sha256::digest(raw));
            json!({"protocolVersion":"loom.wall.v1","resource":{"resourceId":id,"kind":"image","mime":"application/x-neuro-rgba8","size":8,"width":2,"height":1},"dataBase64":STANDARD.encode(raw)})
        }).collect();
        let mut descriptors: Vec<_> = resources.iter().map(|r| r["resource"].clone()).collect();
        let first = (0..descriptors.len())
            .min_by_key(|i| descriptors[*i]["resourceId"].as_str().unwrap())
            .unwrap();
        descriptors[first].as_object_mut().unwrap().remove("height");
        if mismatch {
            descriptors[first]["width"] = 1.into();
        }
        value["snapshot"]["resources"] = descriptors.into();
        value["snapshot"]["scene"]["children"] = json!([
            {"id":"a","type":"image","props":{"resourceId":resources[0]["resource"]["resourceId"]}},
            {"id":"b","type":"image","props":{"resourceId":resources[1]["resource"]["resourceId"]}}]);
        let peer = std::thread::spawn(move || {
            let mut stream = connection(&listener, &config);
            assert!(request(&mut stream)
                .0
                .starts_with("POST /v1/walls/surfaces/open "));
            reply(&mut stream, 200, &serde_json::to_vec(&value).unwrap());
            drop(stream);
            let images = if mismatch { 1 } else { 2 };
            let mut reads = std::collections::BTreeSet::new();
            for _ in 0..images {
                let mut stream = connection(&listener, &config);
                let (head, body) = request(&mut stream);
                assert!(head.starts_with("POST /v1/walls/surfaces/image "));
                assert_eq!(body["view"], value["view"]);
                let id = body["resourceId"].as_str().unwrap();
                assert!(reads.insert(id.to_owned()));
                let response = resources
                    .iter()
                    .find(|r| r["resource"]["resourceId"] == id)
                    .unwrap();
                reply(&mut stream, 200, &serde_json::to_vec(response).unwrap());
            }
            let mut stream = connection(&listener, &config);
            assert!(request(&mut stream)
                .0
                .starts_with("POST /v1/walls/surfaces/close "));
            reply(
                &mut stream,
                200,
                br#"{"protocolVersion":"loom.wall.v1","accepted":true}"#,
            );
        });
        let mut views = Views::new(&client, &lease);
        views.poll(&["art".into()], &shared, &job, 0).unwrap();
        assert!(views.frames().is_empty());
        // BTreeMap按digest排序；单维失败场景确保首先取到含width约束的那张。
        if mismatch {
            let first = views.poll(&["art".into()], &shared, &job, 0);
            assert_eq!(first, Err("Art图片尺寸不匹配"));
        } else {
            views.poll(&["art".into()], &shared, &job, 0).unwrap();
            assert!(views.frames().is_empty());
            views.poll(&["art".into()], &shared, &job, 0).unwrap();
            let frames = views.frames();
            assert_eq!(frames.len(), 1);
            assert_eq!(
                &*frames["art"].images["a"].pixels,
                &[0, 0, 255, 255].repeat(2)
            );
            assert_eq!(
                &*frames["art"].images["b"].pixels,
                &[0, 255, 0, 255].repeat(2)
            );
        }
        drop(views);
        peer.join().unwrap();
    }
}
