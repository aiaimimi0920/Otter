//! 生产 Canvas 使用真实隐藏窗口/D3D12；不以独立候选绘制代替 Canvas 接线。
use super::*;
use crate::desktop::{Apartment, Window};
use windows::{
    core::w,
    Win32::{Foundation::*, System::LibraryLoader::GetModuleHandleW, UI::WindowsAndMessaging::*},
};

unsafe extern "system" fn procedure(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    DefWindowProcW(hwnd, msg, wp, lp)
}
struct Class(HINSTANCE);
impl Drop for Class {
    fn drop(&mut self) {
        unsafe {
            let _ = UnregisterClassW(w!("Otter.Canvas.D3D12.Test"), Some(self.0));
        }
    }
}

#[test]
fn production_canvas_selects_d3d12_and_recovers_to_sticky_compatibility() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let instance: HINSTANCE = GetModuleHandleW(None).unwrap().into();
        assert_ne!(
            RegisterClassW(&WNDCLASSW {
                hInstance: instance,
                lpszClassName: w!("Otter.Canvas.D3D12.Test"),
                lpfnWndProc: Some(procedure),
                ..Default::default()
            }),
            0
        );
        let _class = Class(instance);
        for pinned in [false, true] {
            let window = Window(
                CreateWindowExW(
                    if pinned {
                        WS_EX_NOREDIRECTIONBITMAP
                    } else {
                        WINDOW_EX_STYLE::default()
                    },
                    w!("Otter.Canvas.D3D12.Test"),
                    w!(""),
                    if pinned {
                        WS_POPUP
                    } else {
                        WS_OVERLAPPEDWINDOW
                    },
                    0,
                    0,
                    160,
                    120,
                    None,
                    None,
                    Some(instance),
                    None,
                )
                .unwrap(),
            );
            let mut canvas = Canvas::new(Some(Image {
                width: 8,
                height: 4,
                pixels: [30, 60, 90, 255].repeat(32).into(),
            }))
            .unwrap();
            canvas.pinned = pinned;
            canvas.window_opacity = if pinned { 0.5 } else { 1.0 };
            let stamp = canvas.editor.document_stamp();
            for (width, height, dpi) in [(160, 120, 96), (192, 144, 144), (160, 120, 96)] {
                canvas.paint(window.0, width, height, dpi).unwrap();
                assert!(matches!(
                    canvas.target.as_ref().unwrap().surface,
                    RenderSurface::D3d12(_)
                ));
            }
            assert_eq!(canvas.paints, 3);
            assert_eq!(canvas.confirmed_paints, 3);
            canvas.discard_device();
            assert!(canvas.target.is_none());
            assert!(stamp.matches(&canvas.editor));
            canvas.paint(window.0, 160, 120, 96).unwrap();
            assert!(matches!(
                canvas.target.as_ref().unwrap().surface,
                RenderSurface::D3d12(_)
            ));
            assert_eq!(canvas.confirmed_paints, 4);
            assert!(stamp.matches(&canvas.editor));
            // 业务闭包错误不是设备故障，不触发降级。
            assert!(canvas
                .target
                .as_mut()
                .unwrap()
                .surface
                .draw(|_| { Err(windows::core::Error::new(E_FAIL, "business fixture")) })
                .is_err());
            assert!(!canvas.recover_render());
            assert!(!canvas.compatibility_renderer);
            // 真实 D2D EndDraw 错误：无匹配 Push 的 Pop 令后端进入重建状态。
            assert!(canvas
                .target
                .as_mut()
                .unwrap()
                .surface
                .draw(|target| {
                    target.PopAxisAlignedClip();
                    Ok(())
                })
                .is_err());
            assert!(canvas.recover_render());
            assert!(canvas.target.is_none());
            assert!(!canvas.compatibility_renderer);
            assert!(canvas.render_suspended());
            assert!(canvas.paint(window.0, 160, 120, 96).is_err());
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
            loop {
                match canvas.poll_render_recovery().unwrap() {
                    crate::desktop::composition_retirement::Status::Complete => break,
                    crate::desktop::composition_retirement::Status::Pending => {
                        assert!(std::time::Instant::now() < deadline);
                        std::thread::sleep(std::time::Duration::from_millis(1));
                    }
                    status => panic!("retirement failed: {status:?}"),
                }
            }
            assert!(canvas.compatibility_renderer);
            assert!(!canvas.render_suspended());
            assert!(stamp.matches(&canvas.editor));
            assert!(
                !canvas.recover_render(),
                "downgrade cannot repeat without a target"
            );
            canvas.paint(window.0, 160, 120, 96).unwrap();
            assert!(canvas.target.as_ref().unwrap().surface.is_compatibility());
            assert_eq!(canvas.confirmed_paints, 5);
            // 普通最小化/重绘式资源丢弃不能把选择切回 D3D12。
            canvas.discard_device();
            canvas.paint(window.0, 160, 120, 96).unwrap();
            assert!(canvas.target.as_ref().unwrap().surface.is_compatibility());
            assert_eq!(canvas.confirmed_paints, 6);
            assert!(stamp.matches(&canvas.editor));
            assert!(!canvas.recover_render());
        }
    }
}
