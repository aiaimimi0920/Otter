//! 软件 DC 位图像素与真实隐藏 HWND 提交；没有 D3D device/交换链或 DComp 构造调用。
use super::*;
use crate::desktop::{Apartment, Window};
use windows::{
    core::w,
    Win32::{
        Graphics::DirectWrite::*, System::LibraryLoader::GetModuleHandleW,
        UI::WindowsAndMessaging::*,
    },
};

fn factory() -> ID2D1Factory {
    unsafe { D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None).unwrap() }
}
fn paint(target: &ID2D1RenderTarget) -> Result<()> {
    unsafe {
        target.Clear(Some(&D2D1_COLOR_F::default()));
        let brush = target.CreateSolidColorBrush(
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
            &brush,
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
        let write: IDWriteFactory = DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)?;
        let format = write.CreateTextFormat(
            w!("Segoe UI"),
            None,
            DWRITE_FONT_WEIGHT_NORMAL,
            DWRITE_FONT_STYLE_NORMAL,
            DWRITE_FONT_STRETCH_NORMAL,
            16.0,
            w!("en-us"),
        )?;
        target.DrawText(
            &[65],
            &format,
            &D2D_RECT_F {
                left: 24.0,
                top: 4.0,
                right: 48.0,
                bottom: 32.0,
            },
            &brush,
            D2D1_DRAW_TEXT_OPTIONS_NONE,
            DWRITE_MEASURING_MODE_NATURAL,
        );
    }
    Ok(())
}

#[test]
fn software_pixels_text_image_alpha_resize_and_failed_frames() {
    let _apartment = Apartment::new().unwrap();
    let mut surface = SoftwareSurface::new(
        &factory(),
        D2D_SIZE_U {
            width: 64,
            height: 64,
        },
    )
    .unwrap();
    for size in [64, 96, 64] {
        surface
            .resize(D2D_SIZE_U {
                width: size,
                height: size,
            })
            .unwrap();
        surface.draw(paint).unwrap();
        let data = surface.bitmap.pixels();
        assert_eq!(data[0], [0; 4]);
        let red = data[(8 * size + 8) as usize];
        assert_eq!(&red[..2], &[0, 0]);
        assert!(
            red[2].abs_diff(128) <= 1 && red[3].abs_diff(128) <= 1,
            "{red:?}"
        );
        assert_eq!(data[(5 * size + 17) as usize], [255, 0, 0, 255]);
        assert!((4..32).any(|y| (24..48).any(|x| data[(y * size + x) as usize][3] != 0)));
    }
    assert_eq!(
        surface.scratch.bytes(),
        96 * 96 * 8,
        "resize must retain scratch high-water budget"
    );
    for size in [
        D2D_SIZE_U {
            width: 0,
            height: 1,
        },
        D2D_SIZE_U {
            width: 8193,
            height: 1,
        },
        D2D_SIZE_U {
            width: 8192,
            height: 8192,
        },
    ] {
        assert_eq!(surface.resize(size).unwrap_err().code(), E_INVALIDARG);
    }
    assert!(surface
        .draw(|_| Err(Error::new(E_FAIL, "paint fixture")))
        .is_err());
    assert!(!surface.ready);
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = surface.draw(|_| panic!("paint unwind fixture"));
    }))
    .is_err());
    assert!(!surface.ready);
    surface.draw(paint).unwrap();
    assert!(surface.ready);
}

unsafe extern "system" fn procedure(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    DefWindowProcW(hwnd, msg, wp, lp)
}
struct Class(HINSTANCE);
impl Drop for Class {
    fn drop(&mut self) {
        unsafe {
            let _ = UnregisterClassW(w!("Otter.Software.Test"), Some(self.0));
        }
    }
}

#[test]
fn hidden_workspace_and_layered_pin_present_without_pixel_opacity_mutation() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let instance: HINSTANCE = GetModuleHandleW(None).unwrap().into();
        assert_ne!(
            RegisterClassW(&WNDCLASSW {
                hInstance: instance,
                lpszClassName: w!("Otter.Software.Test"),
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
                        WS_EX_LAYERED
                    } else {
                        WINDOW_EX_STYLE::default()
                    },
                    w!("Otter.Software.Test"),
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
            let mut surface = SoftwareSurface::new(
                &factory(),
                D2D_SIZE_U {
                    width: 64,
                    height: 64,
                },
            )
            .unwrap();
            assert!(surface.present(window.0, pinned, 1.0).is_err());
            surface.draw(paint).unwrap();
            let before = surface.bitmap.pixels();
            assert_eq!(
                surface
                    .present(window.0, pinned, f32::NAN)
                    .unwrap_err()
                    .code(),
                E_INVALIDARG
            );
            surface
                .present(window.0, pinned, if pinned { 0.5 } else { 1.0 })
                .unwrap();
            assert_eq!(surface.bitmap.pixels(), before);
            assert!(!IsWindowVisible(window.0).as_bool());
            assert!(surface.present(window.0, pinned, 1.0).is_err());
            surface.draw(paint).unwrap();
            assert_eq!(
                surface.present(window.0, !pinned, 1.0).unwrap_err().code(),
                E_INVALIDARG
            );
            surface.draw(paint).unwrap();
            surface.present(window.0, pinned, 1.0).unwrap();
        }
    }
}
