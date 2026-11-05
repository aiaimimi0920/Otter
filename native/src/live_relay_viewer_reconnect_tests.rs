//! 独立raw像素/恢复次数/停止oracle；实际TLS、H264和Application另由联合门验证。
use super::*;
use crate::live_relay_media_endpoint::{Credential, Endpoint};
use crate::live_relay_viewer_runtime::tests::{accept, join, listener, session};
use rustls::RootCertStore;
use std::sync::mpsc;

fn endpoint(address: SocketAddr) -> Endpoint {
    Endpoint::new(&format!("http://{address}"), RootCertStore::empty()).unwrap()
}
fn prepared(endpoint: &Endpoint, after: u64) -> Prepared {
    endpoint
        .viewer("live:test", "viewer:test", (1, after), Credential::None)
        .unwrap()
}
fn raw(id: u64) -> Vec<u8> {
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
    bytes.extend_from_slice(&[id as u8, 31, 73, 255]);
    bytes
}
#[allow(clippy::result_large_err)]
fn socket(
    listener: &std::net::TcpListener,
    after: u64,
) -> tungstenite::WebSocket<std::net::TcpStream> {
    tungstenite::accept_hdr(
        accept(listener.try_clone().unwrap()),
        move |request: &tungstenite::handshake::server::Request,
              mut response: tungstenite::handshake::server::Response| {
            assert!(request
                .uri()
                .query()
                .unwrap()
                .contains(&format!("afterEpoch=1&afterFrameId={after}")));
            response
                .headers_mut()
                .insert("sec-websocket-protocol", "loom.live.v1".parse().unwrap());
            Ok(response)
        },
    )
    .unwrap()
}
fn await_pixel(media: &ViewerSession, id: u64) {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(frame) = media
            .take(Cursor {
                epoch: 1,
                frame_id: id,
            })
            .unwrap()
        {
            assert_eq!(&**frame.pixels(), &[id as u8, 31, 73, 255]);
            break;
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(5));
    }
}
#[test]
fn abnormal_disconnect_reuses_permit_cursor_worker_and_skips_duplicate_raw() {
    let listener = listener();
    let address = listener.local_addr().unwrap();
    let (drop_tx, drop_rx) = mpsc::channel();
    let (close_tx, close_rx) = mpsc::channel();
    let peer = std::thread::spawn(move || {
        let mut first = socket(&listener, 0);
        first.send(tungstenite::Message::Binary(raw(1))).unwrap();
        drop_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        first.get_mut().shutdown(std::net::Shutdown::Both).unwrap();
        drop(first);
        let mut second = socket(&listener, 1);
        second.send(tungstenite::Message::Binary(raw(1))).unwrap();
        second.send(tungstenite::Message::Binary(raw(2))).unwrap();
        close_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        second.close(None).unwrap();
    });
    let endpoint = endpoint(address);
    let initial = prepared(&endpoint, 0);
    let (notified, notices) = mpsc::channel();
    let recovery = Recovery::new(
        1,
        move |cursor| Ok(prepared(&endpoint, cursor.frame_id)),
        move |active| {
            notified.send(active).unwrap();
        },
    );
    let media = session();
    let permit = media.admit(1).unwrap();
    let budget = Budget::new(1024);
    let mut runtime = Runtime::start_recovering(
        initial,
        vec![address],
        media.clone(),
        permit.clone(),
        budget.clone(),
        Some(recovery),
        |_| {},
    )
    .unwrap();
    await_pixel(&media, 1);
    assert_eq!(media.committed(&permit).unwrap().frame_id, 1);
    drop_tx.send(()).unwrap();
    assert!(notices.recv_timeout(Duration::from_secs(3)).unwrap());
    assert!(media.validate(&permit).is_ok());
    await_pixel(&media, 2);
    assert!(!notices.recv_timeout(Duration::from_secs(3)).unwrap());
    close_tx.send(()).unwrap();
    assert_eq!(join(&mut runtime), Exit::Closed);
    assert_eq!(budget.used(), 0);
    peer.join().unwrap();
}
#[test]
fn stop_wakes_backoff_and_authority_failure_does_not_open_second_socket() {
    for stop in [true, false] {
        let listener = listener();
        let address = listener.local_addr().unwrap();
        let endpoint = endpoint(address);
        let initial = prepared(&endpoint, 0);
        let peer = std::thread::spawn(move || {
            let socket = socket(&listener, 0);
            drop(socket);
        });
        let (notified, notice) = mpsc::channel();
        let recovery = Recovery::new(
            1,
            |_| Err(()),
            move |active| {
                notified.send(active).unwrap();
            },
        );
        let media = session();
        let mut runtime = Runtime::start_recovering(
            initial,
            vec![address],
            media.clone(),
            media.admit(1).unwrap(),
            Budget::new(1024),
            Some(recovery),
            |_| {},
        )
        .unwrap();
        assert!(notice.recv_timeout(Duration::from_secs(3)).unwrap());
        let started = Instant::now();
        if stop {
            runtime.request_stop();
        }
        assert_eq!(
            join(&mut runtime),
            if stop {
                Exit::Stopped
            } else {
                Exit::AuthorizationEnded
            }
        );
        if stop {
            assert!(started.elapsed() < Duration::from_secs(2));
        }
        peer.join().unwrap();
    }
}
#[test]
fn only_transport_errors_retry_and_progress_requires_current_epoch_new_pixels() {
    use crate::live_relay_media_socket::Error as SocketError;
    for error in [
        TransportError::Tls,
        TransportError::Upgrade,
        TransportError::Profile,
        TransportError::Media(SocketError::Message),
        TransportError::Media(SocketError::Frame),
        TransportError::Media(SocketError::Worker),
        TransportError::Address,
    ] {
        assert!(!error.recoverable(), "{error:?}");
    }
    for error in [
        TransportError::Connect,
        TransportError::Deadline,
        TransportError::Media(SocketError::Transport),
        TransportError::Media(SocketError::Liveness),
    ] {
        assert!(error.recoverable());
    }
    let mut recovery = Recovery::new(1, |_| Err(()), |_| {});
    recovery.attempts = 4;
    recovery.progress(Cursor {
        epoch: 2,
        frame_id: 100,
    });
    recovery.progress(Cursor {
        epoch: 1,
        frame_id: 0,
    });
    assert_eq!(recovery.attempts, 4);
    recovery.progress(Cursor {
        epoch: 1,
        frame_id: 1,
    });
    assert_eq!(recovery.attempts, 0);
    recovery.attempts = 4;
    recovery.progress(Cursor {
        epoch: 1,
        frame_id: 1,
    });
    assert_eq!(recovery.attempts, 4);
    let media = session();
    let control = Control {
        stop: AtomicBool::new(false),
        interrupt: Mutex::new(None),
        permit: media.admit(1).unwrap(),
        session: media,
        worker: Mutex::new(None),
    };
    let exit = Exit::Transport(TransportError::Connect);
    assert!(
        matches!(recovery.retry(exit, &control, Cursor { epoch: 1, frame_id: 1 }), Err(e) if e == exit)
    );
}
