//! 真实 D3D12 queue/交换链与 11On12 D2D 像素；隐藏自有 HWND，不读取用户 profile 或模拟全局输入。
use super::*;
use crate::desktop::{Apartment, Window};
use windows::{
    core::w,
    Win32::{
        Foundation::*,
        Graphics::{Direct3D11::*, DirectWrite::*},
        System::LibraryLoader::GetModuleHandleW,
        UI::WindowsAndMessaging::*,
    },
};

#[test]
fn retirement_confirms_real_dcomp_without_allowing_further_frames() {
    let _serial = retirement::serial_for_test();
    with_surface(|state| {
        state.draw(|_| Ok(())).unwrap();
        state.present().unwrap();
        let mut status = state.retire_composition();
        assert!(state.draw(|_| panic!("retired surface drew")).is_err());
        assert!(state.present().is_err());
        assert!(state.resize(state.size).is_err());
        assert!(state.set_opacity(0.5).is_err());
        while status == RetirementStatus::Pending {
            std::thread::sleep(std::time::Duration::from_millis(2));
            status = state.retire_composition();
        }
        assert_eq!(status, RetirementStatus::Complete);
        assert_eq!(state.retire_composition(), RetirementStatus::Complete);
        assert!(state.draw(|_| panic!("completed retirement drew")).is_err());
    });
}

unsafe extern "system" fn procedure(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    DefWindowProcW(hwnd, msg, wp, lp)
}
struct Class(HINSTANCE);
impl Drop for Class {
    fn drop(&mut self) {
        unsafe {
            let _ = UnregisterClassW(w!("Otter.D3D12.Test"), Some(self.0));
        }
    }
}
fn with_surface(test: impl FnOnce(&mut State)) {
    static SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _serial = SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let instance: HINSTANCE = GetModuleHandleW(None).unwrap().into();
        assert_ne!(
            RegisterClassW(&WNDCLASSW {
                hInstance: instance,
                lpszClassName: w!("Otter.D3D12.Test"),
                lpfnWndProc: Some(procedure),
                ..Default::default()
            }),
            0
        );
        let _class = Class(instance);
        let window = Window(
            CreateWindowExW(
                WS_EX_NOREDIRECTIONBITMAP,
                w!("Otter.D3D12.Test"),
                w!(""),
                WS_POPUP,
                0,
                0,
                64,
                64,
                None,
                None,
                Some(instance),
                None,
            )
            .unwrap(),
        );
        let mut surface = CompositionD3d12::new(
            window.0,
            D2D_SIZE_U {
                width: 64,
                height: 64,
            },
        )
        .unwrap();
        // 不以一个 D3D11 device 冒充 D3D12：交换链和队列均能返回同一个实际 D3D12 device。
        let actual: windows::Win32::Graphics::Direct3D12::ID3D12Device =
            surface.0.chain.GetDevice().unwrap();
        assert_eq!(actual, surface.0.device.device12);
        assert_eq!(surface.resource_target(), &*surface.0.context);
        surface.set_opacity(1.0).unwrap();
        test(&mut surface.0);
    }
}

fn paint(target: &ID2D1RenderTarget) -> Result<()> {
    unsafe {
        target.Clear(Some(&D2D1_COLOR_F::default()));
        let red = target.CreateSolidColorBrush(
            &D2D1_COLOR_F {
                r: 1.0,
                g: 0.0,
                b: 0.0,
                a: 0.5,
            },
            None,
        )?;
        target.FillRectangle(
            &D2D_RECT_F {
                left: 4.0,
                top: 4.0,
                right: 12.0,
                bottom: 12.0,
            },
            &red,
        );
        let bytes = [255u8, 0, 0, 255];
        let image = target.CreateBitmap(
            D2D_SIZE_U {
                width: 1,
                height: 1,
            },
            Some(bytes.as_ptr().cast()),
            4,
            &D2D1_BITMAP_PROPERTIES {
                pixelFormat: D2D1_PIXEL_FORMAT {
                    format: DXGI_FORMAT_B8G8R8A8_UNORM,
                    alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
                },
                dpiX: 96.0,
                dpiY: 96.0,
            },
        )?;
        target.DrawBitmap(
            &image,
            Some(&D2D_RECT_F {
                left: 16.0,
                top: 4.0,
                right: 20.0,
                bottom: 8.0,
            }),
            1.0,
            D2D1_BITMAP_INTERPOLATION_MODE_NEAREST_NEIGHBOR,
            None,
        );
        let factory: IDWriteFactory = DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)?;
        let format = factory.CreateTextFormat(
            w!("Segoe UI"),
            None,
            DWRITE_FONT_WEIGHT_NORMAL,
            DWRITE_FONT_STYLE_NORMAL,
            DWRITE_FONT_STRETCH_NORMAL,
            16.0,
            w!("en-us"),
        )?;
        target.SetTextAntialiasMode(D2D1_TEXT_ANTIALIAS_MODE_GRAYSCALE);
        target.DrawText(
            &[65],
            &format,
            &D2D_RECT_F {
                left: 24.0,
                top: 4.0,
                right: 48.0,
                bottom: 32.0,
            },
            &red,
            D2D1_DRAW_TEXT_OPTIONS_NONE,
            DWRITE_MEASURING_MODE_NATURAL,
        );
    }
    Ok(())
}

