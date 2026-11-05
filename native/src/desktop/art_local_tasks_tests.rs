//! 有界任务占槽、通知、旧失败、panic、撤销和回收；通过通道同步而非 sleep 刷绿。
use super::*;
use crate::art_local_requests::tests as fixture;
use std::{sync::mpsc, time::Duration};
const WAIT: Duration = Duration::from_secs(5);
fn tasks() -> (Tasks, mpsc::Receiver<()>) {
    let (tx, rx) = mpsc::channel();
    (
        Tasks::new(
            "workspace:1".into(),
            Arc::new(move || {
                let _ = tx.send(());
            }),
        ),
        rx,
    )
}
#[test]
fn replaced_failure_is_discarded_and_retiring_worker_keeps_capacity() {
    let (mut tasks, done) = tasks();
    let mut owner = fixture::owner();
    let mut trust = fixture::authority();
    let old = fixture::lease(&mut owner);
    let current = fixture::lease(&mut owner);
    let (release_old, wait_old) = mpsc::channel();
    let (release_new, wait_new) = mpsc::channel();
    tasks
        .start_with(
            fixture::id(),
            fixture::request(),
            old.clone(),
            &mut trust,
            move |_| {
                wait_old.recv_timeout(WAIT).unwrap();
                Err(Error::Worker)
            },
        )
        .unwrap();
    tasks
        .start_with(
            fixture::id(),
            fixture::request(),
            current.clone(),
            &mut trust,
            move |_| {
                wait_new.recv_timeout(WAIT).unwrap();
                Err(Error::Configuration(
                    crate::device_authorization_config::Error::Trust,
                ))
            },
        )
        .unwrap();
    assert!(old.is_revoked());
    assert!(!current.is_revoked());
    let third = fixture::lease(&mut owner);
    assert!(matches!(
        tasks.start_with(
            fixture::id(),
            fixture::request(),
            third.clone(),
            &mut trust,
            |_| panic!("capacity must reject before worker")
        ),
        Err(Error::Capacity)
    ));
    assert!(!third.is_revoked());
    assert!(tasks.take_settled().is_none());
    release_old.send(()).unwrap();
    done.recv_timeout(WAIT).unwrap();
    assert!(tasks.take_settled().is_none());
    assert!(tasks.busy());
    assert!(!current.is_revoked());
    release_new.send(()).unwrap();
    done.recv_timeout(WAIT).unwrap();
    let report = tasks.take_settled().unwrap();
    assert_eq!(report.unit, fixture::id());
    assert!(matches!(report.result, Err(Error::Configuration(_))));
    assert!(current.is_revoked());
    assert!(!tasks.busy());
    assert!(tasks.take_settled().is_none());
}
#[test]
fn worker_panic_notifies_and_revokes_current_request() {
    let (mut tasks, done) = tasks();
    let mut owner = fixture::owner();
    let lease = fixture::lease(&mut owner);
    let mut trust = fixture::authority();
    tasks
        .start_with(
            fixture::id(),
            fixture::request(),
            lease.clone(),
            &mut trust,
            |_| panic!("injected local Art worker panic"),
        )
        .unwrap();
    done.recv_timeout(WAIT).unwrap();
    assert!(matches!(
        tasks.take_settled().unwrap().result,
        Err(Error::Worker)
    ));
    assert!(lease.is_revoked());
    assert!(!tasks.busy());
}
#[test]
fn deleted_nodes_and_cancelled_results_cannot_be_delivered() {
    for remove in [false, true] {
        let (mut tasks, done) = tasks();
        let mut owner = fixture::owner();
        let mut trust = fixture::authority();
        let lease = fixture::lease(&mut owner);
        let (release, wait) = mpsc::channel();
        tasks
            .start_with(
                fixture::id(),
                fixture::request(),
                lease.clone(),
                &mut trust,
                move |_| {
                    wait.recv_timeout(WAIT).unwrap();
                    Err(Error::Worker)
                },
            )
            .unwrap();
        if remove {
            tasks.retain_units(|_| false);
        } else {
            tasks.cancel(&fixture::id());
        }
        assert!(lease.is_revoked());
        assert!(tasks.busy());
        release.send(()).unwrap();
        done.recv_timeout(WAIT).unwrap();
        assert!(tasks.take_settled().is_none());
        assert!(!tasks.busy());
    }
}
#[test]
fn shutdown_revokes_all_workers_before_joining_and_leaves_no_results() {
    let (mut tasks, done) = tasks();
    let mut owner = fixture::owner();
    let mut trust = fixture::authority();
    let first = fixture::lease(&mut owner);
    let second = fixture::lease(&mut owner);
    let observed = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    for (index, lease) in [first.clone(), second.clone()].into_iter().enumerate() {
        let other = if index == 0 {
            second.clone()
        } else {
            first.clone()
        };
        let own = lease.clone();
        let finished = observed.clone();
        tasks
            .start_with(
                UnitId::parse(&format!("node:{index}")).unwrap(),
                fixture::request(),
                lease,
                &mut trust,
                move |_| {
                    assert!(!own.wait(WAIT));
                    assert!(!other.wait(WAIT));
                    finished.fetch_add(1, Ordering::Release);
                    Err(Error::Worker)
                },
            )
            .unwrap();
    }
    tasks.shutdown();
    done.recv_timeout(WAIT).unwrap();
    done.recv_timeout(WAIT).unwrap();
    assert!(first.is_revoked() && second.is_revoked());
    // shutdown 忽略 join 的 panic；主测试必须确认两个 worker 都实际通过撤销断言。
    assert_eq!(observed.load(Ordering::Acquire), 2);
    assert!(!tasks.busy());
    assert!(tasks.take_settled().is_none());
}
#[test]
fn config_origin_rejection_preserves_current_owner_without_worker() {
    let (mut tasks, done) = tasks();
    let mut owner = fixture::owner();
    let mut trust = fixture::authority();
    let config = Config::new("https://other.example", &std::env::temp_dir(), "test").unwrap();
    let lease = fixture::lease(&mut owner);
    assert!(matches!(
        tasks.start(
            fixture::id(),
            fixture::request(),
            lease.clone(),
            &mut trust,
            config
        ),
        Err(Error::Configuration(
            crate::device_authorization_config::Error::Origin
        ))
    ));
    assert!(!lease.is_revoked());
    assert!(!tasks.busy());
    assert!(done.try_recv().is_err());
}

