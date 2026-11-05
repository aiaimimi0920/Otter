//! 普通重定向 HWND 的真实 D3D11→BitBlt CPU→D3D11；复用可见门的窗口和像素采样，不发送输入。
use super::*;

unsafe fn retire(surface: &mut RenderSurface, hwnd: HWND) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    loop {
        match surface.retire_composition().unwrap() {
            RetirementStatus::Complete => break,
            RetirementStatus::Pending => {}
            status => panic!("workspace retirement failed: {status:?}"),
        }
        assert!(std::time::Instant::now() < deadline);
        let mut message = MSG::default();
        for _ in 0..64 {
            if !PeekMessageW(&mut message, Some(hwnd), 0, 0, PM_REMOVE).as_bool() {
                break;
            }
            DispatchMessageW(&message);
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
}

#[test]
#[ignore = "brief visible topmost test window; run explicitly with desktop access"]
fn redirected_workspace_d3d11_bitblt_round_trip_preserves_pixels_and_dpi() {
    let _serial = crate::desktop::composition_retirement::serial_for_test();
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let previous = SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        assert!(!previous.0.is_null());
        let _dpi = DpiScope(previous);
        let instance: HINSTANCE = GetModuleHandleW(None).unwrap().into();
        assert_ne!(
            RegisterClassW(&WNDCLASSW {
                hInstance: instance,
                lpszClassName: w!("Otter.Software.Transition.Test"),
                lpfnWndProc: Some(procedure),
                ..Default::default()
            }),
            0
        );
        let _class = Class(instance);
        let front = window(instance, false, 48, 64);
        let foreground = GetForegroundWindow();
        let size = D2D_SIZE_U {
            width: 64,
            height: 64,
        };
        let mut gpu = compatibility(front.0, size);
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
        retire(&mut gpu, front.0);
        drop(gpu);

        let factory: ID2D1Factory =
            D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None).unwrap();
        let mut cpu = SoftwareSurface::new(&factory, size).unwrap();
        let transition = Transition::begin(front.0, false).unwrap();
        cpu.resource_target().SetDpi(144.0, 144.0);
        cpu.draw(|target| {
            target.Clear(Some(&D2D1_COLOR_F {
                r: 0.0,
                g: 1.0,
                b: 0.0,
                a: 1.0,
            }));
            let brush = target.CreateSolidColorBrush(
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
                &brush,
            );
            Ok(())
        })
        .unwrap();
        cpu.present(front.0, false, 1.0).unwrap();
        transition.commit().unwrap();
        expect_pixel(front.0, 10, [255, 0, 0]);
        expect_pixel(front.0, 14, [0, 255, 0]);
        drop(cpu);

        let owns = || IsWindow(Some(front.0)).as_bool();
        let style = Transition::gpu_guarded(front.0, &owns).unwrap();
        assert!(!style.removes_layered());
        style.commit().unwrap();
        let allow = || true;
        let mut reset =
            crate::desktop::render_window_visibility::Reset::begin(front.0, false, &owns, &allow)
                .unwrap();
        let mut gpu = compatibility(front.0, size);
        gpu.resource_target().SetDpi(144.0, 144.0);
        assert!(gpu
            .draw(|target| {
                target.Clear(Some(&D2D1_COLOR_F {
                    r: 0.0,
                    g: 0.0,
                    b: 1.0,
                    a: 1.0,
                }));
                let brush = target.CreateSolidColorBrush(
                    &D2D1_COLOR_F {
                        r: 1.0,
                        g: 1.0,
                        b: 1.0,
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
                    &brush,
                );
                Ok(())
            })
            .unwrap());
        reset.restore().unwrap();
        expect_pixel(front.0, 10, [255, 255, 255]);
        expect_pixel(front.0, 14, [0, 0, 255]);
        assert_eq!(
            GetWindowLongPtrW(front.0, GWL_EXSTYLE) as u32
                & (WS_EX_LAYERED.0 | WS_EX_NOREDIRECTIONBITMAP.0),
            0
        );
        assert_eq!(GetForegroundWindow(), foreground);
        retire(&mut gpu, front.0);
    }
}
