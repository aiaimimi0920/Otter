//! 新建 Accept 精确交接；复用意图/Ready 不获自动解绑资格，真实 TLS 故障不提前释放许可。
use super::*;
use crate::projection_delivery::{
    accept::tests as png,
    journal::{tests::Directory, Store},
};

fn start(client: Client, grant: &mut Grant, store: &Store) -> Job {
    let origin = client.origin().to_owned();
    Job::start_prepared(
        client,
        grant.lease(&origin, 0).unwrap(),
        Operation::Accept {
            invitation: Box::new(png::invitation(&origin)),
            journal: store.clone(),
        },
    )
    .unwrap()
}
fn unchanged_except_marker(before: &[u8], after: &[u8]) {
    let mut expected: serde_json::Value = serde_json::from_slice(before).unwrap();
    expected["entries"][0]["cleanup"] = true.into();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(after).unwrap(),
        expected
    );
}
#[test]
fn accept_abandonment_new_http_and_decode_errors_keep_permit_until_exact_reap() {
    for mode in 0..3 {
        let directory = Directory::new();
        let store = Store::open(&directory.0).unwrap();
        let (listener, client, tls) = fixture();
        let peer = std::thread::spawn(move || {
            let mut stream =
                StreamOwned::new(ServerConnection::new(tls).unwrap(), accept(&listener));
            assert!(request(&mut stream)
                .0
                .starts_with("POST /v1/projections/accept "));
            if mode == 1 {
                reply(&mut stream, 403, br#"{"error":"access_denied"}"#);
            }
            if mode == 2 {
                reply(&mut stream, 200, b"broken accept snapshot");
            }
            listener
        });
        let mut grant = grant(&client);
        let mut job = start(client, &mut grant, &store);
        wait(&job);
        let before = std::fs::read(directory.path()).unwrap();
        let mut original = store.pending().unwrap().pop().unwrap();
        assert_eq!(original.activate(), Err(delivery::journal::Error::Busy));
        assert!(job.take_abandoned().is_none());
        let slots: Vec<_> = (0..3)
            .map(|_| crate::live_relay_control_runtime::Slot::acquire().unwrap())
            .collect();
        assert!(crate::live_relay_control_runtime::Slot::acquire().is_err());
        assert!(job.try_join().unwrap().is_err());
        let extra = crate::live_relay_control_runtime::Slot::acquire().unwrap();
        let abandoned = job.take_abandoned().unwrap();
        assert!(abandoned.same_record(&original));
        assert!(job.take_abandoned().is_none());
        drop(abandoned.queue_cleanup().unwrap());
        unchanged_except_marker(&before, &std::fs::read(directory.path()).unwrap());
        assert!(
            matches!(peer.join().unwrap().accept(), Err(e) if e.kind() == io::ErrorKind::WouldBlock)
        );
        drop((slots, extra));
        drop((job, original, store));
        assert!(Store::open(&directory.0).unwrap().pending().unwrap()[0].cleanup_pending());
    }
}
#[test]
fn accept_abandonment_reused_intent_is_not_assumed_unsaved_even_after_http_failure() {
    let directory = Directory::new();
    let store = Store::open(&directory.0).unwrap();
    let (listener, client, tls) = fixture();
    let origin = client.origin().to_owned();
    let old = store.begin(&png::invitation(&origin), "device-1").unwrap();
    let before = std::fs::read(directory.path()).unwrap();
    let peer = std::thread::spawn(move || {
        let mut stream = StreamOwned::new(ServerConnection::new(tls).unwrap(), accept(&listener));
        request(&mut stream);
        reply(&mut stream, 403, br#"{"error":"access_denied"}"#);
    });
    let mut grant = grant(&client);
    let mut job = start(client, &mut grant, &store);
    wait(&job);
    assert!(job.try_join().unwrap().is_err());
    assert!(job.take_abandoned().is_none());
    assert_eq!(std::fs::read(directory.path()).unwrap(), before);
    let mut retry = old.retry();
    retry.activate().unwrap();
    peer.join().unwrap();
}
#[test]
fn accept_abandonment_completed_success_revoked_before_join_cannot_become_ready() {
    let directory = Directory::new();
    let store = Store::open(&directory.0).unwrap();
    let (listener, client, tls) = fixture();
    let origin = client.origin().to_owned();
    let response = serde_json::to_vec(&png::response(&origin)).unwrap();
    let peer = std::thread::spawn(move || {
        let mut stream = StreamOwned::new(ServerConnection::new(tls).unwrap(), accept(&listener));
        request(&mut stream);
        reply(&mut stream, 200, &response);
    });
    let mut grant = grant(&client);
    let mut job = start(client, &mut grant, &store);
    wait(&job);
    drop(grant);
    assert!(matches!(job.try_join(), Some(Err(JobError::Stopped))));
    drop(job.take_abandoned().unwrap().queue_cleanup().unwrap());
    assert!(store.pending().unwrap()[0].cleanup_pending());
    peer.join().unwrap();
}
#[test]
fn accept_abandonment_ready_transfer_has_no_duplicate_compensation_ticket() {
    let directory = Directory::new();
    let store = Store::open(&directory.0).unwrap();
    let (listener, client, tls) = fixture();
    let origin = client.origin().to_owned();
    let response = serde_json::to_vec(&png::response(&origin)).unwrap();
    let peer = std::thread::spawn(move || {
        let mut stream = StreamOwned::new(ServerConnection::new(tls).unwrap(), accept(&listener));
        request(&mut stream);
        reply(&mut stream, 200, &response);
    });
    let mut grant = grant(&client);
    let mut job = start(client, &mut grant, &store);
    wait(&job);
    let Output::Received(ready) = job.try_join().unwrap().unwrap() else {
        panic!("missing ready")
    };
    assert!(job.take_abandoned().is_none());
    assert!(!store.pending().unwrap()[0].cleanup_pending());
    let mut retry = store.pending().unwrap().pop().unwrap();
    assert_eq!(retry.activate(), Err(delivery::journal::Error::Busy));
    drop(ready);
    retry.activate().unwrap();
    peer.join().unwrap();
}
