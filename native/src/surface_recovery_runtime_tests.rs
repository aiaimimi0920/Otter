use super::*;
use crate::surface_recovery_runtime::{Error, Job as QueryJob};
#[test]
fn recovery_query_cancel_and_late_revocation_preserve_request_but_drop_candidate() {
    for mode in 0..3 {
        let cancel_in_flight = mode == 0;
        let (listener, client, config) = fixture();
        let origin = client.origin.clone();
        let mut grant = Grant::new(session(&origin, u64::MAX));
        let lease = grant.lease(&origin, 0).unwrap();
        let (sent_tx, sent_rx) = std::sync::mpsc::channel();
        let (finish_tx, finish_rx) = std::sync::mpsc::channel();
        let peer = std::thread::spawn(move || {
            let mut stream =
                StreamOwned::new(ServerConnection::new(config).unwrap(), accept(&listener));
            let (head, _) = request(&mut stream);
            assert!(head.starts_with("GET /v1/surfaces/stream?after=0&timeoutMs=0 "));
            sent_tx.send(()).unwrap();
            if cancel_in_flight {
                let _ = finish_rx.recv_timeout(std::time::Duration::from_secs(8));
            } else {
                let body = serde_json::json!({"protocolVersion":"loom.surface-stream.v1","next":1,"reset":false,
                    "messages":[{"method":"loom.surface.snapshot","params":{"hookNodeId":"node:1","generation":2,
                        "snapshot":{"protocolVersion":"loom.surface.v1","instanceId":"surface:1","attachmentId":"attachment:1","artId":"art:1","revision":3}}}]});
                reply(&mut stream, 200, &serde_json::to_vec(&body).unwrap());
            }
        });
        let original = request_data();
        let mut job = QueryJob::start_prepared(client, lease, original.clone()).unwrap();
        sent_rx
            .recv_timeout(std::time::Duration::from_secs(5))
            .unwrap();
        let start = std::time::Instant::now();
        if cancel_in_flight {
            job.request_stop();
        }
        assert_eq!(
            unsafe {
                WaitForSingleObject(HANDLE(job.wait_handle().unwrap().as_raw_handle()), 5000)
            },
            WAIT_OBJECT_0
        );
        if mode != 2 {
            drop(grant);
        }
        let outcome = job.try_join().unwrap();
        let elapsed = start.elapsed();
        let _ = finish_tx.send(());
        peer.join().unwrap();
        if mode == 2 {
            assert_eq!(outcome.result.unwrap().unwrap().instance(), "surface:1");
        } else {
            assert_eq!(outcome.result, Err(Error::Stopped));
        }
        assert!(Arc::ptr_eq(&original, &outcome.request));
        assert_eq!(outcome.origin, origin);
        assert_eq!(outcome.device, "device-1");
        assert!(job.wait_handle().is_none() && job.try_join().is_none());
        if cancel_in_flight {
            assert!(elapsed < std::time::Duration::from_secs(2));
        }
    }
}
