//! 当前场景队列、版本重置、无效候选和窗口轮转；使用真实隐藏 HWND。
use super::*;
fn queued(grant: &mut Grant, hex: char) -> AuthorizedWindow {
    let mut result = outcome();
    let raw = image_raw(result.binding.as_ref().unwrap(), 1, "lease:queue");
    let raw = raw
        .get()
        .replace(&"a".repeat(64), &hex.to_string().repeat(64));
    result.result = Ok(serde_json::value::RawValue::from_string(raw).unwrap());
    AuthorizedWindow::new(
        &result,
        grant.lease("https://localhost:1234", 0).unwrap(),
        0,
        [200, 140],
    )
    .unwrap()
}
fn activate(window: &mut AuthorizedWindow) {
    let mut owner = window.begin_activation(0).unwrap().unwrap();
    let state = crate::surface_live_binding::Lifecycle::Active;
    let revision = owner.begin(state).unwrap();
    let binding = owner.binding();
    owner.acknowledge(&serde_json::to_vec(&serde_json::json!({"descriptor":{
        "instanceId":binding.instance(),"attachmentId":binding.attachment(),"hookNodeId":binding.node(),"deviceId":binding.device()},
        "lifecycle":"active","lifecycleRevision":revision})).unwrap(),state,revision).unwrap();
    assert!(window.finish_activation(owner).is_ok());
}
#[test]
fn resource_window_queue_waits_activation_and_consumes_once_per_scene() {
    let _com = Apartment::new();
    let mut grant = grant();
    let mut window = queued(&mut grant, 'a');
    assert!(window.next_image_request(0).unwrap().is_none());
    activate(&mut window);
    assert_eq!(
        window
            .next_image_request(0)
            .unwrap()
            .unwrap()
            .as_resource()
            .unwrap()
            .resource_id(),
        id()
    );
    // 未接纳的请求仍在队首；只有调度成功/终态失败才显式消费。
    assert!(window.next_image_request(0).unwrap().is_some());
    window.finish_image_request();
    assert!(window.next_image_request(0).unwrap().is_none());
    window.show(0).unwrap();
    let mut update = completed(&mut window);
    let raw = image_raw(window.binding(), 2, "lease:new-scene");
    let snapshot = update.convergence.as_mut().unwrap().as_mut().unwrap();
    snapshot.scene = image_scene(&raw);
    snapshot.resources = ResourceSnapshot::new(&raw, window.binding(), 2).unwrap();
    let receipt = window
        .accept_event(update, 0)
        .unwrap_or_else(|_| panic!("event"));
    assert!(matches!(receipt.presentation, Ok(Some(2))));
    assert_eq!(
        window
            .next_image_request(0)
            .unwrap()
            .unwrap()
            .as_resource()
            .unwrap()
            .id(),
        "lease:new-scene"
    );
    window.close().unwrap();
    assert!(window.next_image_request(0).unwrap().is_none());
}
#[test]
fn resource_window_queue_rotates_first_window_and_terminal_failures_do_not_repeat() {
    use crate::desktop::surface_windows::{StartImageError, Windows};
    let _com = Apartment::new();
    let mut grant = grant();
    let mut windows = Windows::default();
    for hex in ['a', 'b'] {
        let mut window = queued(&mut grant, hex);
        activate(&mut window);
        windows
            .insert(window)
            .unwrap_or_else(|_| panic!("capacity"));
    }
    let mut attempts = Vec::new();
    for _ in 0..2 {
        windows.schedule_images(0, |request| {
            attempts.push(request.as_resource().unwrap().resource_id().to_owned());
            Err(StartImageError::Busy)
        });
    }
    assert_eq!(
        attempts,
        vec![
            id(),
            format!("sha256:{}", "b".repeat(64)),
            format!("sha256:{}", "b".repeat(64)),
            id()
        ]
    );
    let mut failures = 0;
    windows.schedule_images(0, |_| {
        failures += 1;
        Err(StartImageError::Failed("fixture failure".into()))
    });
    assert_eq!(failures, 2);
    windows.schedule_images(0, |_| panic!("terminal candidate retried"));
    assert!(windows.status().unwrap().contains("fixture failure"));
    windows.close_all().unwrap();
}
