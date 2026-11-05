//! 测试专属 Application/认证挂载输入；真实授权与 GET 仍走生产后台路径。
use super::*;
use crate::{
    desktop::{document_window::DocumentWindow, surface_authorized_window::AuthorizedWindow},
    surface_live_binding::{Binding, Lifecycle},
};
use std::sync::Arc;
pub(super) struct Class(HINSTANCE);
impl Drop for Class {
    fn drop(&mut self) {
        unsafe {
            let _ = UnregisterClassW(w!("Otter.Native.Workspace"), Some(self.0));
        }
    }
}
pub(super) unsafe fn application() -> (Class, Application) {
    let instance: HINSTANCE = windows::Win32::System::LibraryLoader::GetModuleHandleW(None)
        .unwrap()
        .into();
    assert_ne!(
        RegisterClassW(&WNDCLASSW {
            hInstance: instance,
            lpszClassName: w!("Otter.Native.Workspace"),
            lpfnWndProc: Some(crate::desktop::procedure::window_proc),
            ..Default::default()
        }),
        0
    );
    let document =
        DocumentWindow::create(instance, State::new(None, false).unwrap(), false).unwrap();
    (Class(instance), Application::new(instance, document))
}
pub(super) fn window(
    app: &mut Application,
    origin: &str,
    now: u64,
    ids: &[String],
) -> AuthorizedWindow {
    window_images(
        app,
        origin,
        now,
        ids,
        &vec![4; ids.len()],
        "application/x-neuro-rgba8",
    )
}
pub(super) fn window_images(
    app: &mut Application,
    origin: &str,
    now: u64,
    ids: &[String],
    sizes: &[usize],
    mime: &str,
) -> AuthorizedWindow {
    let children: Vec<_> = ids.iter().enumerate().map(|(i,id)|serde_json::json!({"id":format!("image-{i}"),"type":"image","props":{"resourceId":id},"layout":{"width":32,"height":32}})).collect();
    let leases: Vec<_> = ids.iter().zip(sizes).map(|(id,size)|serde_json::json!({"leaseId":format!("lease:{}", &id[7..]),"resource":{"resourceId":id,"kind":"image","mime":mime,"size":size,"width":1,"height":1},"transport":{"kind":"loom_resource","path":format!("/v1/surfaces/resources/{}",&id[7..])},"expiresAtMs":now+60000})).collect();
    window_with_sources(app, origin, now, children, leases)
}
pub(super) fn window_with_sources(
    app: &mut Application,
    origin: &str,
    now: u64,
    children: Vec<serde_json::Value>,
    leases: Vec<serde_json::Value>,
) -> AuthorizedWindow {
    let binding = Binding::from_mount(br#"{"instance":{"descriptor":{"instanceId":"instance:test","generation":1},"attachments":{"attachment:test":{"descriptor":{"attachmentId":"attachment:test","instanceId":"instance:test","hookNodeId":"node:test","deviceId":"device-old"},"lifecycle":"mounted","lifecycleRevision":1}}}}"#, origin, "device-old", "node:test").unwrap();
    let raw = serde_json::value::to_raw_value(&serde_json::json!({"protocolVersion":"loom.surface.v1","instanceId":"instance:test","attachmentId":"attachment:test","revision":1,"scene":{"id":"root","type":"column","children":children},"resourceLeases":leases})).unwrap();
    let outcome = crate::surface_attach_runtime::Outcome {
        origin: origin.into(),
        device: "device-old".into(),
        binding: Some(binding),
        uncertain: false,
        request: Arc::new(
            crate::surface_attach_runtime::Request::new(
                "art:test".into(),
                "node:test".into(),
                super::super::super::super::surface_start::report().unwrap(),
                crate::live_relay_http::Persistence::Temporary,
            )
            .unwrap(),
        ),
        result: Ok(raw),
    };
    let mut window = AuthorizedWindow::new(
        &outcome,
        app.authorization_owner.lease(origin, now).unwrap(),
        now,
        [200, 140],
    )
    .unwrap();
    let mut owner = window.begin_activation(now).unwrap().unwrap();
    let revision = owner.begin(Lifecycle::Active).unwrap();
    let binding = owner.binding();
    owner.acknowledge(&serde_json::to_vec(&serde_json::json!({"descriptor":{"instanceId":binding.instance(),"attachmentId":binding.attachment(),"hookNodeId":binding.node(),"deviceId":binding.device()},"lifecycle":"active","lifecycleRevision":revision})).unwrap(),Lifecycle::Active,revision).unwrap();
    assert!(window.finish_activation(owner).is_ok());
    window
}
pub(super) fn config(
    origin: &str,
    root: &std::path::Path,
) -> std::result::Result<Config, crate::device_authorization_config::Error> {
    Config::from_lookup(origin, |key| match key {
        "OTTER_NATIVE_PROFILE_DIR" => Some(root.as_os_str().to_owned()),
        "OTTER_LOOM_TLS_ORIGIN" => Some(origin.into()),
        "OTTER_LOOM_TLS_CA_FILE" => Some(root.join("fixture-ca.pem").into_os_string()),
        _ => None,
    })
}
pub(super) fn wait(app: &Application) {
    use std::os::windows::io::AsRawHandle;
    let handles: Vec<_> = app.remote_sessions.wait_handles().collect();
    assert!(!handles.is_empty());
    for handle in handles {
        assert_eq!(
            unsafe {
                windows::Win32::System::Threading::WaitForSingleObject(
                    HANDLE(handle.as_raw_handle()),
                    5000,
                )
            },
            WAIT_OBJECT_0
        );
    }
}