// 仅测试读回：生产窗口不会增加 GPU→CPU→GPU 往返或整图副本。
struct Acquired<'a>(&'a State, &'a ID3D11Resource);
impl Drop for Acquired<'_> {
    fn drop(&mut self) {
        unsafe {
            self.0
                .device
                .bridge
                .ReleaseWrappedResources(&[Some(self.1.clone())]);
            self.0.device.immediate.Flush();
        }
    }
}
struct Mapped<'a>(&'a ID3D11DeviceContext, &'a ID3D11Texture2D);
impl Drop for Mapped<'_> {
    fn drop(&mut self) {
        unsafe {
            self.0.Unmap(self.1, 0);
        }
    }
}
fn pixels(surface: &State) -> Vec<[u8; 4]> {
    unsafe {
        let index = surface.chain.GetCurrentBackBufferIndex() as usize;
        let buffer = &surface.buffers[index];
        surface
            .device
            .bridge
            .AcquireWrappedResources(&[Some(buffer.wrapped.clone())]);
        let _acquired = Acquired(surface, &buffer.wrapped);
        let source: ID3D11Texture2D = buffer.wrapped.cast().unwrap();
        let mut desc = D3D11_TEXTURE2D_DESC::default();
        source.GetDesc(&mut desc);
        desc.Usage = D3D11_USAGE_STAGING;
        desc.BindFlags = 0;
        desc.MiscFlags = 0;
        desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ.0 as u32;
        let mut staging = None;
        surface
            .device
            .device11
            .CreateTexture2D(&desc, None, Some(&mut staging))
            .unwrap();
        let staging = staging.unwrap();
        let context = &surface.device.immediate;
        context.CopyResource(&staging, &source);
        let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
        context
            .Map(&staging, 0, D3D11_MAP_READ, 0, Some(&mut mapped))
            .unwrap();
        let _mapped = Mapped(context, &staging);
        let mut result = Vec::new();
        for y in 0..desc.Height {
            for x in 0..desc.Width {
                let data = (mapped.pData as *const u8).add((y * mapped.RowPitch + x * 4) as usize);
                result.push(std::ptr::read_unaligned(data.cast::<[u8; 4]>()));
            }
        }
        result
    }
}

#[test]
fn multiple_windows_share_generation_but_keep_independent_frames() {
    let mut released = None;
    with_surface(|first| unsafe {
        released = Some(Rc::downgrade(&first.device));
        let instance: HINSTANCE = GetModuleHandleW(None).unwrap().into();
        let mut windows = Vec::new();
        for _ in 0..3 {
            windows.push(Window(
                CreateWindowExW(
                    WS_EX_NOREDIRECTIONBITMAP,
                    w!("Otter.D3D12.Test"),
                    w!(""),
                    WS_POPUP,
                    0,
                    0,
                    64,
                    64,
                    None,
                    None,
                    Some(instance),
                    None,
                )
                .unwrap(),
            ));
        }
        let mut siblings = windows
            .iter()
            .map(|window| {
                CompositionD3d12::new(
                    window.0,
                    D2D_SIZE_U {
                        width: 64,
                        height: 64,
                    },
                )
                .unwrap()
            })
            .collect::<Vec<_>>();
        for sibling in &siblings {
            assert!(Rc::ptr_eq(&first.device, &sibling.0.device));
            assert_ne!(first.context, sibling.0.context);
            assert_ne!(first.chain, sibling.0.chain);
        }
        // 在一个真实帧中尝试另窗绘制会被拒绝，外层帧仍能结束和提交。
        first
            .draw(|target| {
                assert!(siblings[0].draw(paint).is_err());
                paint(target)
            })
            .unwrap();
        first.present().unwrap();
        for (index, sibling) in siblings.iter_mut().enumerate() {
            sibling.set_opacity((index + 1) as f32 / 4.0).unwrap();
            sibling
                .resize(D2D_SIZE_U {
                    width: 80,
                    height: 80,
                })
                .unwrap();
            sibling.draw(paint).unwrap();
            assert_eq!(pixels(&sibling.0)[5 * 80 + 17], [255, 0, 0, 255]);
            sibling.present().unwrap();
        }
        drop(siblings.remove(1));
        first.draw(paint).unwrap();
        first.present().unwrap();
        for sibling in &mut siblings {
            sibling.draw(paint).unwrap();
            sibling.present().unwrap();
        }
        drop(siblings);
        drop(windows);
    });
    assert!(
        released.unwrap().upgrade().is_none(),
        "weak cache retained last generation"
    );
}

