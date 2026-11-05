//! 原 Job 准备回调的确定性失败/晚撤权/四槽交接；不把 worker 退出当意图已经清理。
use super::*;
fn configured(
    client: &Client,
    directory: &Directory,
    pem: &str,
) -> crate::device_authorization_config::Config {
    let ca = directory.0.join("flow-ca.pem");
    std::fs::write(&ca, pem).unwrap();
    crate::device_authorization_config::Config::from_lookup(client.origin(), |key| match key {
        "OTTER_NATIVE_PROFILE_DIR" => Some(directory.0.as_os_str().to_owned()),
        "OTTER_LOOM_TLS_ORIGIN" => Some(client.origin().into()),
        "OTTER_LOOM_TLS_CA_FILE" => Some(ca.as_os_str().to_owned()),
        _ => None,
    })
    .unwrap()
}
#[test]
fn sender_tracked_preparation_error_and_panic_keep_original_request_after_join() {
    for panic in [false, true] {
        let (listener, client, _, pem) = crate::device_session_client::tests::fixture_with_pem();
        let (directory, _store, request) = prepared(&client);
        let original = request.clone();
        let config = configured(&client, &directory, &pem);
        let mut grant = grant(&client);
        let mut job = Job::start_sender(
            config,
            grant.lease(client.origin(), 0).unwrap(),
            Box::new(move |_, track| {
                track(request);
                assert!(!panic, "synthetic panic after durable publication");
                Err(JobError::Sender(sender::Error::RecoveryRetained))
            }),
        )
        .unwrap();
        wait(&job);
        assert!(
            job.take_sender_request().is_none(),
            "handoff cannot race the worker"
        );
        assert!(job.try_join().unwrap().is_err());
        let recovered = job.take_sender_request().unwrap();
        assert!(recovered.same_intent(&original));
        assert!(job.take_sender_request().is_none());
        assert!(!recovered.intent().cancelled());
        no_request(&listener);
    }
}
#[test]
fn sender_tracked_preparation_late_revocation_keeps_original_and_does_not_connect() {
    let (listener, client, _, pem) = crate::device_session_client::tests::fixture_with_pem();
    let (directory, _store, request) = prepared(&client);
    let original = request.clone();
    let config = configured(&client, &directory, &pem);
    let mut grant = grant(&client);
    let mut job = Job::start_sender(
        config,
        grant.lease(client.origin(), 0).unwrap(),
        Box::new(move |_, track| {
            track(request);
            Err(JobError::Sender(sender::Error::Write))
        }),
    )
    .unwrap();
    wait(&job);
    job.request_stop();
    assert!(matches!(job.try_join(), Some(Err(JobError::Stopped))));
    assert!(job.take_sender_request().unwrap().same_intent(&original));
    no_request(&listener);
}
#[test]
fn sender_preparation_uses_original_four_slots_until_all_completed_jobs_are_reaped() {
    let (listener, client, _, pem) = crate::device_session_client::tests::fixture_with_pem();
    let (directory, _store, _) = prepared(&client);
    let config = || configured(&client, &directory, &pem);
    let mut grant = grant(&client);
    let mut jobs = Vec::new();
    for _ in 0..4 {
        let job = Job::start_sender(
            config(),
            grant.lease(client.origin(), 0).unwrap(),
            Box::new(|_, _| Err(JobError::Stopped)),
        )
        .unwrap();
        wait(&job);
        jobs.push(job);
    }
    assert_eq!(
        Job::start_sender(
            config(),
            grant.lease(client.origin(), 0).unwrap(),
            Box::new(|_, _| panic!("fifth worker must not start"))
        )
        .err()
        .unwrap()
        .kind(),
        std::io::ErrorKind::WouldBlock
    );
    for mut job in jobs {
        assert!(job.try_join().unwrap().is_err());
    }
    let mut admitted = Job::start_sender(
        config(),
        grant.lease(client.origin(), 0).unwrap(),
        Box::new(|_, _| Err(JobError::Stopped)),
    )
    .unwrap();
    wait(&admitted);
    assert!(admitted.try_join().unwrap().is_err());
    no_request(&listener);
}

#[test]
fn sender_ui_join_never_rereads_outbox_but_later_validation_still_requires_baseline() {
    use std::os::windows::fs::OpenOptionsExt;
    for prepare in [false, true] {
        let (listener, client, tls, pem) = crate::device_session_client::tests::fixture_with_pem();
        let (directory, _store, request) = prepared(&client);
        let expected = response(&request);
        let peer = std::thread::spawn(move || {
            let mut stream = receive(&listener, tls);
            assert!(http_request(&mut stream)
                .0
                .starts_with("POST /v1/projections/read "));
            reply(&mut stream, 200, &serde_json::to_vec(&expected).unwrap());
            listener
        });
        let mut grant = grant(&client);
        let mut job = if prepare {
            Job::start_sender(
                configured(&client, &directory, &pem),
                grant.lease(client.origin(), 0).unwrap(),
                Box::new(move |_, track| {
                    track(request.clone());
                    Ok(crate::projection_delivery_runtime::SenderInput {
                        request,
                        source: sender::binding::tests::source("unit-1", &"a".repeat(64)),
                        name: "Loopback Target".into(),
                    })
                }),
            )
            .unwrap()
        } else {
            start(&client, &mut grant, request)
        };
        wait(&job);
        // 内容未变，worker 已退出；独占句柄确定性阻断任何 join-time 基线重读。
        let exclusive = std::fs::OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(directory.path())
            .unwrap();
        let Output::Sent(ready) = job.try_join().unwrap().unwrap() else {
            panic!("missing sender result")
        };
        let current = grant.lease(client.origin(), 0).unwrap();
        assert!(ready.authorized(&current, 0));
        // Ready 元数据不是后续网络许可；完整校验仍须后台读原基线。
        assert!(ready.validate(&current, 0).is_err());
        drop(exclusive);
        assert!(ready.validate(&current, 0).is_ok());
        no_request(&peer.join().unwrap());
    }
}
