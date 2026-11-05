//! 读回仅用于测试：证明交换链存储真实预乘 alpha，并覆盖 ResizeBuffers 的引用释放。
use super::super::{Apartment, Window};
use super::*;
use windows::{
    core::w,
    Win32::{Foundation::*, System::LibraryLoader::GetModuleHandleW, UI::WindowsAndMessaging::*},
};

unsafe extern "system" fn procedure(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    DefWindowProcW(hwnd, msg, wp, lp)
}

struct Class(HINSTANCE);

pub(super) fn retire(composition: &mut Composition) {
    let _serial = retirement::serial_for_test();
    let mut status = composition.retire_composition();
    assert!(composition
        .resize(D2D_SIZE_U {
            width: 32,
            height: 32
        })
        .is_err());
    assert!(composition.set_opacity(0.5).is_err());
    assert!(composition.present().is_err());
    assert!(composition.ensure_attached().is_err());
    while status == RetirementStatus::Pending {
        std::thread::sleep(std::time::Duration::from_millis(2));
        status = composition.retire_composition();
    }
    assert_eq!(status, RetirementStatus::Complete);
    assert_eq!(composition.retire_composition(), RetirementStatus::Complete);
}

#[test]
fn external_resize_cannot_bypass_surface_admission() {
    assert!(validate_size(D2D_SIZE_U {
        width: 4096,
        height: 4096
    })
    .is_ok());
    for (width, height) in [(0, 1), (8193, 1), (8192, 8192), (u32::MAX, u32::MAX)] {
        assert!(validate_size(D2D_SIZE_U { width, height }).is_err());
    }
}
impl Drop for Class {
    fn drop(&mut self) {
        unsafe {
            let _ = UnregisterClassW(w!("Otter.Composition.Test"), Some(self.0));
        }
    }
}

pub(super) unsafe fn samples(composition: &Composition) -> ([u8; 4], [u8; 4]) {
    let device: ID3D11Device = composition.state.chain.GetDevice().unwrap();
    let context = device.GetImmediateContext().unwrap();
    let source: ID3D11Texture2D = composition.state.chain.GetBuffer(0).unwrap();
    let mut desc = D3D11_TEXTURE2D_DESC::default();
    source.GetDesc(&mut desc);
    desc.Usage = D3D11_USAGE_STAGING;
    desc.BindFlags = 0;
    desc.MiscFlags = 0;
    desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ.0 as u32;
    let mut staging = None;
    device
        .CreateTexture2D(&desc, None, Some(&mut staging))
        .unwrap();
    let staging = staging.unwrap();
    context.CopyResource(&staging, &source);
    let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
    context
        .Map(&staging, 0, D3D11_MAP_READ, 0, Some(&mut mapped))
        .unwrap();
    let data = mapped.pData as *const u8;
    let mut clear = [0; 4];
    let mut fill = [0; 4];
    std::ptr::copy_nonoverlapping(data, clear.as_mut_ptr(), 4);
    std::ptr::copy_nonoverlapping(
        data.add(8 * mapped.RowPitch as usize + 8 * 4),
        fill.as_mut_ptr(),
        4,
    );
    context.Unmap(&staging, 0);
    (clear, fill)
}

#[test]
fn transparent_backbuffer_pixels_and_resize_are_valid() {
    verify_backbuffer(false);
}
#[test]
fn software_fallback_after_primary_chain_failure_draws_alpha_and_resizes() {
    verify_backbuffer(true);
}
fn verify_backbuffer(force_chain_failure: bool) {
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
                lpszClassName: w!("Otter.Composition.Test"),
                lpfnWndProc: Some(procedure),
                ..Default::default()
            }),
            0
        );
        let _class = Class(instance);
        let window = Window(
            CreateWindowExW(
                WS_EX_NOREDIRECTIONBITMAP,
                w!("Otter.Composition.Test"),
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
        let size = D2D_SIZE_U {
            width: 64,
            height: 64,
        };
        let mut composition = if force_chain_failure {
            let selected = selection::select(|backend| {
                if backend == selection::Backend::D3d11Hardware {
                    return Err((selection::Stage::Presentation, Error::from(E_FAIL).into()));
                }
                let device = devices::acquire(backend)
                    .map_err(|error| (selection::Stage::Device, error.into()))?;
                let surface = Composition::with_device_owner(
                    window.0,
                    size,
                    device.get(),
                    Some(device.clone()),
                )
                .map_err(|error| (selection::Stage::Presentation, error))?;
                Ok(surface)
            })
            .unwrap();
            assert_eq!(selected.backend, selection::Backend::D3d11Warp);
            assert_eq!(
                selected.fallback.unwrap().stage,
                selection::Stage::Presentation
            );
            selected.value
        } else {
            Composition::new(window.0, size).unwrap()
        };
        assert!(composition.state.opacity_effect.is_none());
        composition.set_opacity(1.0).unwrap();
        assert!(composition.state.opacity_effect.is_none());
        composition.set_opacity(0.35).unwrap();
        let effect = composition.state.opacity_effect.as_ref().unwrap().clone();
        composition.set_opacity(0.35).unwrap();
        assert_eq!(
            composition.state.opacity_effect.as_ref().unwrap().as_raw(),
            effect.as_raw()
        );
        for opacity in [f32::NAN, f32::INFINITY, -0.1, 1.1] {
            assert!(composition.set_opacity(opacity).is_err());
            assert_eq!(composition.state.opacity, 0.35);
        }
        composition.set_opacity(0.0).unwrap();
        assert_eq!(composition.state.opacity, 0.0);
        composition.set_opacity(0.35).unwrap();
        for size in [64, 96, 32] {
            assert_eq!(composition.state.opacity, 0.35);
            assert_eq!(
                composition.state.opacity_effect.as_ref().unwrap().as_raw(),
                effect.as_raw()
            );
            composition
                .resize(D2D_SIZE_U {
                    width: size,
                    height: size,
                })
                .unwrap();
            let target = composition.renderer();
            let brush = target
                .CreateSolidColorBrush(
                    &D2D1_COLOR_F {
                        r: 1.0,
                        g: 0.0,
                        b: 0.0,
                        a: 0.5,
                    },
                    None,
                )
                .unwrap();
            target.BeginDraw();
            target.Clear(Some(&D2D1_COLOR_F::default()));
            target.FillRectangle(
                &D2D_RECT_F {
                    left: 4.0,
                    top: 4.0,
                    right: 12.0,
                    bottom: 12.0,
                },
                &brush,
            );
            target.EndDraw(None, None).unwrap();
            let (clear, fill) = samples(&composition);
            assert_eq!(clear, [0, 0, 0, 0]);
            assert_eq!(&fill[..2], &[0, 0]);
            assert!(fill[2].abs_diff(128) <= 1 && fill[3].abs_diff(128) <= 1);
            composition.present().unwrap();
        }
        retire(&mut composition);
        let mut surface =
            crate::desktop::render_surface::RenderSurface::D3d11(Box::new(composition));
        assert!(surface
            .draw(|_| panic!("retired D3D11 entered BeginDraw"))
            .is_err());
    }
}
