//! 工作区撤销和迟到描述结果的 owner 回归；不借此声称完整 Application 交互已验收。
use super::*;
use crate::art_local_requests::{tests as fixture, Error};
use std::{sync::mpsc, time::Duration};

#[test]
fn reset_revokes_pending_description_and_discards_old_workspace_completion() {
    let (notify, ready) = mpsc::channel();
    let mut owner = Owner::new(Arc::new(move || {
        let _ = notify.send(());
    }));
    owner.workspace = Some("workspace:1".into());
    owner.tasks.reset_workspace("workspace:1".into());
    let mut device = fixture::owner();
    let mut authority = fixture::authority();
    let lease = fixture::lease(&mut device);
    let (release, waiting) = mpsc::channel();
    let token = owner
        .tasks
        .start_description_with(
            fixture::id(),
            description::Request {
                origin: fixture::request().binding.origin,
                workspace: "workspace:1".into(),
                instance: "instance:1".into(),
                art: "vendor/demo".into(),
            },
            lease.clone(),
            &mut authority,
            move |_| {
                waiting.recv_timeout(Duration::from_secs(5)).unwrap();
                Err(Error::Worker)
            },
        )
        .unwrap();
    let state = super::super::State::new(None, false).unwrap();
    owner.status_owner = Some(token.clone());
    owner.entries.push(Entry {
        unit: fixture::id(),
        instance: "instance:1".into(),
        token,
        stamp: Stamp::capture(&state),
        inputs: dependencies::Stamp::capture(
            &Workspace::with_initial(fixture::id()),
            &fixture::id(),
        )
        .unwrap(),
        result_stamp: state.execution_results.borrow().output_stamp(),
        active: true,
        images: images::Images::default(),
        window: None,
    });
    owner.reset();
    assert!(lease.is_revoked());
    assert!(owner.workspace.is_none());
    assert!(!owner.entries[0].active);
    assert!(owner.tasks.busy());
    release.send(()).unwrap();
    ready.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(owner.tasks.take_settled().is_none());
    assert!(!owner.tasks.busy());
    assert_eq!(
        owner
            .poll(&[], &mut Workspace::default(), &[], false)
            .as_deref(),
        Some("Art 已停止")
    );
    assert!(owner
        .poll(&[], &mut Workspace::default(), &[], false)
        .is_none());
    owner.shutdown();
    assert!(owner.entries.is_empty());
}

#[test]
fn retired_window_completion_cannot_cancel_new_request_for_same_unit() {
    let _serial = crate::surface_javascript::fixture::SERIAL.lock().unwrap();
    let (notify, ready) = mpsc::channel();
    let mut owner = Owner::new(Arc::new(move || {
        let _ = notify.send(());
    }));
    owner.tasks.reset_workspace("workspace:1".into());
    let mut device = fixture::owner();
    let mut authority = fixture::authority();
    let current_lease = fixture::lease(&mut device);
    let (release, waiting) = mpsc::channel();
    let token = owner
        .tasks
        .start_description_with(
            fixture::id(),
            description::Request {
                origin: fixture::request().binding.origin,
                workspace: "workspace:1".into(),
                instance: "instance:new".into(),
                art: "vendor/demo".into(),
            },
            current_lease.clone(),
            &mut authority,
            move |_| {
                waiting.recv_timeout(Duration::from_secs(5)).unwrap();
                Err(Error::Worker)
            },
        )
        .unwrap();
    let (runtime, _grant, probe) = crate::surface_javascript::signed::runtime::tests::start(None);
    // probe 到达说明真实旧 child 已经创建；下载请求仍停在独立的确定性门上。
    let probe = probe.recv_timeout(Duration::from_secs(5)).unwrap();
    let state = super::super::State::new(None, false).unwrap();
    owner.entries.push(Entry {
        unit: fixture::id(),
        instance: "instance:old".into(),
        token: Token {
            id: token.id,
            generation: 0,
        },
        stamp: Stamp::capture(&state),
        inputs: dependencies::Stamp::capture(
            &Workspace::with_initial(fixture::id()),
            &fixture::id(),
        )
        .unwrap(),
        result_stamp: state.execution_results.borrow().output_stamp(),
        active: false,
        images: images::Images::default(),
        window: Some(Window::new(
            runtime,
            crate::surface_javascript::fixture::binding(),
        )),
    });
    assert!(owner
        .poll(&[], &mut Workspace::default(), &[], false)
        .is_none());
    assert!(!current_lease.is_revoked());
    if let Some(entry) = owner.entries.first() {
        use std::os::windows::io::AsRawHandle;
        use windows::Win32::{
            Foundation::{HANDLE, WAIT_OBJECT_0},
            System::Threading::WaitForMultipleObjects,
        };
        let handles: Vec<_> = entry
            .window
            .as_ref()
            .unwrap()
            .wait_handles()
            .map(|handle| HANDLE(handle.as_raw_handle()))
            .collect();
        assert_eq!(
            unsafe { WaitForMultipleObjects(&handles, false, 8000) },
            WAIT_OBJECT_0
        );
        assert!(owner
            .poll(&[], &mut Workspace::default(), &[], false)
            .is_none());
    }
    assert!(owner.entries.is_empty());
    assert!(probe.exited());
    assert!(!current_lease.is_revoked());
    assert!(owner.tasks.busy());
    release.send(()).unwrap();
    ready.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(matches!(
        owner.tasks.take_settled().unwrap().result,
        Err(Error::Worker)
    ));
    assert!(current_lease.is_revoked());
}
