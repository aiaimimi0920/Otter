//! accept 真实 HTTPS→后台完整 PNG→原 Grant 交付；不访问真实 Loom，不发送虚假 displayed。
use super::*;
use crate::{
    device_authorization_lease::Grant,
    projection_delivery::accept::tests as pixels,
    projection_delivery_runtime::{Job, Operation, Output},
};
use std::os::windows::io::AsRawHandle;
use windows::Win32::{
    Foundation::{HANDLE, WAIT_OBJECT_0},
    System::Threading::WaitForSingleObject,
};

fn wait(job: &Job) {
    assert_eq!(
        unsafe { WaitForSingleObject(HANDLE(job.wait_handle().unwrap().as_raw_handle()), 5000) },
        WAIT_OBJECT_0
    );
}
fn accept_operation(origin: &str) -> Operation {
    let directory = delivery::journal::tests::Directory::new();
    Operation::Accept {
        invitation: Box::new(pixels::invitation(origin)),
        journal: delivery::journal::Store::open(&directory.0).unwrap(),
    }
}
fn start_reply(
    response: impl FnOnce(&str) -> serde_json::Value + Send + 'static,
) -> (Client, std::thread::JoinHandle<()>) {
    let (listener, client, tls) = fixture();
    let origin = client.origin().to_owned();
    let peer = std::thread::spawn(move || {
        let mut stream = StreamOwned::new(ServerConnection::new(tls).unwrap(), accept(&listener));
        let (head, body) = request(&mut stream);
        assert!(head.starts_with("POST /v1/projections/accept HTTP/1.1"));
        let lower = head.to_ascii_lowercase();
        assert!(lower.contains("authorization: device fixture-secret\r\n"));
        assert_eq!(
            lower
                .lines()
                .find_map(|line| line.strip_prefix("x-loom-device-nonce: "))
                .unwrap()
                .len(),
            32
        );
        let entry = pixels::invitation(&origin);
        assert_eq!(
            body,
            serde_json::from_slice::<serde_json::Value>(&entry.accept_body(&origin).unwrap())
                .unwrap()
        );
        reply(
            &mut stream,
            200,
            &serde_json::to_vec(&response(&origin)).unwrap(),
        );
    });
    (client, peer)
}
#[test]
fn authenticated_accept_returns_compressed_pixels_but_does_not_report_displayed() {
    let (client, peer) = start_reply(pixels::response);
    let cancel = Cancel::new();
    let authorized = session(client.origin(), u64::MAX);
    let received = Control::new(&client, &authorized, &cancel)
        .accept_projection(&pixels::invitation(client.origin()))
        .unwrap();
    // HTTP 结果仍未经过 COM 解码；Debug 不泄露图片/签名/nonce。
    let debug = format!("{received:?}");
    assert!(!debug.contains("imageBase64") && !debug.contains(&"s".repeat(86)));
    peer.join().unwrap();
}
#[test]
fn shared_loom_busy_before_accept_retries_the_exact_logged_request() {
    let (listener, client, tls) = fixture();
    let origin = client.origin().to_owned();
    let server_origin = origin.clone();
    let peer = std::thread::spawn(move || {
        let mut first = StreamOwned::new(
            ServerConnection::new(tls.clone()).unwrap(),
            accept(&listener),
        );
        let (head, body) = request(&mut first);
        assert!(head.starts_with("POST /v1/projections/accept "));
        reply(&mut first, 429, br#"{"error":{"code":"projection_busy"}}"#);
        let mut second = StreamOwned::new(ServerConnection::new(tls).unwrap(), accept(&listener));
        let (retry_head, retry_body) = request(&mut second);
        assert!(retry_head.starts_with("POST /v1/projections/accept "));
        assert_eq!(retry_body, body);
        assert_ne!(head, retry_head); // 独立 nonce，不重放原 HTTP 请求。
        reply(
            &mut second,
            200,
            &serde_json::to_vec(&pixels::response(&server_origin)).unwrap(),
        );
    });
    let mut grant = Grant::new(session(&origin, u64::MAX));
    let mut job = Job::start_prepared(
        client,
        grant.lease(&origin, 0).unwrap(),
        accept_operation(&origin),
    )
    .unwrap();
    wait(&job);
    assert!(matches!(job.try_join(), Some(Ok(Output::Received(_)))));
    peer.join().unwrap();
}
#[test]
fn only_definite_shared_busy_can_retire_the_unapplied_accept() {
    use crate::{live_relay_http::Error as Http, projection_delivery_runtime::Error as JobError};
    for (definite, existing) in [(true, false), (true, true), (false, false)] {
        let (listener, client, tls) = fixture();
        let origin = client.origin().to_owned();
        let peer = std::thread::spawn(move || {
            for _ in 0..if definite { 3 } else { 1 } {
                let mut stream = StreamOwned::new(
                    ServerConnection::new(tls.clone()).unwrap(),
                    accept(&listener),
                );
                assert!(request(&mut stream)
                    .0
                    .starts_with("POST /v1/projections/accept "));
                let response: &[u8] = if definite {
                    br#"{"error":{"code":"projection_busy"}}"#
                } else {
                    b"unexpected gateway response"
                };
                reply(&mut stream, 429, response);
            }
            listener
        });
        let directory = delivery::journal::tests::Directory::new();
        let journal = delivery::journal::Store::open(&directory.0).unwrap();
        let invitation = pixels::invitation(&origin);
        if existing {
            journal.begin(&invitation, "device-1").unwrap();
        }
        let mut grant = Grant::new(session(&origin, u64::MAX));
        let mut job = Job::start_prepared(
            client,
            grant.lease(&origin, 0).unwrap(),
            Operation::Accept {
                invitation: Box::new(invitation),
                journal: journal.clone(),
            },
        )
        .unwrap();
        wait(&job);
        assert_eq!(
            job.try_join().unwrap().unwrap_err(),
            match (definite, existing) {
                (true, false) => JobError::AcceptBusyUnapplied,
                (true, true) => JobError::Http(Http::ProjectionBusy),
                (false, _) => JobError::Http(Http::Status(429)),
            }
        );
        assert_eq!(job.take_abandoned().is_none(), definite);
        assert_eq!(journal.pending().unwrap().is_empty(), definite && !existing);
        assert_eq!(
            peer.join().unwrap().accept().unwrap_err().kind(),
            io::ErrorKind::WouldBlock
        );
    }
}
#[test]
fn completed_ready_survives_job_drop_and_only_transfers_with_original_grant() {
    let (client, peer) = start_reply(pixels::response);
    let origin = client.origin().to_owned();
    let mut grant = Grant::new(session(&origin, u64::MAX));
    let destination = grant.lease(&origin, 0).unwrap();
    let mut job = Job::start_prepared(
        client,
        grant.lease(&origin, 0).unwrap(),
        accept_operation(&origin),
    )
    .unwrap();
    wait(&job);
    let Output::Received(ready) = job.try_join().unwrap().unwrap() else {
        panic!("missing PNG ready")
    };
    drop(job);
    peer.join().unwrap();
    assert_eq!(ready.validate(&destination, 0).unwrap().revision(), 5);
    let mut other = Grant::new(session(&origin, u64::MAX));
    assert!(ready
        .validate(&other.lease(&origin, 0).unwrap(), 0)
        .is_err());
    let (image, held) = ready.transfer(&destination, 0).unwrap();
    assert_eq!((image.image().width, image.image().height), (2, 1));
    assert_eq!(held.ticket().unit(), image.unit());
    assert!(held.validate(&destination, 0).is_ok());
    assert!(held.validate(&other.lease(&origin, 0).unwrap(), 0).is_err());
    let mut slots = Vec::new();
    for _ in 0..3 {
        slots.push(crate::live_relay_control_runtime::Slot::acquire().unwrap());
    }
    assert!(crate::live_relay_control_runtime::Slot::acquire().is_err());
    drop(grant);
    assert!(held.validate(&destination, 0).is_err());
    drop(held);
    let _fourth = crate::live_relay_control_runtime::Slot::acquire().unwrap();
}
#[test]
fn ready_pixels_hold_shared_capacity_and_cannot_escape_revocation_after_join() {
    let (client, peer) = start_reply(pixels::response);
    let origin = client.origin().to_owned();
    let mut grant = Grant::new(session(&origin, u64::MAX));
    let destination = grant.lease(&origin, 0).unwrap();
    let mut job = Job::start_prepared(
        client,
        grant.lease(&origin, 0).unwrap(),
        accept_operation(&origin),
    )
    .unwrap();
    wait(&job);
    let Output::Received(ready) = job.try_join().unwrap().unwrap() else {
        panic!("missing ready")
    };
    drop(job);
    peer.join().unwrap();
    let mut slots = Vec::new();
    for _ in 0..3 {
        slots.push(crate::live_relay_control_runtime::Slot::acquire().unwrap());
    }
    assert_eq!(
        crate::live_relay_control_runtime::Slot::acquire()
            .err()
            .unwrap()
            .kind(),
        io::ErrorKind::WouldBlock
    );
    drop(grant);
    assert!(ready.validate(&destination, 0).is_err());
    drop(ready);
    let _fourth = crate::live_relay_control_runtime::Slot::acquire().unwrap();
}
#[test]
fn valid_png_header_and_correct_digest_are_not_enough_for_worker_success() {
    let (client, peer) = start_reply(|origin| {
        use base64::Engine;
        use sha2::{Digest, Sha256};
        let bytes = pixels::png()[..33].to_vec();
        let mut response = pixels::response(origin);
        response["snapshot"]["imageBase64"] = base64::engine::general_purpose::STANDARD
            .encode(&bytes)
            .into();
        response["digest"] = format!("{:x}", Sha256::digest(&bytes)).into();
        response
    });
    let origin = client.origin().to_owned();
    let mut grant = Grant::new(session(&origin, u64::MAX));
    let mut job = Job::start_prepared(
        client,
        grant.lease(&origin, 0).unwrap(),
        accept_operation(&origin),
    )
    .unwrap();
    wait(&job);
    assert!(matches!(
        job.try_join(),
        Some(Err(crate::projection_delivery_runtime::Error::Decode(
            delivery::Error::Snapshot
        )))
    ));
    peer.join().unwrap();
}

#[test]
fn accept_uses_its_own_six_mib_limit_without_expanding_metadata_requests() {
    for oversized in [false, true] {
        use std::io::Write;
        let (listener, client, tls) = fixture();
        let origin = client.origin().to_owned();
        let server_origin = origin.clone();
        let peer = std::thread::spawn(move || {
            let mut stream =
                StreamOwned::new(ServerConnection::new(tls).unwrap(), accept(&listener));
            request(&mut stream);
            if oversized {
                write!(
                    stream,
                    "HTTP/1.1 200 Fixture\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    crate::projection_delivery::accept::MAX_RESPONSE + 1
                )
                .unwrap();
                stream.flush().unwrap();
            } else {
                let mut body = pixels::response(&server_origin);
                body["extension"] = "x".repeat(delivery::MAX_JSON).into();
                let body = serde_json::to_vec(&body).unwrap();
                assert!(body.len() > delivery::MAX_JSON);
                reply(&mut stream, 200, &body);
            }
        });
        let cancel = Cancel::new();
        let authorized = session(&origin, u64::MAX);
        let response = Control::new(&client, &authorized, &cancel)
            .accept_projection(&pixels::invitation(&origin));
        if oversized {
            assert!(matches!(response, Err(LiveError::Http(Error::Limit))));
        } else {
            assert!(response.is_ok());
        }
        peer.join().unwrap();
    }
    assert_eq!(delivery::MAX_JSON, 1024 * 1024);
}
