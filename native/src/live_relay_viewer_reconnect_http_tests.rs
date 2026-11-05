//! 同Grant的真实TLS/WSS恢复：精确水位、新nonce、无再次加入，撤销后原许可与资源退休。
use super::*;
use crate::live_relay_control_runtime::interaction;
use crate::{
    byte_budget::Budget, live_relay_viewer_frames::Cursor, live_relay_viewer_identity::Snapshot,
    live_relay_viewer_runtime::Runtime as MediaRuntime, live_relay_viewer_session::ViewerSession,
};
use std::sync::mpsc;
use std::time::{Duration, Instant};

fn pixel(id: u64) -> Vec<u8> {
    use crate::live_relay_frame::*;
    let mut bytes = Metadata {
        epoch: 1,
        frame_id: id,
        capture_timestamp_ms: id,
        encode_timestamp_ms: id,
        width: 1,
        height: 1,
        dropped_frames: 0,
        keyframe: true,
        color_space: ColorSpace::Srgb,
        codec: Codec::RawBgra,
    }
    .encode_header(4)
    .unwrap()
    .to_vec();
    bytes.extend_from_slice(&[id as u8, 41, 91, 255]);
    bytes
}
fn receive(media: &ViewerSession, id: u64) {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(frame) = media
            .take(Cursor {
                epoch: 1,
                frame_id: id,
            })
            .unwrap()
        {
            assert_eq!(&**frame.pixels(), &[id as u8, 41, 91, 255]);
            return;
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(5));
    }
}
#[test]
fn authorized_reconnect_uses_same_owner_and_new_nonce_then_revoke_retires_everything() {
    let (listener, client, tls) = fixture();
    let mut grant = Grant::new(session(&client.origin, u64::MAX));
    let lease = Arc::new(grant.lease(&client.origin, 0).unwrap());
    let snapshot = Snapshot::parse(&snapshot(), "live:test/part").unwrap();
    let media =
        Arc::new(ViewerSession::new(snapshot.identity().clone(), "device-1".into()).unwrap());
    media.activate(&snapshot).unwrap();
    let permit = media.admit(1).unwrap();
    lease.bind_media(&media).unwrap();
    let (drop_tx, drop_rx) = mpsc::channel();
    let peer = std::thread::spawn(move || {
        let mut nonces = std::collections::HashSet::new();
        for after in [0, 1] {
            let mut stream = StreamOwned::new(
                ServerConnection::new(tls.clone()).unwrap(),
                accept(&listener),
            );
            let (head, _) = request(&mut stream);
            assert!(head.starts_with("GET /v1/live/media?"));
            let nonce = head
                .lines()
                .find_map(|line| {
                    let (key, value) = line.split_once(':')?;
                    key.eq_ignore_ascii_case("x-loom-device-nonce")
                        .then(|| value.trim().to_owned())
                })
                .unwrap();
            assert!(nonces.insert(nonce));
            let mut socket = super::playback::upgrade_media_at(stream, &head, after);
            socket
                .send(tungstenite::Message::Binary(pixel(after + 1)))
                .unwrap();
            if after == 0 {
                drop_rx.recv_timeout(Duration::from_secs(5)).unwrap();
                socket
                    .get_mut()
                    .sock
                    .interrupt_handle()
                    .unwrap()
                    .cancel()
                    .unwrap();
            } else {
                while socket.read().is_ok() {}
            }
        }
        assert_eq!(nonces.len(), 2);
    });
    let budget = Budget::new(1024);
    let mut runtime: MediaRuntime = lease
        .start_media_recovering(
            &client,
            media.clone(),
            permit.clone(),
            budget.clone(),
            Arc::new(interaction::Queue::default()),
            |_| {},
        )
        .unwrap();
    receive(&media, 1);
    drop_tx.send(()).unwrap();
    receive(&media, 2);
    assert!(media.validate(&permit).is_ok());
    drop(grant);
    assert!(media.validate(&permit).is_err());
    runtime.request_stop();
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if runtime.try_join().is_some() {
            break;
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(5));
    }
    drop(runtime);
    assert_eq!(budget.used(), 0);
    peer.join().unwrap();
}
