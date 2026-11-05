//! 真实 HWND 当前资源快照：合成协议，不访问网络；显示场景与资源版本同一提交。
use super::*;
#[path = "surface_window_image_request_tests.rs"]
mod requests;
use crate::{
    desktop::surface_authorized_window::ImageError,
    surface_resource_snapshot::{
        tests::{id, raw},
        ResourceSnapshot,
    },
};
#[test]
fn resource_window_installs_ready_and_removes_expired_images_without_closing_window() {
    let _com = Apartment::new();
    let mut grant = grant();
    let mut result = outcome();
    let mut snapshot: serde_json::Value =
        serde_json::from_str(image_raw(result.binding.as_ref().unwrap(), 1, "lease:install").get())
            .unwrap();
    snapshot["resourceLeases"][0]["expiresAtMs"] = 50.into();
    result.result = Ok(serde_json::value::to_raw_value(&snapshot).unwrap());
    let mut window = AuthorizedWindow::new(
        &result,
        grant.lease("https://localhost:1234", 0).unwrap(),
        0,
        [200, 140],
    )
    .unwrap();
    let request = window.image_resource(&id(), 0).unwrap();
    let pixels: crate::pixels::Pixels = vec![255; 4].into();
    let weak = pixels.downgrade();
    let ready = crate::surface_resource_runtime::Ready::from_test_image(
        grant.lease("https://localhost:1234", 0).unwrap(),
        request,
        crate::image_decode::Image {
            width: 1,
            height: 1,
            pixels,
        },
    );
    window.install_resource(ready, 0).unwrap();
    assert_eq!(window.installed_images_for_test(), 1);
    assert_eq!(window.expires_at_ms(), 50);
    unsafe {
        windows::Win32::UI::WindowsAndMessaging::SendMessageW(
            window.edit_test_parent(),
            windows::Win32::UI::WindowsAndMessaging::WM_PAINT,
            None,
            None,
        );
    }
    assert!(window.take_error().is_none());
    assert_eq!(window.uploaded_images_for_test(), 1);
    assert!(weak.upgrade().is_some());
    window.refresh(50).unwrap();
    assert!(window.is_alive());
    assert_eq!(window.installed_images_for_test(), 0);
    assert!(weak.upgrade().is_none());
    window.close().unwrap();
}
pub(super) fn image_raw(
    binding: &crate::surface_live_binding::Binding,
    revision: u64,
    lease: &str,
) -> Box<serde_json::value::RawValue> {
    let mut value: serde_json::Value =
        serde_json::from_str(raw(binding, revision, lease).get()).unwrap();
    value["scene"] = serde_json::json!({"id":"root","type":"column","children":[
        {"id":"pending","type":"image","props":{"resourceId":id()}},
        {"id":"button","type":"button","props":{"label":"Available"},"events":{"click":"submit"}}
    ]});
    serde_json::value::to_raw_value(&value).unwrap()
}
pub(super) fn image_scene(raw: &serde_json::value::RawValue) -> Scene {
    let value: serde_json::Value = serde_json::from_str(raw.get()).unwrap();
    Scene::parse(&serde_json::to_vec(&value["scene"]).unwrap()).unwrap()
}
#[test]
fn resource_window_unresolved_root_image_keeps_owner_without_pixel_or_network() {
    let _com = Apartment::new();
    let mut grant = grant();
    let mut result = outcome();
    let mut value: serde_json::Value =
        serde_json::from_str(raw(result.binding.as_ref().unwrap(), 1, "lease:root").get()).unwrap();
    value["scene"] = serde_json::json!({"id":"root","type":"image","props":{"resourceId":id()}});
    result.result = Ok(serde_json::value::to_raw_value(&value).unwrap());
    let mut window = AuthorizedWindow::new(
        &result,
        grant.lease("https://localhost:1234", 0).unwrap(),
        0,
        [200, 140],
    )
    .unwrap();
    assert!(window.is_alive());
    assert_eq!(window.image_resource(&id(), 0).unwrap().id(), "lease:root");
    window.close().unwrap();
}
#[test]
fn resource_window_initial_snapshot_tracks_activation_and_close() {
    let _com = Apartment::new();
    let mut grant = grant();
    let mut result = outcome();
    result.result = Ok(image_raw(
        result.binding.as_ref().unwrap(),
        1,
        "lease:initial",
    ));
    let mut window = AuthorizedWindow::new(
        &result,
        grant.lease("https://localhost:1234", 0).unwrap(),
        0,
        [200, 140],
    )
    .unwrap();
    assert_eq!(
        window.image_resource(&id(), 0).unwrap().id(),
        "lease:initial"
    );
    let mut owner = window.begin_activation(0).unwrap().unwrap();
    let state = crate::surface_live_binding::Lifecycle::Active;
    let revision = owner.begin(state).unwrap();
    let binding = owner.binding();
    owner.acknowledge(&serde_json::to_vec(&serde_json::json!({"descriptor":{
        "instanceId":binding.instance(),"attachmentId":binding.attachment(),"hookNodeId":binding.node(),"deviceId":binding.device()},
        "lifecycle":"active","lifecycleRevision":revision})).unwrap(),state,revision).unwrap();
    assert!(window.finish_activation(owner).is_ok());
    window.show(0).unwrap();
    let request = window.image_resource(&id(), 0).unwrap();
    assert_eq!(request.binding(), window.binding());
    assert_eq!(request.id(), "lease:initial");
    window.close().unwrap();
    assert!(matches!(
        window.image_resource(&id(), 0),
        Err(ImageError::Inactive)
    ));
}
#[test]
fn resource_window_event_commits_new_snapshot_and_rejects_stale_candidate() {
    let _com = Apartment::new();
    let mut grant = grant();
    let mut window = active_window(&mut grant);
    let mut update = completed(&mut window);
    let raw = image_raw(window.binding(), 2, "lease:event");
    let snapshot = update.convergence.as_mut().unwrap().as_mut().unwrap();
    snapshot.scene = image_scene(&raw);
    snapshot.resources = ResourceSnapshot::new(&raw, window.binding(), 2).unwrap();
    let receipt = window
        .accept_event(update, 0)
        .unwrap_or_else(|_| panic!("event owner"));
    assert!(matches!(receipt.presentation, Ok(Some(2))));
    assert_eq!(window.image_resource(&id(), 0).unwrap().id(), "lease:event");
    let mut stale = completed(&mut window);
    stale
        .convergence
        .as_mut()
        .unwrap()
        .as_mut()
        .unwrap()
        .resources = ResourceSnapshot::new(
        &image_raw(window.binding(), 1, "lease:old"),
        window.binding(),
        1,
    )
    .unwrap();
    let receipt = window
        .accept_event(stale, 0)
        .unwrap_or_else(|_| panic!("stale owner"));
    assert!(matches!(
        receipt.presentation,
        Err(PresentationError::Evidence)
    ));
    assert_eq!(window.image_resource(&id(), 0).unwrap().id(), "lease:event");
    drop(grant);
    assert!(matches!(
        window.image_resource(&id(), 0),
        Err(ImageError::Inactive)
    ));
    assert!(!window.is_alive());
}
