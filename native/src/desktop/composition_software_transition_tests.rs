//! Explicit interactive gate: owned top-level color blocks without activation/input; sample only desktop-composited pixels at block centers.
use crate::desktop::{
    composition_d3d12::CompositionD3d12,
    composition_retirement::Status as RetirementStatus,
    render_surface::RenderSurface,
    software_surface::{window_style::Transition, SoftwareSurface},
    Apartment, Window,
};
use windows::{
    core::w,
    Win32::{
        Foundation::*,
        Graphics::{
            Direct2D::{Common::*, *},
            Dwm::DwmFlush,
            Gdi::*,
        },
        System::LibraryLoader::GetModuleHandleW,
        UI::{HiDpi::*, WindowsAndMessaging::*},
    },
};

#[path = "composition_workspace_transition_tests.rs"]
mod workspace;

struct Class(HINSTANCE);
struct DpiScope(DPI_AWARENESS_CONTEXT);
impl Drop for DpiScope {
    fn drop(&mut self) {
        unsafe {
            SetThreadDpiAwarenessContext(self.0);
        }
    }
}
unsafe extern "system" fn procedure(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    DefWindowProcW(hwnd, msg, wp, lp)
}
impl Drop for Class {
    fn drop(&mut self) {
        unsafe {
            let _ = UnregisterClassW(w!("Otter.Software.Transition.Test"), Some(self.0));
            // After registration succeeds, the window class owns the background brush; the system releases it on UnregisterClass, so never DeleteObject again.
        }
    }
}

unsafe fn window(instance: HINSTANCE, no_redirection: bool, x: i32, size: i32) -> Window {
    Window(
        CreateWindowExW(
            WS_EX_TOOLWINDOW
                | WS_EX_TOPMOST
                | WS_EX_NOACTIVATE
                | if no_redirection {
                    WS_EX_NOREDIRECTIONBITMAP
                } else {
                    WINDOW_EX_STYLE::default()
                },
            w!("Otter.Software.Transition.Test"),
            w!("Otter renderer test"),
            WS_POPUP,
            x,
            x,
            size,
            size,
            None,
            None,
            Some(instance),
            None,
        )
        .unwrap(),
    )
}

unsafe fn show(hwnd: HWND) {
    let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
    // Explicitly maintain owned color-block stacking without activation; do not mistake background occlusion for rendering failure.
    SetWindowPos(
        hwnd,
        Some(HWND_TOPMOST),
        0,
        0,
        0,
        0,
        SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
    )
    .unwrap();
    UpdateWindow(hwnd).ok().unwrap();
}

unsafe fn pixel(hwnd: HWND, offset: i32) -> COLORREF {
    let mut point = POINT {
        x: offset,
        y: offset,
    };
    ClientToScreen(hwnd, &mut point).ok().unwrap();
    let dc = GetDC(None);
    assert!(!dc.is_invalid());
    let value = GetPixel(dc, point.x, point.y);
    ReleaseDC(None, dc);
    value
}

