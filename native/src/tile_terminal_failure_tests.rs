//! 未知连接写与撤权只终止旧 owner，不自动重放 connect，也不恢复旧候选。
use super::*;
use crate::{device_authorization_lease::Grant, tile_runtime::Runtime, tile_wire::Endpoint};
#[test]
fn lost_connect_response_is_not_replayed_and_never_publishes_a_frame() {
    terminal(false);
}
#[test]
fn unauthorized_state_revokes_both_request_leases_and_stops_without_repair_pairing() {
    terminal(true);
}
fn terminal(unauthorized: bool) {
    let (listener, client, config) = fixture();
    let origin = client.origin().to_owned();
    let mut grant = Grant::new(session(&origin, u64::MAX));
    let lease = grant.lease(&origin, 0).unwrap();
    let images = grant.lease(&origin, 0).unwrap();
    let art = grant.lease(&origin, 0).unwrap();
    let output = crate::tile_tests::output();
    let endpoint = Endpoint::image("device-1", &output);
    let peer = std::thread::spawn(move || {
        let mut stream = StreamOwned::new(
            ServerConnection::new(config.clone()).unwrap(),
            accept(&listener),
        );
        let (head, _) = request(&mut stream);
        assert!(head.starts_with("GET /v1/walls/state "));
        reply(
            &mut stream,
            if unauthorized { 401 } else { 200 },
            &if unauthorized {
                b"{}".to_vec()
            } else {
                serde_json::to_vec(&crate::tile_tests::catalog(&endpoint, 2)).unwrap()
            },
        );
        drop(stream);
        if !unauthorized {
            let mut stream =
                StreamOwned::new(ServerConnection::new(config).unwrap(), accept(&listener));
            let (head, _) = request(&mut stream);
            assert!(head.starts_with("POST /v1/walls/connect "));
            drop(stream);
        }
        std::thread::sleep(Duration::from_millis(200));
        assert!(
            listener.accept().is_err(),
            "no implicit reconnect/re-pair or mutation replay"
        );
    });
    let runtime = Runtime::test_start(client, lease, images, art, output).unwrap();
    let end = Instant::now() + Duration::from_secs(5);
    while !runtime.finished() {
        assert!(Instant::now() < end);
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(runtime.shared.lock().frame.is_none());
    assert!(runtime.shared.lock().presented.is_none());
    drop(runtime);
    drop(grant);
    peer.join().unwrap();
}
