//! 隐藏 HWND + WARP 合成帧贯通生产系统菜单、后台读回及同 ID 静态发布。
use super::*;
#[path = "application_live_add_art_tests.rs"]
mod add_art;
#[path = "application_live_checkpoint_tests.rs"]
mod checkpoint;
#[path = "application_cpu_save_tests.rs"]
mod cpu_save;
#[path = "application_live_display_tests.rs"]
mod display;
#[path = "application_live_execution_tests.rs"]
mod execution;
#[path = "application_live_group_name_tests.rs"]
mod group_name;
#[path = "application_live_groups_tests.rs"]
mod groups;
#[path = "application_live_link_tests.rs"]
mod links;
#[path = "application_live_ports_tests.rs"]
mod ports;
#[path = "application_live_recycle_tests.rs"]
mod recycle;
#[path = "application_live_refresh_tests.rs"]
mod refresh;
#[path = "application_save_live_tests.rs"]
mod save;
#[path = "application_live_unlink_tests.rs"]
mod unlink;
#[path = "application_live_workspace_tests.rs"]
mod workspace;
use std::{
    sync::{
        atomic::{AtomicBool, AtomicUsize},
        Arc,
    },
    time::{Duration, Instant},
};
use windows::core::Interface;
use windows::Win32::Graphics::Gdi::ClientToScreen;
use windows::Win32::Graphics::{Direct3D::D3D_DRIVER_TYPE_WARP, Direct3D11::*, Dxgi::Common::*};
use windows::Win32::UI::HiDpi::{
    SetThreadDpiAwarenessContext, DPI_AWARENESS_CONTEXT, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
};
struct Environment {
    instance: HINSTANCE,
    previous_dpi: DPI_AWARENESS_CONTEXT,
    _apartment: crate::desktop::Apartment,
    _serial: std::sync::MutexGuard<'static, ()>,
}
impl Environment {
    unsafe fn new() -> Self {
        let serial = crate::loom_bridge_client::tests::execution_job::SERIAL
            .lock()
            .unwrap();
        let apartment = crate::desktop::Apartment::new().unwrap();
        let previous_dpi = SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        assert!(!previous_dpi.0.is_null());
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
        Self {
            instance,
            previous_dpi,
            _apartment: apartment,
            _serial: serial,
        }
    }
    unsafe fn app(&self) -> Application {
        Application::new(
            self.instance,
            DocumentWindow::create(self.instance, State::new(None, false).unwrap(), false).unwrap(),
        )
    }
}
impl Drop for Environment {
    fn drop(&mut self) {
        unsafe {
            let _ = UnregisterClassW(w!("Otter.Native.Workspace"), Some(self.instance));
            let _ = SetThreadDpiAwarenessContext(self.previous_dpi);
        }
    }
}
unsafe fn texture() -> crate::live_texture::Texture {
    texture_with_color([0, 0, 255, 255])
}
unsafe fn texture_with_color(color: [u8; 4]) -> crate::live_texture::Texture {
    let (mut device, mut context) = (None, None);
    D3D11CreateDevice(
        None,
        D3D_DRIVER_TYPE_WARP,
        HMODULE::default(),
        D3D11_CREATE_DEVICE_BGRA_SUPPORT,
        None,
        D3D11_SDK_VERSION,
        Some(&mut device),
        None,
        Some(&mut context),
    )
    .unwrap();
    let context = context.unwrap();
    let protection: ID3D11Multithread = context.cast().unwrap();
    let _ = protection.SetMultithreadProtected(true);
    let desc = D3D11_TEXTURE2D_DESC {
        Width: 2,
        Height: 2,
        MipLevels: 1,
        ArraySize: 1,
        Format: DXGI_FORMAT_B8G8R8A8_UNORM,
        SampleDesc: DXGI_SAMPLE_DESC {
            Count: 1,
            Quality: 0,
        },
        Usage: D3D11_USAGE_DEFAULT,
        ..Default::default()
    };
    let pixels = color.repeat(4);
    let initial = D3D11_SUBRESOURCE_DATA {
        pSysMem: pixels.as_ptr().cast(),
        SysMemPitch: 8,
        SysMemSlicePitch: 16,
    };
    let mut texture = None;
    device
        .unwrap()
        .CreateTexture2D(&desc, Some(&initial), Some(&mut texture))
        .unwrap();
    crate::live_texture::snapshot(&context, &texture.unwrap(), &AtomicBool::new(false)).unwrap()
}
unsafe fn live_session(with_frame: bool) -> (HWND, Session) {
    use crate::desktop::composition::live::window::LiveWindow;
    let frame = with_frame.then(|| texture());
    let (ready, wait) = std::sync::mpsc::channel();
    let (job, window) = crate::live_job::LiveJob::spawn_bound(
        &Arc::new(AtomicUsize::new(0)),
        |consumer| LiveWindow::new(consumer, 64, 64).map_err(std::io::Error::other),
        move |source, _| {
            let (closed, stop) = std::sync::mpsc::channel();
            source
                .on_idle(Arc::new(move || {
                    let _ = closed.send(());
                }))
                .unwrap();
            if let Some(frame) = frame {
                source.publish(frame).unwrap();
            }
            ready.send(()).unwrap();
            stop.recv_timeout(Duration::from_secs(10)).unwrap();
            Ok(())
        },
    )
    .unwrap();
    let hwnd = window.hwnd_for_test();
    let session = Session::new(job, window);
    wait.recv_timeout(Duration::from_secs(2)).unwrap();
    (hwnd, session)
}
pub(in crate::desktop::application) unsafe fn live(
    app: &mut Application,
    with_frame: bool,
) -> (HWND, crate::workspace_id::UnitId) {
    let (hwnd, session) = live_session(with_frame);
    let id = session.unit_id().clone();
    app.admit_live(session, false).unwrap();
    (hwnd, id)
}
unsafe fn queue(hwnd: HWND) {
    SendMessageW(hwnd, WM_SYSCOMMAND, Some(WPARAM(0x1fe0)), Some(LPARAM(0)));
}
unsafe fn settle(app: &mut Application) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(5);
    while app.live_freeze_job.is_some() {
        app.process_live_freeze(false)?;
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(1));
    }
    Ok(())
}
#[test]
fn freeze_uses_commit_time_image_region_after_live_move_and_resize() {
    unsafe {
        let environment = Environment::new();
        let mut app = environment.app();
        let (hwnd, id) = live(&mut app, true);
        queue(hwnd);
        app.process_live_freeze(false).unwrap();
        // 读回在后台进行；最终位置以提交时客户区为准，而非排队时的旧位置。
        SetWindowPos(
            hwnd,
            None,
            -240,
            160,
            420,
            260,
            SWP_NOACTIVATE | SWP_NOZORDER,
        )
        .unwrap();
        let mut client = RECT::default();
        GetClientRect(hwnd, &mut client).unwrap();
        let mut origin = POINT::default();
        assert!(ClientToScreen(hwnd, &mut origin).as_bool());
        let side = (client.right - client.left).min(client.bottom - client.top);
        let expected_x = origin.x + ((client.right - side) as f64 / 2.0).round() as i32;
        let expected_y = origin.y + ((client.bottom - side) as f64 / 2.0).round() as i32;
        crate::desktop::composition::live::window::local::tests::wheel(
            hwnd,
            &[windows::Win32::UI::Input::KeyboardAndMouse::VK_MENU],
            -650,
            (0, 0),
        );
        settle(&mut app).unwrap();
        let document = app
            .documents
            .iter()
            .find(|document| document.state.unit_id == id)
            .unwrap();
        let mut actual = RECT::default();
        GetWindowRect(document.window.0, &mut actual).unwrap();
        assert_eq!(
            [actual.left, actual.top, actual.right, actual.bottom],
            [expected_x, expected_y, expected_x + side, expected_y + side]
        );
        let canvas = document.state.canvas.borrow();
        assert!((canvas.normal_opacity - 0.35).abs() < 0.00001);
        assert!((canvas.window_opacity - 0.35).abs() < 0.00001);
        assert_eq!(canvas.minified_opacity, 0.9);
        let image = canvas.image.as_ref().unwrap();
        assert_eq!([image.width, image.height], [2, 2]);
        assert_eq!(&*image.pixels, &[0, 0, 255, 255].repeat(4));
        drop(canvas);
        assert!(!IsWindow(Some(hwnd)).as_bool());
        app.shutdown_live().unwrap();
    }
}

