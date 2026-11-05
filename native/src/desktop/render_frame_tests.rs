//! 真实 WIC/D2D 帧验证，保护错误/panic 后 EndDraw 及 clip/transform 恢复，不以 mock 代替像素。
use super::*;
use windows::{
    core::Error,
    Win32::{
        Foundation::{E_ABORT, E_FAIL},
        Graphics::{Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM, Imaging::*},
        System::Com::*,
    },
};

fn fixture(test: impl FnOnce(&ID2D1RenderTarget, &IWICBitmap)) {
    let _apartment = crate::desktop::Apartment::new().unwrap();
    unsafe {
        let wic: IWICImagingFactory =
            CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER).unwrap();
        let output = wic
            .CreateBitmap(8, 8, &GUID_WICPixelFormat32bppPBGRA, WICBitmapCacheOnLoad)
            .unwrap();
        let factory: ID2D1Factory =
            D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None).unwrap();
        let target = factory
            .CreateWicBitmapRenderTarget(
                &output,
                &D2D1_RENDER_TARGET_PROPERTIES {
                    dpiX: 96.0,
                    dpiY: 96.0,
                    pixelFormat: D2D1_PIXEL_FORMAT {
                        format: DXGI_FORMAT_B8G8R8A8_UNORM,
                        alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
                    },
                    ..Default::default()
                },
            )
            .unwrap();
        test(&target, &output);
    }
}
fn solid(target: &ID2D1RenderTarget) -> Result<()> {
    unsafe {
        target.Clear(Some(&D2D1_COLOR_F::default()));
        let brush = target.CreateSolidColorBrush(
            &D2D1_COLOR_F {
                r: 0.0,
                g: 1.0,
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
    }
    Ok(())
}
fn assert_pixels(target: &ID2D1RenderTarget, output: &IWICBitmap) {
    draw(target, || solid(target)).unwrap();
    let mut pixels = [0u8; 256];
    unsafe {
        output
            .CopyPixels(std::ptr::null(), 32, &mut pixels)
            .unwrap();
    }
    assert!(pixels
        .chunks_exact(4)
        .all(|pixel| pixel == [0, 255, 0, 255]));
}
#[test]
fn error_and_panic_restore_state_and_leave_next_frame_drawable() {
    fixture(|target, output| {
        for panic in [false, true] {
            let mut original = Default::default();
            unsafe {
                target.GetTransform(&mut original);
            }
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                draw(target, || {
                    let _transform = save_transform(target);
                    let mut moved = original;
                    moved.M31 = 100.0;
                    unsafe {
                        target.SetTransform(&moved);
                    }
                    let _clip = Clip::push(
                        target,
                        &D2D_RECT_F {
                            left: 0.0,
                            top: 0.0,
                            right: 1.0,
                            bottom: 1.0,
                        },
                    );
                    if panic {
                        panic!("injected drawing panic");
                    }
                    Err(Error::new(E_ABORT, "injected drawing error"))
                })
            }));
            if panic {
                assert!(result.is_err());
            } else {
                assert_eq!(result.unwrap().unwrap_err().code(), E_ABORT);
            }
            let mut restored = Default::default();
            unsafe {
                target.GetTransform(&mut restored);
            }
            assert_eq!(restored, original);
            assert_pixels(target, output);
        }
    });
}
#[test]
fn end_draw_failure_is_reported_and_paint_error_keeps_precedence() {
    fixture(|target, output| {
        let error = draw(target, || {
            // 合法测试故障注入：无匹配 push 的 pop 由 D2D 在 EndDraw 报错，不访问无效指针。
            unsafe {
                target.PopAxisAlignedClip();
            }
            Ok(())
        })
        .unwrap_err();
        assert!(error.code().is_err());
        assert_pixels(target, output);
        let error = draw(target, || {
            unsafe {
                target.PopAxisAlignedClip();
            }
            Err(Error::new(E_FAIL, "primary paint error"))
        })
        .unwrap_err();
        assert_eq!(error.code(), E_FAIL);
        assert_eq!(error.message(), "primary paint error");
        assert_pixels(target, output);
    });
}
