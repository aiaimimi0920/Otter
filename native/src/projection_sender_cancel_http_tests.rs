//! 在 read/Create 阻塞中精确撤销原 socket；磁盘外改不能覆盖，原 envelope/PNG 不丢失。
use super::*;
fn assert_closed(stream: &mut impl Read) {
    match stream.read(&mut [0]) {
        Ok(0) => {}
        Err(error)
            if matches!(
                error.kind(),
                io::ErrorKind::UnexpectedEof
                    | io::ErrorKind::ConnectionAborted
                    | io::ErrorKind::ConnectionReset
                    | io::ErrorKind::NotConnected
                    | io::ErrorKind::BrokenPipe
            ) => {}
        result => panic!("expected closed socket, not data or timeout: {result:?}"),
    }
}
#[test]
fn sender_cancel_during_read_or_create_interrupts_socket_and_preserves_durable_original() {
    for create in [false, true] {
        let (listener, client, tls) = fixture();
        let (directory, store, request) = prepared(&client);
        let before: Value =
            serde_json::from_slice(&std::fs::read(directory.path()).unwrap()).unwrap();
        let (arrived, ready) = std::sync::mpsc::sync_channel(1);
        let peer = std::thread::spawn(move || {
            if create {
                let mut stream = receive(&listener, tls.clone());
                assert!(http_request(&mut stream)
                    .0
                    .starts_with("POST /v1/projections/read "));
                reply(&mut stream, 404, MISSING);
            }
            let mut stream = receive(&listener, tls);
            assert!(http_request(&mut stream).0.starts_with(if create {
                "POST /v1/projections/create "
            } else {
                "POST /v1/projections/read "
            }));
            arrived.send(()).unwrap();
            assert_closed(&mut stream);
            listener
        });
        let mut grant = grant(&client);
        let mut job = start(&client, &mut grant, request.clone());
        ready.recv_timeout(Duration::from_secs(5)).unwrap();
        store.cancel(request.intent()).unwrap();
        wait(&job);
        assert!(matches!(job.try_join(), Some(Err(JobError::Stopped))));
        let mut expected = before;
        expected["entries"][0]["cancelled"] = true.into();
        assert_eq!(
            serde_json::from_slice::<Value>(&std::fs::read(directory.path()).unwrap()).unwrap(),
            expected
        );
        assert!(store.pending(&AtomicBool::new(false)).unwrap()[0].cancelled());
        no_request(&peer.join().unwrap());
    }
}
#[test]
fn sender_external_edit_makes_cancel_fail_closed_but_still_interrupts_original_socket() {
    let (listener, client, tls) = fixture();
    let (directory, store, request) = prepared(&client);
    let (arrived, ready) = std::sync::mpsc::sync_channel(1);
    let peer = std::thread::spawn(move || {
        let mut stream = receive(&listener, tls);
        http_request(&mut stream);
        arrived.send(()).unwrap();
        assert_closed(&mut stream);
        listener
    });
    let mut grant = grant(&client);
    let mut job = start(&client, &mut grant, request.clone());
    ready.recv_timeout(Duration::from_secs(5)).unwrap();
    let external = b"external bytes are not ours";
    std::fs::write(directory.path(), external).unwrap();
    assert_eq!(store.cancel(request.intent()), Err(sender::Error::Changed));
    wait(&job);
    assert!(job.try_join().unwrap().is_err());
    assert_eq!(std::fs::read(directory.path()).unwrap(), external);
    assert!(store.pending(&AtomicBool::new(false)).is_err());
    no_request(&peer.join().unwrap());
}
#[test]
fn sender_revoked_running_grant_durably_cancels_without_creating_or_unlinking() {
    let (listener, client, tls) = fixture();
    let (_directory, store, request) = prepared(&client);
    let (arrived, ready) = std::sync::mpsc::sync_channel(1);
    let peer = std::thread::spawn(move || {
        let mut stream = receive(&listener, tls);
        http_request(&mut stream);
        arrived.send(()).unwrap();
        assert_closed(&mut stream);
        listener
    });
    let mut grant = grant(&client);
    let mut job = start(&client, &mut grant, request);
    ready.recv_timeout(Duration::from_secs(5)).unwrap();
    drop(grant);
    wait(&job);
    assert!(matches!(job.try_join(), Some(Err(JobError::Stopped))));
    assert!(store.pending(&AtomicBool::new(false)).unwrap()[0].cancelled());
    no_request(&peer.join().unwrap());
}
