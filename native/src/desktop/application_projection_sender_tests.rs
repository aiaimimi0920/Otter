//! 隐藏原生发送测试的隔离 owner/工作区/TLS 夹具；不注册全局输入，不使用真实 profile。
use super::*;
use crate::{
    desktop::{device_authorization_ui as ui, image::Image, workspace_profile::Profile, Apartment},
    device_authorization_config::Config,
    device_authorization_runtime::Task,
    device_session_client::tests::{accept, fixture_with_pem, pairing::Files, reply, request},
};
use rustls::{ServerConnection, StreamOwned};
use std::{
    os::windows::io::AsRawHandle,
    sync::atomic::AtomicBool,
    time::{Duration, Instant},
};
use windows::Win32::System::{LibraryLoader::GetModuleHandleW, Threading::WaitForSingleObject};
#[path = "application_projection_edit_source_tests.rs"]
mod collaboration;
#[path = "application_projection_sender_geometry_tests.rs"]
mod geometry;
#[path = "application_projection_sender_group_tests.rs"]
mod groups;
#[path = "application_projection_sender_guard_tests.rs"]
mod guard;
#[path = "application_projection_sender_joint_tests.rs"]
mod joint;
#[path = "application_projection_sender_offline_tests.rs"]
mod offline;
#[path = "application_projection_sender_source_tests.rs"]
mod source;
#[path = "application_projection_sender_stop_tests.rs"]
mod stop;
#[path = "application_projection_sender_target_tests.rs"]
mod target;
#[path = "application_projection_sender_tls_tests.rs"]
mod tls;
#[path = "application_projection_sender_continuous_tests.rs"]
mod updates;
struct Class(HINSTANCE);
impl Drop for Class {
    fn drop(&mut self) {
        unsafe {
            drain_messages();
            let _ = UnregisterClassW(w!("Otter.Native.Workspace"), Some(self.0));
        }
    }
}
unsafe fn drain_messages() {
    // 同线程测试重开是新实例；沿现有 checkpoint 夹具排空上一实例的 WM_QUIT，不改生产 GetMessage。
    let mut msg = MSG::default();
    for _ in 0..1024 {
        if !PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
            return;
        }
        if msg.message != WM_QUIT {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
    panic!("sender test message drain exceeded existing smoke bound");
}
unsafe fn application(root: &std::path::Path) -> (Class, Application) {
    let instance: HINSTANCE = GetModuleHandleW(None).unwrap().into();
    assert_ne!(
        RegisterClassW(&WNDCLASSW {
            hInstance: instance,
            lpszClassName: w!("Otter.Native.Workspace"),
            lpfnWndProc: Some(crate::desktop::procedure::window_proc),
            ..Default::default()
        }),
        0
    );
    let image = Image {
        width: 32,
        height: 32,
        pixels: [255, 0, 0, 255].repeat(32 * 32).into(),
    };
    let document =
        DocumentWindow::create(instance, State::new(Some(image), true).unwrap(), false).unwrap();
    let mut app = Application::new(instance, document);
    app.install_workspace_checkpoint(Some(Profile::open(root).unwrap()), true)
        .unwrap();
    app.projection_sender
        .start(root.to_path_buf(), app.execution_wake.callback());
    app.projection_sender.profile.collect(true);
    assert!(app.projection_sender.profile.entries().is_some());
    settle(&mut app);
    assert!(app.projection_sender_scope().is_some());
    (Class(instance), app)
}
unsafe fn settle(app: &mut Application) {
    let end = Instant::now() + Duration::from_secs(10);
    loop {
        app.pending_workspace_save_with(|_, _, message, warning| assert!(!warning, "{message}"));
        if app.save_job.is_none() {
            break;
        }
        assert!(Instant::now() < end, "sender checkpoint deadline");
        std::thread::sleep(Duration::from_millis(1));
    }
}
fn config(
    origin: &str,
    root: &std::path::Path,
) -> std::result::Result<Config, crate::device_authorization_config::Error> {
    Config::from_lookup(origin, |key| match key {
        "OTTER_NATIVE_PROFILE_DIR" => Some(root.as_os_str().to_owned()),
        "OTTER_LOOM_TLS_ORIGIN" => Some(origin.into()),
        "OTTER_LOOM_TLS_CA_FILE" => Some(root.join("ca.pem").into_os_string()),
        _ => None,
    })
}
fn wait_remote(app: &Application) {
    let handle = app.remote_sessions.wait_handles().next().unwrap();
    assert_eq!(
        unsafe { WaitForSingleObject(HANDLE(handle.as_raw_handle()), 5000) },
        WAIT_OBJECT_0
    );
}
fn serial() -> std::sync::MutexGuard<'static, ()> {
    crate::loom_bridge_client::tests::execution_job::SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}
