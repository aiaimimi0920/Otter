//! Hidden Canvas HWNDs: shared D3D11 selection and sticky WARP recovery after confirmed retirement.
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

fn settle_recovery(canvas: &mut Canvas) {
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
}

#[test]
fn production_canvas_defaults_to_d3d11_and_recovers_to_sticky_warp() {
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
                let surface = &canvas.target.as_ref().unwrap().surface;
                assert!(matches!(surface, RenderSurface::D3d11(_)));
                assert!(!surface.is_d3d11_warp());
            }
            assert_eq!(canvas.paints, 3);
            assert_eq!(canvas.confirmed_paints, 3);
            canvas.discard_device();
            canvas.paint(window.0, 160, 120, 96).unwrap();
            assert_eq!(canvas.confirmed_paints, 4);
            assert!(matches!(
                canvas.target.as_ref().unwrap().surface,
                RenderSurface::D3d11(_)
            ));
            assert!(!canvas.target.as_ref().unwrap().surface.is_d3d11_warp());
            // Business-closure errors are not device failures and must not trigger fallback.
            assert!(canvas
                .target
                .as_mut()
                .unwrap()
                .surface
                .draw(|_| { Err(windows::core::Error::new(E_FAIL, "business fixture")) })
                .is_err());
            assert!(!canvas.recover_render());
            assert!(!canvas.compatibility_renderer);
            // Real D2D EndDraw failure on D3D11 hardware downgrades to sticky D3D11 WARP compatibility.
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
            settle_recovery(&mut canvas);
            assert!(canvas.compatibility_renderer);
            assert!(!canvas.render_suspended());
            assert!(stamp.matches(&canvas.editor));
            assert!(
                !canvas.recover_render(),
                "downgrade cannot repeat without a target"
            );
            canvas.paint(window.0, 160, 120, 96).unwrap();
            assert!(canvas.target.as_ref().unwrap().surface.is_compatibility());
            assert!(canvas.target.as_ref().unwrap().surface.is_d3d11_warp());
            assert_eq!(canvas.confirmed_paints, 5);
            // Ordinary minimize/redraw-style resource disposal must not switch selection back to hardware primary.
            canvas.discard_device();
            canvas.paint(window.0, 160, 120, 96).unwrap();
            assert!(canvas.target.as_ref().unwrap().surface.is_d3d11_warp());
            assert_eq!(canvas.confirmed_paints, 6);
            assert!(stamp.matches(&canvas.editor));
            assert!(!canvas.recover_render());
        }
    }
}