unsafe fn expect_pixel(hwnd: HWND, offset: i32, expected: [u8; 3]) {
    let mut last = COLORREF(0xffff_ffff);
    for _ in 0..30 {
        let mut message = MSG::default();
        for _ in 0..64 {
            if !PeekMessageW(&mut message, Some(hwnd), 0, 0, PM_REMOVE).as_bool() {
                break;
            }
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
        DwmFlush().unwrap();
        last = pixel(hwnd, offset);
        let channels = [last.0 as u8, (last.0 >> 8) as u8, (last.0 >> 16) as u8];
        if last.0 != 0xffff_ffff
            && channels
                .into_iter()
                .zip(expected)
                .all(|(a, b)| a.abs_diff(b) <= 4)
        {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    panic!(
        "desktop transition pixel mismatch: actual={:08x}, expected={expected:?}",
        last.0
    );
}

#[test]
#[ignore = "brief visible topmost test windows; run explicitly with desktop access"]
fn same_hwnd_d3d12_to_software_preserves_no_redirection_and_composes_alpha() {
    verify(false);
}

#[test]
#[ignore = "brief visible topmost test windows; run explicitly with desktop access"]
fn same_hwnd_d3d11_to_software_preserves_no_redirection_and_composes_alpha() {
    verify(true);
}

fn compatibility(hwnd: HWND, size: D2D_SIZE_U) -> RenderSurface {
    let factory: ID2D1Factory =
        unsafe { D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None) }.unwrap();
    // Use the real compatibility-selection entry of ordinary workspaces rather than directly selecting a composition implementation in tests.
    let surface = RenderSurface::legacy(&factory, hwnd, size, 96, false).unwrap();
    assert!(matches!(surface, RenderSurface::D3d11(_)));
    surface
}

fn verify(d3d11: bool) {
    let _serial = crate::desktop::composition_retirement::serial_for_test();
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let previous = SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        assert!(!previous.0.is_null());
        let _dpi = DpiScope(previous);
        let instance: HINSTANCE = GetModuleHandleW(None).unwrap().into();
        let brush = CreateSolidBrush(COLORREF(0x00ff0000)); // Blue background.
        assert!(!brush.is_invalid());
        if RegisterClassW(&WNDCLASSW {
            hInstance: instance,
            lpszClassName: w!("Otter.Software.Transition.Test"),
            lpfnWndProc: Some(procedure),
            hbrBackground: brush,
            ..Default::default()
        }) == 0
        {
            let _ = DeleteObject(brush.into()); // Registration failed, so the system has not taken ownership of the brush.
            panic!("transition window class registration failed");
        }
        let _class = Class(instance);
        let backdrop = window(instance, false, 32, 128);
        show(backdrop.0);
        let front = window(instance, true, 48, 64);
        let size = D2D_SIZE_U {
            width: 64,
            height: 64,
        };
        let mut gpu = if d3d11 {
            compatibility(front.0, size)
        } else {
            RenderSurface::D3d12(Box::new(CompositionD3d12::new(front.0, size).unwrap()))
        };
        assert!(gpu
            .draw(|target| {
                target.Clear(Some(&D2D1_COLOR_F {
                    r: 1.0,
                    g: 0.0,
                    b: 0.0,
                    a: 1.0,
                }));
                Ok(())
            })
            .unwrap());
        show(front.0);
        expect_pixel(front.0, 16, [255, 0, 0]);
        // Use the product owner's retirement entry point; the test thread continues processing owned-window messages instead of waiting directly for DComp.
        // Retain the GPU owner until software-pixel verification finishes; completion evidence does not mean GPU release.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        let mut retirement = gpu.retire_composition().expect("GPU composition receipt");
        assert!(gpu.draw(|_| panic!("retired GPU drew again")).is_err());
        assert!(gpu.resize(size).is_err());
        assert!(gpu.set_opacity(0.5).is_err());
        while retirement == RetirementStatus::Pending {
            assert!(
                std::time::Instant::now() < deadline,
                "retirement did not terminate"
            );
            let mut message = MSG::default();
            for _ in 0..64 {
                if !PeekMessageW(&mut message, Some(front.0), 0, 0, PM_REMOVE).as_bool() {
                    break;
                }
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
            assert!(IsWindow(Some(front.0)).as_bool());
            std::thread::sleep(std::time::Duration::from_millis(2));
            retirement = gpu.retire_composition().expect("GPU composition receipt");
        }
        assert_eq!(retirement, RetirementStatus::Complete);
        let factory: ID2D1Factory =
            D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None).unwrap();
        let mut cpu = SoftwareSurface::new(&factory, size).unwrap();
        let transition = Transition::begin(front.0, true).unwrap();
        let paint = |target: &ID2D1RenderTarget| {
            target.Clear(Some(&D2D1_COLOR_F {
                r: 0.0,
                g: 1.0,
                b: 0.0,
                a: 0.5,
            }));
            Ok(())
        };
        cpu.draw(paint).unwrap();
        cpu.present(front.0, true, 1.0).unwrap();
        transition.commit().unwrap();
        expect_pixel(front.0, 16, [0, 128, 127]);
        cpu.draw(paint).unwrap();
        cpu.present(front.0, true, 0.5).unwrap();
        expect_pixel(front.0, 16, [0, 64, 191]);
        cpu.resize(D2D_SIZE_U {
            width: 32,
            height: 32,
        })
        .unwrap();
        cpu.resource_target().SetDpi(144.0, 144.0);
        cpu.draw(|target| {
            paint(target)?;
            let red = target.CreateSolidColorBrush(
                &D2D1_COLOR_F {
                    r: 1.0,
                    g: 0.0,
                    b: 0.0,
                    a: 1.0,
                },
                None,
            )?;
            target.FillRectangle(
                &D2D_RECT_F {
                    left: 0.0,
                    top: 0.0,
                    right: 8.0,
                    bottom: 8.0,
                },
                &red,
            );
            Ok(())
        })
        .unwrap();
        cpu.present(front.0, true, 1.0).unwrap();
        let mut rect = RECT::default();
        GetWindowRect(front.0, &mut rect).unwrap();
        assert_eq!((rect.right - rect.left, rect.bottom - rect.top), (32, 32));
        // 8 DIP at 144 DPI equals 12 physical pixels; opposite sides of the boundary distinguish unscaled from correctly scaled output.
        expect_pixel(front.0, 10, [255, 0, 0]);
        expect_pixel(front.0, 14, [0, 128, 127]);
        assert_ne!(
            GetWindowLongPtrW(front.0, GWL_EXSTYLE) as u32 & WS_EX_NOREDIRECTIONBITMAP.0,
            0
        );
        // Product-style CPU-to-GPU entry: remove LAYERED only after confirmed retirement and old-owner release,
        // without touching NOREDIRECTIONBITMAP; rebuild a real backend and sample composited pixels rather than relying only on HRESULT.
        drop(cpu);
        drop(gpu);
        let style = GetWindowLongPtrW(front.0, GWL_EXSTYLE) as u32;
        assert_ne!(style & WS_EX_LAYERED.0, 0);
        // This isolated window has no business procedure that can destroy it reentrantly; front's lease covers the transaction.
        let owns = || IsWindow(Some(front.0)).as_bool();
        let allow = || true;
        let style = Transition::gpu_guarded(front.0, &owns).unwrap();
        let removed_layered = style.removes_layered();
        style.commit().unwrap();
        let mut visibility = crate::desktop::render_window_visibility::Reset::begin(
            front.0,
            removed_layered,
            &owns,
            &allow,
        )
        .unwrap();
        let size = D2D_SIZE_U {
            width: 32,
            height: 32,
        }; // Use actual HWND dimensions after CPU updates.
        let fresh = window(instance, true, 96, 32);
        let mut control = if d3d11 {
            compatibility(fresh.0, size)
        } else {
            RenderSurface::D3d12(Box::new(CompositionD3d12::new(fresh.0, size).unwrap()))
        };
        control.set_opacity(0.5).unwrap();
        assert!(control
            .draw(|target| {
                target.Clear(Some(&D2D1_COLOR_F {
                    r: 0.0,
                    g: 1.0,
                    b: 1.0,
                    a: 1.0,
                }));
                Ok(())
            })
            .unwrap());
        show(fresh.0);
        expect_pixel(fresh.0, 14, [0, 128, 255]);
        println!("native-software-return-control: d3d11={d3d11} fresh-hwnd=true opacity=true");
        let mut returned = if d3d11 {
            compatibility(front.0, size)
        } else {
            RenderSurface::D3d12(Box::new(CompositionD3d12::new(front.0, size).unwrap()))
        };
        assert!(returned
            .draw(|target| {
                target.Clear(Some(&D2D1_COLOR_F {
                    r: 0.0,
                    g: 1.0,
                    b: 1.0,
                    a: 1.0,
                }));
                Ok(())
            })
            .unwrap());
        visibility.restore().unwrap();
        expect_pixel(front.0, 14, [0, 255, 255]);
        println!("native-software-return-color: d3d11={d3d11} distinct-cyan=true");
        returned.set_opacity(0.5).unwrap();
        assert!(returned
            .draw(|target| {
                target.Clear(Some(&D2D1_COLOR_F {
                    r: 0.0,
                    g: 1.0,
                    b: 1.0,
                    a: 1.0,
                }));
                Ok(())
            })
            .unwrap());
        expect_pixel(front.0, 14, [0, 128, 255]);
        // Dynamically change the blue background to yellow, excluding coincidental matches from stale CPU pixels or fixed background caches.
        let dc = GetDC(Some(backdrop.0));
        assert!(!dc.is_invalid());
        let yellow = CreateSolidBrush(COLORREF(0x0000ffff));
        assert!(!yellow.is_invalid());
        assert_ne!(
            FillRect(
                dc,
                &RECT {
                    left: 0,
                    top: 0,
                    right: 128,
                    bottom: 128
                },
                yellow
            ),
            0
        );
        assert!(DeleteObject(yellow.into()).as_bool());
        assert_ne!(ReleaseDC(Some(backdrop.0), dc), 0);
        GdiFlush().ok().unwrap();
        expect_pixel(front.0, 14, [127, 255, 128]);
        assert_eq!(
            GetWindowLongPtrW(front.0, GWL_EXSTYLE) as u32 & WS_EX_LAYERED.0,
            0
        );
        assert_ne!(
            GetWindowLongPtrW(front.0, GWL_EXSTYLE) as u32 & WS_EX_NOREDIRECTIONBITMAP.0,
            0
        );
        println!("native-software-return: d3d11={d3d11} same-hwnd=true gpu-recreated=true desktop-pixel=true");
        println!("native-software-transition: d3d12={} d3d11={d3d11} same-hwnd=true retained-gpu=true alpha=true resize=true dpi=144 async-retirement=true shared-surface=true", !d3d11);
    }
}
