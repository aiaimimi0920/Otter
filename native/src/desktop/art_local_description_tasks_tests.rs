//! 描述阶段复用下载任务的槽位、通知和撤销；不让 UI 先同步联网换取 metadata。
use super::*;
use crate::art_local_requests::{description, tests as fixture};
use std::{sync::mpsc, time::Duration};
const WAIT: Duration = Duration::from_secs(5);
fn request() -> description::Request {
    description::Request {
        origin: fixture::request().binding.origin,
        workspace: "workspace:1".into(),
        instance: "instance:1".into(),
        art: "vendor/demo".into(),
    }
}
#[test]
fn retiring_description_keeps_capacity_and_cannot_publish_old_error() {
    let (tx, rx) = mpsc::channel();
    let mut tasks = Tasks::new(
        "workspace:1".into(),
        Arc::new(move || {
            let _ = tx.send(());
        }),
    );
    let mut owner = fixture::owner();
    let mut trust = fixture::authority();
    let first = fixture::lease(&mut owner);
    let current = fixture::lease(&mut owner);
    let (release, wait) = mpsc::channel();
    let (release_current, wait_current) = mpsc::channel();
    let old = tasks
        .start_description_with(
            fixture::id(),
            request(),
            first.clone(),
            &mut trust,
            move |_| {
                wait.recv_timeout(WAIT).unwrap();
                Err(Error::Worker)
            },
        )
        .unwrap();
    let new = tasks
        .start_description_with(
            fixture::id(),
            request(),
            current.clone(),
            &mut trust,
            move |_| {
                wait_current.recv_timeout(WAIT).unwrap();
                Err(Error::Description(
                    crate::live_relay_http::art_describe::Error::Wire,
                ))
            },
        )
        .unwrap();
    assert!(first.is_revoked());
    assert!(new.generation > old.generation);
    assert!(!tasks.has_capacity());
    assert!(matches!(
        tasks.start_description_with(
            fixture::id(),
            request(),
            fixture::lease(&mut owner),
            &mut trust,
            |_| panic!("must not spawn")
        ),
        Err(Error::Capacity)
    ));
    assert!(!current.is_revoked());
    release.send(()).unwrap();
    rx.recv_timeout(WAIT).unwrap();
    assert!(tasks.take_settled().is_none());
    assert!(tasks.has_capacity());
    assert!(!current.is_revoked());
    release_current.send(()).unwrap();
    rx.recv_timeout(WAIT).unwrap();
    assert!(matches!(
        tasks.take_settled().unwrap().result,
        Err(Error::Description(_))
    ));
    assert!(current.is_revoked());
    assert!(!tasks.busy());
}
#[test]
fn configured_description_failure_is_delivered_and_revokes_original_lease() {
    let (tx, rx) = mpsc::channel();
    let mut tasks = Tasks::new(
        "workspace:1".into(),
        Arc::new(move || {
            let _ = tx.send(());
        }),
    );
    let mut owner = fixture::owner();
    let mut trust = fixture::authority();
    let lease = fixture::lease(&mut owner);
    let request = request();
    let absent = std::env::temp_dir().join(format!(
        "absent-description-ca-{:?}.pem",
        unsafe { windows::Win32::System::Com::CoCreateGuid() }.unwrap()
    ));
    let config = Config::from_lookup(&request.origin, |key| match key {
        "OTTER_NATIVE_PROFILE_DIR" => Some(std::env::temp_dir().into_os_string()),
        "OTTER_LOOM_TLS_ORIGIN" => Some(request.origin.clone().into()),
        "OTTER_LOOM_TLS_CA_FILE" => Some(absent.clone().into_os_string()),
        _ => None,
    })
    .unwrap();
    tasks
        .start_description(fixture::id(), request, lease.clone(), &mut trust, config)
        .unwrap();
    rx.recv_timeout(WAIT).unwrap();
    assert!(matches!(
        tasks.take_settled().unwrap().result,
        Err(Error::Configuration(_))
    ));
    assert!(lease.is_revoked());
    assert!(!tasks.busy());
}