#[test]
fn empty_live_client_rejects_freeze_without_stopping_source() {
    unsafe {
        let environment = Environment::new();
        let mut app = environment.app();
        let (hwnd, _) = live(&mut app, true);
        queue(hwnd);
        app.process_live_freeze(false).unwrap();
        SetWindowPos(
            hwnd,
            None,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOACTIVATE | SWP_NOZORDER,
        )
        .unwrap();
        assert!(settle(&mut app).is_err());
        assert_eq!(app.documents.len(), 1);
        assert!(app.live_sessions[0].is_alive());
        assert!(IsWindow(Some(hwnd)).as_bool());
        app.shutdown_live().unwrap();
    }
}
#[test]
fn menu_freezes_current_pixels_with_same_identity_and_stops_only_after_publish() {
    unsafe {
        let environment = Environment::new();
        let mut app = environment.app();
        let (hwnd, id) = live(&mut app, true);
        // 与最后静态文档回收的空宿主替换使用同一实例身份路径。
        app.workspace = app
            .workspace
            .replacement_unit(
                &app.documents[0].state.unit_id,
                crate::desktop::document_state::new_unit_id().unwrap(),
            )
            .unwrap();
        app.documents[0].state.unit_id = app
            .workspace
            .units()
            .iter()
            .find(|unit| **unit != id)
            .unwrap()
            .clone();
        queue(hwnd);
        queue(hwnd);
        app.process_live_freeze(false).unwrap();
        assert!(app.live_freeze_job.is_some());
        assert!(app.live_sessions[0].is_alive());
        assert_eq!(app.documents.len(), 1);
        settle(&mut app).unwrap();
        assert_eq!(app.documents.len(), 2);
        assert_eq!(app.documents[1].state.unit_id, id);
        assert!(app.workspace.contains(&id));
        assert_eq!(
            &*app.documents[1]
                .state
                .canvas
                .borrow()
                .image
                .as_ref()
                .unwrap()
                .pixels,
            &[0, 0, 255, 255].repeat(4)
        );
        assert!(!app.live_sessions[0].is_alive());
        assert!(!IsWindow(Some(hwnd)).as_bool());
        app.shutdown_live().unwrap();
        assert!(app.live_sessions.is_empty());
    }
}
#[test]
fn workspace_replacement_before_request_rejects_old_live_and_failed_admission_keeps_it_alive() {
    unsafe {
        let environment = Environment::new();
        for replaced in [false, true] {
            let mut app = environment.app();
            let (hwnd, id) = live(&mut app, true);
            if replaced {
                app.workspace = crate::workspace::Workspace::with_initial(
                    app.documents[0].state.unit_id.clone(),
                );
                queue(hwnd);
                assert!(app.process_live_freeze(false).is_err());
            } else {
                // 重复静态 owner 不得覆盖 Live 的已登记身份。
                app.documents[0].state.unit_id = id;
                queue(hwnd);
                app.process_live_freeze(false).unwrap();
                assert!(settle(&mut app).is_err());
            }
            assert!(app.live_sessions[0].is_alive());
            assert!(IsWindow(Some(hwnd)).as_bool());
            assert_eq!(app.documents.len(), 1);
            assert!(app.live_freeze_job.is_none());
            app.shutdown_live().unwrap();
        }
    }
}
#[test]
fn no_frame_closed_origin_and_changed_workspace_never_publish_stale_pixels() {
    unsafe {
        let environment = Environment::new();
        for mode in 0..3 {
            let mut app = environment.app();
            let (hwnd, _) = live(&mut app, mode != 0);
            queue(hwnd);
            if mode == 0 {
                assert!(app.process_live_freeze(false).is_err());
                assert!(app.live_sessions[0].is_alive());
            } else {
                app.process_live_freeze(false).unwrap();
                if mode == 1 {
                    DestroyWindow(hwnd).unwrap();
                } else {
                    app.restored_session.as_mut().unwrap().original =
                        Vec::from(b"{}".as_slice()).into();
                }
                assert!(settle(&mut app).is_err());
                if mode == 2 {
                    assert!(app.live_sessions[0].is_alive());
                }
            }
            assert_eq!(app.documents.len(), 1);
            assert!(app.live_freeze_job.is_none());
            app.shutdown_live().unwrap();
        }
    }
}