#[test]
fn client_preparation_failure_uses_same_completion_and_releases_original_lease() {
    let (mut tasks, done) = tasks();
    let mut owner = fixture::owner();
    let mut trust = fixture::authority();
    let lease = fixture::lease(&mut owner);
    let request = fixture::request();
    let absent = std::env::temp_dir().join(format!(
        "absent-art-ca-{:?}.pem",
        unsafe { windows::Win32::System::Com::CoCreateGuid() }.unwrap()
    ));
    let config = Config::from_lookup(&request.binding.origin, |key| match key {
        "OTTER_NATIVE_PROFILE_DIR" => Some(std::env::temp_dir().into_os_string()),
        "OTTER_LOOM_TLS_ORIGIN" => Some(request.binding.origin.clone().into()),
        "OTTER_LOOM_TLS_CA_FILE" => Some(absent.clone().into_os_string()),
        _ => None,
    })
    .unwrap();
    tasks
        .start(fixture::id(), request, lease.clone(), &mut trust, config)
        .unwrap();
    done.recv_timeout(WAIT).unwrap();
    assert!(matches!(
        tasks.take_settled().unwrap().result,
        Err(Error::Configuration(
            crate::device_authorization_config::Error::Trust
        ))
    ));
    assert!(lease.is_revoked());
    assert!(!tasks.busy());
}