#[test]
fn shared_generation_rejects_poison_and_releases_frame_admission() {
    with_surface(|_| {
        // 独立空设备没有 GPU 在途，毒化不会模拟或宣称真实驱动故障。
        let device = device::Device::new().unwrap();
        {
            let lease = device.begin_frame().unwrap();
            assert!(device.begin_frame().is_err());
            assert!(device.drain().is_err());
            assert!(device.ensure_usable().is_ok());
            drop(lease);
        }
        device.drain().unwrap();
        device.poison();
        assert!(device.ensure_usable().is_err());
        assert!(device.begin_frame().is_err());
        assert!(device.drain().is_err());
    });
}

#[test]
fn visual_opacity_preserves_pixels_and_survives_resize() {
    with_surface(|surface| {
        assert!(surface.opacity_effect.is_none());
        for opacity in [0.5, 0.0, 1.0] {
            surface.set_opacity(opacity).unwrap();
            assert_eq!(surface.opacity, opacity);
            let effect = surface.opacity_effect.clone().unwrap();
            surface.set_opacity(opacity).unwrap();
            assert_eq!(surface.opacity_effect.as_ref().unwrap(), &effect);
            for size in [64, 96] {
                surface
                    .resize(D2D_SIZE_U {
                        width: size,
                        height: size,
                    })
                    .unwrap();
                surface.draw(paint).unwrap();
                let data = pixels(surface);
                assert_eq!(data[(5 * size + 17) as usize], [255, 0, 0, 255]);
                assert!(data[(8 * size + 8) as usize][3].abs_diff(128) <= 1);
                surface.present().unwrap();
                assert_eq!(surface.opacity, opacity);
            }
        }
        for invalid in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, -0.1, 1.1] {
            assert_eq!(
                surface.set_opacity(invalid).unwrap_err().code(),
                E_INVALIDARG
            );
            assert_eq!(surface.opacity, 1.0);
            assert!(!surface.failed);
        }
        surface.failed = true;
        assert!(surface.set_opacity(0.25).is_err());
        assert!(surface.set_opacity(1.0).is_err());
        assert_eq!(surface.opacity, 1.0);
    });
}

#[test]
fn actual_d3d12_texture_text_alpha_present_and_resize() {
    with_surface(|surface| {
        for size in [64, 96, 64] {
            surface
                .resize(D2D_SIZE_U {
                    width: size,
                    height: size,
                })
                .unwrap();
            let mut indices = std::collections::BTreeSet::new();
            for _ in 0..4 {
                indices.insert(unsafe { surface.chain.GetCurrentBackBufferIndex() });
                surface.draw(paint).unwrap();
                let data = pixels(surface);
                assert_eq!(data[0], [0; 4]);
                let red = data[(8 * size + 8) as usize];
                assert_eq!(&red[..2], &[0, 0]);
                assert!(red[2].abs_diff(128) <= 1 && red[3].abs_diff(128) <= 1);
                assert_eq!(data[(5 * size + 17) as usize], [255, 0, 0, 255]);
                assert!(
                    (4..32).any(|y| (24..48).any(|x| data[(y * size + x) as usize][3] > 0)),
                    "text missing"
                );
                assert!(surface.present().unwrap());
            }
            assert_eq!(indices.len(), 2, "did not exercise both actual backbuffers");
        }
        surface.device.drain().unwrap();
    });
}
#[test]
fn rejected_or_panicked_draw_cannot_present_and_next_frame_recovers() {
    with_surface(|surface| {
        assert!(surface.present().is_err());
        assert!(surface
            .draw(|_| Err(Error::new(E_FAIL, "injected draw failure")))
            .is_err());
        assert!(surface.present().is_err());
        let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = surface.draw(|_| panic!("injected drawing panic"));
        }));
        assert!(panic.is_err());
        assert!(surface.present().is_err());
        surface.draw(paint).unwrap();
        assert_eq!(pixels(surface)[0], [0; 4]);
        assert!(surface.present().unwrap());
        assert!(surface.present().is_err());
    });
}
#[test]
fn invalid_resize_preserves_existing_surface() {
    with_surface(|surface| {
        for (width, height) in [
            (0, 64),
            (64, 0),
            (8193, 1),
            (8192, 8192),
            (u32::MAX, u32::MAX),
        ] {
            assert_eq!(
                surface
                    .resize(D2D_SIZE_U { width, height })
                    .unwrap_err()
                    .code(),
                E_INVALIDARG
            );
        }
        surface.draw(paint).unwrap();
        assert!(surface.present().unwrap());
    });
}
