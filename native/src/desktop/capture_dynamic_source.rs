//! 生产动态范围采集适配：复用现有 GDI 区域截图，不复制采集实现，不负责文件或 UI 交付。
use super::Image;
use crate::{
    capture_color::SdrFrame,
    capture_dynamic::{self, Mode, Options, Outcome},
};
use std::sync::atomic::AtomicBool;
use windows::Win32::Foundation::RECT;

/// 由显式截图请求的后台线程调用；整个流程都使用同一物理 screen，不能在 UI 线程等待 WGC。
/// 取消可阻止开始或交付，但无法中断已经进入的 BitBlt/驱动调用。
pub fn region(
    screen: RECT,
    mode: Mode,
    standard_region: bool,
    gain: f32,
    cancel: &AtomicBool,
) -> Result<Outcome, String> {
    if cancel.load(std::sync::atomic::Ordering::Acquire) {
        return Err("Capture cancelled".into());
    }
    let options = Options::detect(mode, standard_region, gain)
        .map_err(|error| format!("Cannot determine capture platform: {error:?}"))?;
    run(screen, options, cancel, |screen| {
        let (width, height) = crate::capture_target::dimensions(screen)
            .map_err(|error| format!("Invalid capture rectangle: {error:?}"))?;
        super::region(screen.left, screen.top, width as i32, height as i32)
            .map_err(|error| error.to_string())
    })
}
fn run(
    screen: RECT,
    options: Options,
    cancel: &AtomicBool,
    capture: impl FnOnce(RECT) -> Result<Image, String>,
) -> Result<Outcome, String> {
    capture_dynamic::region(screen, options, cancel, || {
        let image = capture(screen)?;
        Ok(SdrFrame {
            width: image.width,
            height: image.height,
            pixels: image.pixels,
        })
    })
    .map_err(|error| format!("Dynamic range capture failed: {error:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capture_dynamic::{Frame, Reason};
    use std::sync::atomic::Ordering;
    use windows::Win32::{Foundation::COLORREF, Graphics::Gdi::SetPixelV};
    #[test]
    fn sdr_adapter_preserves_negative_screen_and_real_memory_dc_pixels() {
        let screen = RECT {
            left: -1920,
            top: -200,
            right: -1918,
            bottom: -198,
        };
        let cancel = AtomicBool::new(false);
        let result = run(
            screen,
            Options::detect(Mode::Sdr, true, 1.0).unwrap(),
            &cancel,
            |received| {
                assert_eq!(received, screen);
                super::super::synthetic(2, 2, |dc| unsafe {
                    SetPixelV(dc, 0, 0, COLORREF(0xff)).ok()?;
                    SetPixelV(dc, 1, 1, COLORREF(0xff0000)).ok()
                })
                .map_err(|e| e.to_string())
            },
        )
        .unwrap();
        assert_eq!(result.reason, Some(Reason::ExplicitSdr));
        assert!(!result.downgraded);
        let Frame::Sdr(frame) = result.frame else {
            panic!("SDR adapter returned HDR")
        };
        assert_eq!(&frame.pixels[..4], &[0, 0, 255, 255]);
        assert_eq!(&frame.pixels[12..16], &[255, 0, 0, 255]);
    }
    #[test]
    fn cancelled_invalid_and_failed_requests_do_not_publish_or_capture_twice() {
        let screen = RECT {
            left: 0,
            top: 0,
            right: 1,
            bottom: 1,
        };
        let options = Options::detect(Mode::Sdr, true, 1.0).unwrap();
        let cancel = AtomicBool::new(true);
        assert!(run(screen, options, &cancel, |_| panic!("must not capture")).is_err());
        assert!(region(screen, Mode::Hdr, true, 1.0, &cancel).is_err());
        cancel.store(false, Ordering::Release);
        assert!(run(RECT::default(), options, &cancel, |_| panic!(
            "invalid rectangle"
        ))
        .is_err());
        let calls = std::cell::Cell::new(0);
        let result = run(screen, options, &cancel, |_| {
            calls.set(calls.get() + 1);
            Err("injected GDI failure".into())
        });
        assert!(result.err().unwrap().contains("injected GDI failure"));
        assert_eq!(calls.get(), 1);
        let result = run(screen, options, &cancel, |_| {
            cancel.store(true, Ordering::Release);
            Ok(Image {
                width: 1,
                height: 1,
                pixels: vec![255; 4].into(),
            })
        });
        assert!(result.err().unwrap().contains("Cancelled"));
    }
}
