use super::*;
use crate::{
    live_relay_media_endpoint::{Credential, Endpoint},
    live_relay_viewer_frames::Cursor,
    live_relay_viewer_identity::Snapshot,
};
use rustls::RootCertStore;
use std::{
    net::TcpListener,
    time::{Duration, Instant},
};

pub(super) fn snapshot() -> Snapshot {
    Snapshot::parse(br#"{"session":{"protocolVersion":"loom.live.v1","sessionId":"live:test","sourceDeviceId":"source:test","sourceHookId":"hook:test","revision":1,"viewerDevices":["viewer:test"]},"epoch":1,"closed":false}"#, "live:test").unwrap()
}
pub(super) fn session() -> Arc<ViewerSession> {
    let snapshot = snapshot();
    let session =
        Arc::new(ViewerSession::new(snapshot.identity().clone(), "viewer:test".into()).unwrap());
    session.activate(&snapshot).unwrap();
    session
}
pub(super) fn listener() -> TcpListener {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    listener
}
pub(super) fn accept(listener: TcpListener) -> std::net::TcpStream {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        match listener.accept() {
            Ok((tcp, _)) => {
                tcp.set_nonblocking(false).unwrap();
                tcp.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
                tcp.set_write_timeout(Some(Duration::from_secs(3))).unwrap();
                return tcp;
            }
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                assert!(Instant::now() < deadline);
                std::thread::sleep(Duration::from_millis(5));
            }
            Err(e) => panic!("fixture accept: {e}"),
        }
    }
}
pub(super) fn join(runtime: &mut Runtime) -> Exit {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(exit) = runtime.try_join() {
            return exit;
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
#[allow(clippy::result_large_err)]
fn network_runtime_publishes_pixels_then_stops_both_threads_without_retiring_new_generation() {
    let listener = listener();
    let address = listener.local_addr().unwrap();
    let peer = std::thread::spawn(move || {
        let mut socket = tungstenite::accept_hdr(
            accept(listener),
            |_: &tungstenite::handshake::server::Request,
             mut response: tungstenite::handshake::server::Response| {
                response
                    .headers_mut()
                    .insert("sec-websocket-protocol", "loom.live.v1".parse().unwrap());
                Ok(response)
            },
        )
        .unwrap();
        use crate::live_relay_frame::{Codec, ColorSpace, Metadata};
        let mut bytes = Metadata {
            epoch: 1,
            frame_id: 1,
            capture_timestamp_ms: 0,
            encode_timestamp_ms: 0,
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
        bytes.extend_from_slice(&[1, 2, 3, 255]);
        socket.send(tungstenite::Message::Binary(bytes)).unwrap();
        assert!(socket.read().is_err());
    });
    let endpoint = Endpoint::new(
        &format!("http://127.0.0.1:{}", address.port()),
        RootCertStore::empty(),
    )
    .unwrap();
    let prepared = endpoint
        .viewer("live:test", "viewer:test", (0, 0), Credential::None)
        .unwrap();
    let session = session();
    let budget = Budget::new(1024);
    let mut runtime = Runtime::start(
        prepared,
        vec![address],
        session.clone(),
        session.admit(1).unwrap(),
        budget.clone(),
    )
    .unwrap();
    assert!(runtime.wait_handle().is_some());
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(frame) = session
            .take(Cursor {
                epoch: 1,
                frame_id: 1,
            })
            .unwrap()
        {
            assert_eq!(&**frame.pixels(), &[1, 2, 3, 255]);
            break;
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(5));
    }
    runtime.request_stop();
    session.activate(&snapshot()).unwrap();
    let fresh = session.admit(1).unwrap();
    assert_eq!(join(&mut runtime), Exit::Stopped);
    assert!(runtime.wait_handle().is_none());
    drop(runtime);
    assert!(session.validate(&fresh).is_ok());
    assert_eq!(budget.used(), 0);
    peer.join().unwrap();
}

#[test]
fn stop_interrupts_stalled_tls_handshake_and_retires_original_permit() {
    for after_hello in [false, true] {
        let listener = listener();
        let address = listener.local_addr().unwrap();
        let (accepted, ready) = std::sync::mpsc::channel();
        let peer = std::thread::spawn(move || {
            use std::io::Read;
            let mut tcp = accept(listener);
            tcp.set_read_timeout(Some(Duration::from_secs(8))).unwrap();
            let mut bytes = [0u8; 4096];
            if after_hello {
                assert!(matches!(tcp.read(&mut bytes), Ok(n) if n > 0));
            }
            accepted.send(()).unwrap();
            loop {
                let result = tcp.read(&mut bytes);
                if !matches!(result, Ok(n) if n > 0) {
                    eprintln!("stalled TLS peer finished: {result:?}");
                    break;
                }
            }
        });
        let endpoint = Endpoint::new(
            &format!("https://localhost:{}", address.port()),
            RootCertStore::empty(),
        )
        .unwrap();
        let prepared = endpoint
            .viewer(
                "live:test",
                "viewer:test",
                (0, 0),
                Credential::Device("fixture-token"),
            )
            .unwrap();
        let session = session();
        let mut runtime = Runtime::start(
            prepared,
            vec![address],
            session.clone(),
            session.admit(1).unwrap(),
            Budget::new(1024),
        )
        .unwrap();
        ready.recv_timeout(Duration::from_secs(3)).unwrap();
        if after_hello {
            std::thread::sleep(Duration::from_millis(30));
        }
        let start = Instant::now();
        runtime.request_stop();
        eprintln!(
            "runtime stop after_hello={after_hello} elapsed: {:?}",
            start.elapsed()
        );
        let retired = session.admit(1).is_err();
        let exit = join(&mut runtime);
        let elapsed = start.elapsed();
        eprintln!("runtime handshake join elapsed: {elapsed:?}");
        // 保留原停止时延断言，但先回收 fixture，失败不留下服务端线程干扰后续用例。
        peer.join().unwrap();
        assert!(retired);
        assert_eq!(exit, Exit::Stopped);
        assert!(elapsed < Duration::from_secs(2));
    }
}

#[test]
fn invalid_permit_is_rejected_before_network_thread_creation() {
    let endpoint = Endpoint::new("http://127.0.0.1:1", RootCertStore::empty()).unwrap();
    let session = session();
    for (id, device) in [("live:other", "viewer:test"), ("live:test", "viewer:other")] {
        let prepared = endpoint
            .viewer(id, device, (0, 0), Credential::None)
            .unwrap();
        assert!(Runtime::start(
            prepared,
            vec!["127.0.0.1:1".parse().unwrap()],
            session.clone(),
            session.admit(1).unwrap(),
            Budget::new(1024)
        )
        .is_err());
    }
    let old = session.admit(1).unwrap();
    session.suspend().unwrap();
    let prepared = endpoint
        .viewer("live:test", "viewer:test", (0, 0), Credential::None)
        .unwrap();
    assert!(Runtime::start(
        prepared,
        vec!["127.0.0.1:1".parse().unwrap()],
        session,
        old,
        Budget::new(1024)
    )
    .is_err());
}

#[test]
fn network_slots_are_bounded_and_returned_on_drop() {
    let mut slots = (0..8).map(|_| Slot::acquire().unwrap()).collect::<Vec<_>>();
    assert!(matches!(Slot::acquire(), Err(e) if e.kind() == io::ErrorKind::WouldBlock));
    slots.pop();
    let replacement = Slot::acquire().unwrap();
    drop(slots);
    drop(replacement);
    assert_eq!(ACTIVE.load(Ordering::Acquire), 0);
}
