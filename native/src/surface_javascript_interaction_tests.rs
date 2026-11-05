//! 后台 listener/update 使用同 child；未消费帧、旧快照及收据唤醒不得被越过。
use super::*;
fn viewport() -> crate::surface_viewport::Viewport {
    crate::surface_viewport::Viewport::new([800, 600], 96).unwrap()
}
#[test]
fn javascript_runtime_async_click_update_and_presentation_notification_preserve_child() {
    let _serial = crate::surface_javascript::fixture::SERIAL.lock().unwrap();
    let (_grant, destination, mut mount, mut runtime) = started(false);
    let frame = runtime
        .take_reply(&destination, &mount)
        .unwrap()
        .unwrap()
        .unwrap();
    // Text 也有稳定 identity；从真实投影找按钮，不依赖文档创建计数。
    let target = frame
        .scene
        .root()
        .children()
        .iter()
        .find(|node| node.kind() == crate::surface_scene::Kind::Button)
        .unwrap()
        .id()
        .to_owned();
    let probe = runtime.probe().unwrap();
    activate(&mut mount);
    runtime.rebind(copy(&mount)).unwrap();
    wait_reply(&mut runtime);
    runtime.take_reply(&destination, &mount).unwrap().unwrap();
    runtime.click(copy(&mount), target, viewport()).unwrap();
    assert_eq!(
        runtime.update(copy(&mount), viewport()).unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
    wait_reply(&mut runtime);
    let frame = runtime
        .take_reply(&destination, &mount)
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(frame.events.len(), 1);
    assert!(frame.events[0].is_javascript());
    assert_eq!(frame.events[0].revision(), 3);
    assert_eq!(
        crate::surface_javascript::fixture::text(&frame.scene),
        "原生🙂:42:1"
    );
    mutate(&mut mount, |s| {
        s["revision"] = 4.into();
        s["state"]["quote"] = 45.into();
    });
    runtime.update(copy(&mount), viewport()).unwrap();
    wait_reply(&mut runtime);
    let frame = runtime
        .take_reply(&destination, &mount)
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(
        crate::surface_javascript::fixture::text(&frame.scene),
        "原生🙂:45:1"
    );
    assert!(frame.events.is_empty() && !probe.exited());
    runtime.hold_notification(true).unwrap();
    assert!(runtime.take_reply(&destination, &mount).is_none());
    assert_eq!(
        unsafe {
            WaitForSingleObject(
                HANDLE(runtime.wait_handles().nth(1).unwrap().as_raw_handle()),
                0,
            )
        },
        WAIT_OBJECT_0
    );
    runtime.hold_notification(false).unwrap();
    assert_eq!(
        unsafe {
            WaitForSingleObject(
                HANDLE(runtime.wait_handles().nth(1).unwrap().as_raw_handle()),
                0,
            )
        },
        windows::Win32::Foundation::WAIT_TIMEOUT
    );
    runtime.request_stop();
    finish(&mut runtime);
    assert!(probe.exited());
}
#[test]
fn javascript_runtime_update_reply_rejects_changed_raw_snapshot_and_reaps() {
    let _serial = crate::surface_javascript::fixture::SERIAL.lock().unwrap();
    let (_grant, destination, mut mount, mut runtime) = started(false);
    runtime.take_reply(&destination, &mount).unwrap().unwrap();
    let probe = runtime.probe().unwrap();
    activate(&mut mount);
    mutate(&mut mount, |s| {
        s["revision"] = 4.into();
        s["state"]["quote"] = 45.into();
    });
    runtime.update(copy(&mount), viewport()).unwrap();
    wait_reply(&mut runtime);
    mutate(&mut mount, |s| s["state"]["quote"] = 46.into());
    assert!(runtime.take_reply(&destination, &mount).unwrap().is_err());
    finish(&mut runtime);
    assert!(probe.exited());
}
