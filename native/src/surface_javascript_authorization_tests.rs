//! 就绪移交和闲置/操作前后的撤权门；只回收自有测试 child，不伪造系统全局输入。
use super::*;
#[test]
fn javascript_entry_cannot_revive_expired_execution_or_change_lifecycle_without_revision() {
    let now = clock();
    let mut mount = script("https://localhost:1234", b"source");
    mutate(&mut mount, |s| {
        s["resourceLeases"][0]["expiresAtMs"] = (now + 1000).into()
    });
    let entry = mount.javascript_entry(now).unwrap();
    let mut next = MountedSurface {
        binding: mount.binding.clone(),
        snapshot: mount.snapshot.clone(),
    };
    mutate(&mut next, |s| {
        s["revision"] = 4.into();
        s["resourceLeases"][0]["expiresAtMs"] = (now + 3000).into();
    });
    assert!(matches!(
        entry.current(&next, now + 1000, true),
        Err(ResourceError::Expired)
    ));
    let mut changed = mounted();
    changed["instance"]["attachments"]["attachment:1"]["lifecycle"] = "active".into();
    next.binding = Binding::from_mount(
        &serde_json::to_vec(&changed).unwrap(),
        mount.binding.origin(),
        "device-1",
        "node:1",
    )
    .unwrap();
    assert!(matches!(
        entry.current(&next, now, true),
        Err(ResourceError::Identity)
    ));
    next.binding = mount.binding.clone();
    activate(&mut next);
    assert!(entry.current(&next, now, true).is_ok());
}
fn prepared() -> (
    Grant,
    crate::device_authorization_lease::Lease,
    MountedSurface,
    crate::surface_javascript::authorized::Ready,
) {
    prepared_with(crate::surface_javascript::fixture::SOURCE.as_bytes(), false)
}
fn prepared_with(
    source: &'static [u8],
    active: bool,
) -> (
    Grant,
    crate::device_authorization_lease::Lease,
    MountedSurface,
    crate::surface_javascript::authorized::Ready,
) {
    let (listener, client, config) = fixture();
    let mut mount = script(&client.origin, source);
    if active {
        activate(&mut mount);
    }
    let mut grant = Grant::new(session(&client.origin, u64::MAX));
    let destination = grant.lease(&client.origin, clock()).unwrap();
    let peer = std::thread::spawn(move || {
        let mut stream =
            StreamOwned::new(ServerConnection::new(config).unwrap(), accept(&listener));
        request(&mut stream);
        reply(&mut stream, 200, source);
    });
    let input = MountedSurface {
        binding: mount.binding.clone(),
        snapshot: mount.snapshot.clone(),
    };
    let mut job = Job::start_prepared(
        client.clone(),
        grant.lease(&client.origin, clock()).unwrap(),
        input,
    )
    .unwrap();
    wait(&job);
    let ready = job.try_join().unwrap().unwrap();
    peer.join().unwrap();
    drop(job);
    (grant, destination, mount, ready)
}
#[test]
fn javascript_pending_initial_emit_cannot_cross_lifecycle_or_be_bypassed_by_update() {
    let _serial = crate::surface_javascript::fixture::SERIAL.lock().unwrap();
    const SOURCE: &[u8] = br#"NeuroSurface.define({mount({root,emit}){
        root.textContent='ready';
        emit({nodeId:'refresh',event:'click',action:'refresh',class:'discrete',payload:{mount:true}});
    }});"#;
    for scenario in 0..3 {
        let (_grant, destination, mut mount, ready) = prepared_with(SOURCE, true);
        let mut runtime = ready.transfer(&destination, &mount).unwrap();
        let probe = runtime.probe();
        if scenario < 2 {
            let mut changed = mounted();
            let attachment = &mut changed["instance"]["attachments"]["attachment:1"];
            attachment["lifecycle"] = if scenario == 0 { "inactive" } else { "active" }.into();
            attachment["lifecycleRevision"] = 9.into();
            mount.binding = Binding::from_mount(
                &serde_json::to_vec(&changed).unwrap(),
                mount.binding.origin(),
                "device-1",
                "node:1",
            )
            .unwrap();
            assert!(runtime.take_initial(&mount).is_err(), "scenario={scenario}");
        } else {
            mutate(&mut mount, |s| s["revision"] = 4.into());
            assert!(runtime.update(&mount).is_err());
        }
        assert!(!runtime.is_alive() && probe.exited(), "scenario={scenario}");
    }
}
#[test]
fn javascript_ready_rejects_another_grant_even_with_identical_origin_device_and_source() {
    let _serial = crate::surface_javascript::fixture::SERIAL.lock().unwrap();
    let (_grant, _destination, mount, ready) = prepared();
    let probe = ready.probe();
    let mut foreign = Grant::new(session(mount.binding.origin(), u64::MAX));
    let foreign = foreign.lease(mount.binding.origin(), clock()).unwrap();
    assert!(matches!(
        ready.transfer(&foreign, &mount),
        Err(JsError::Stopped)
    ));
    assert!(probe.exited());
}
#[test]
fn javascript_idle_revocation_signals_owner_and_refresh_reaps_original_child() {
    let _serial = crate::surface_javascript::fixture::SERIAL.lock().unwrap();
    let (grant, destination, mount, ready) = prepared();
    let mut runtime = ready.transfer(&destination, &mount).unwrap();
    let probe = runtime.probe();
    drop(grant);
    assert_eq!(
        unsafe { WaitForSingleObject(HANDLE(runtime.revocation_handle().as_raw_handle()), 0) },
        WAIT_OBJECT_0
    );
    assert!(matches!(runtime.refresh(&mount), Err(JsError::Stopped)));
    assert!(!runtime.is_alive() && probe.exited());
}
#[test]
fn javascript_inactive_stale_changed_program_and_expired_entry_cannot_emit_or_keep_child() {
    let _serial = crate::surface_javascript::fixture::SERIAL.lock().unwrap();
    for scenario in 0..5 {
        let (_grant, destination, mut mount, ready) = prepared();
        let mut runtime = ready.transfer(&destination, &mount).unwrap();
        let frame = runtime.take_initial(&mount).unwrap();
        let target = frame.scene.root().children()[1].id().to_owned();
        let probe = runtime.probe();
        if scenario != 0 {
            activate(&mut mount);
        }
        match scenario {
            1 => mutate(&mut mount, |s| s["revision"] = 2.into()),
            2 => mutate(&mut mount, |s| {
                let digest = "a".repeat(64);
                s["revision"] = 4.into();
                s["entryResourceId"] = format!("sha256:{digest}").into();
                s["resourceLeases"][0]["resource"]["resourceId"] =
                    format!("sha256:{digest}").into();
                s["resourceLeases"][0]["transport"]["path"] =
                    format!("/v1/surfaces/resources/{digest}").into();
            }),
            3 => mutate(&mut mount, |s| {
                s["resourceLeases"][0]["expiresAtMs"] = 0.into()
            }),
            4 => mutate(&mut mount, |s| {
                s["resourceLeases"][0]["leaseId"] = "replaced:same-revision".into()
            }),
            _ => {}
        }
        let result = if scenario == 2 {
            runtime.update(&mount)
        } else {
            runtime.click(&mount, &target)
        };
        assert!(result.is_err(), "scenario={scenario}");
        assert!(!runtime.is_alive() && probe.exited(), "scenario={scenario}");
    }
}
#[test]
fn javascript_cancelled_authenticated_download_returns_without_starting_runtime() {
    let (listener, client, config) = fixture();
    let mount = script(&client.origin, b"not delivered source");
    let mut grant = Grant::new(session(&client.origin, u64::MAX));
    let (entered_tx, entered_rx) = std::sync::mpsc::channel();
    let (finish_tx, finish_rx) = std::sync::mpsc::channel();
    let peer = std::thread::spawn(move || {
        let mut stream =
            StreamOwned::new(ServerConnection::new(config).unwrap(), accept(&listener));
        request(&mut stream);
        entered_tx.send(()).unwrap();
        let _ = finish_rx.recv_timeout(std::time::Duration::from_secs(8));
    });
    let mut job = Job::start_prepared(
        client.clone(),
        grant.lease(&client.origin, clock()).unwrap(),
        mount,
    )
    .unwrap();
    entered_rx
        .recv_timeout(std::time::Duration::from_secs(5))
        .unwrap();
    job.request_stop();
    wait(&job);
    assert!(matches!(job.try_join().unwrap(), Err(JsError::Stopped)));
    finish_tx.send(()).unwrap();
    peer.join().unwrap();
}
