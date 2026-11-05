//! 截图命令的获取/交付策略；全屏路径不能启动区域选择或窗口枚举。
use super::*;
use crate::desktop::capture::Frame;

pub(super) struct Route {
    pub new_pin: bool,
    region: bool,
}
impl Route {
    pub fn resolve(command: usize) -> Option<Self> {
        let (new_pin, region) = match command {
            COMMAND => (false, true),
            FULL_SCREEN => (false, false),
            NEW_PIN => (true, true),
            NEW_FULL_PIN => (true, false),
            LONG_RECORDING | LONG_VERTICAL | LONG_HORIZONTAL => (true, true),
            _ => return None,
        };
        Some(Self { new_pin, region })
    }

    pub fn acquire_with(
        &self,
        snapshot: impl FnOnce() -> Result<Vec<RECT>>,
        capture: impl FnOnce() -> Result<Frame>,
        select: impl FnOnce(Image, POINT, Vec<RECT>) -> Result<Option<(Image, POINT)>>,
    ) -> Result<Option<(Image, Option<POINT>)>> {
        let windows = if self.region { snapshot()? } else { Vec::new() };
        let frame = capture()?;
        if self.region {
            select(frame.image, frame.origin, windows)
                .map(|result| result.map(|(image, origin)| (image, Some(origin))))
        } else {
            Ok(Some((frame.image, Some(frame.origin))))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn commands_separate_selection_and_document_delivery() {
        for (command, new_pin, region) in [
            (COMMAND, false, true),
            (FULL_SCREEN, false, false),
            (NEW_PIN, true, true),
            (NEW_FULL_PIN, true, false),
            (LONG_VERTICAL, true, true),
            (LONG_RECORDING, true, true),
            (LONG_HORIZONTAL, true, true),
        ] {
            let route = Route::resolve(command).unwrap();
            assert_eq!((route.new_pin, route.region), (new_pin, region));
        }
        assert!(Route::resolve(usize::MAX).is_none());
    }
    #[test]
    fn full_capture_never_enumerates_or_selects_and_preserves_pixels_and_origin() {
        for command in [FULL_SCREEN, NEW_FULL_PIN] {
            let pixels: crate::pixels::Pixels = vec![255; 16].into();
            let original = pixels.clone();
            let count = std::cell::Cell::new(0);
            let result = Route::resolve(command)
                .unwrap()
                .acquire_with(
                    || panic!("Full capture must not enumerate windows"),
                    || {
                        count.set(count.get() + 1);
                        Ok(Frame {
                            image: Image {
                                width: 2,
                                height: 2,
                                pixels,
                            },
                            origin: POINT { x: -1920, y: 80 },
                        })
                    },
                    |_, _, _| panic!("Full capture must not open selector"),
                )
                .unwrap()
                .unwrap();
            assert_eq!(count.get(), 1);
            assert!(result.0.pixels.same_storage(&original));
            let origin = result.1.unwrap();
            assert_eq!((origin.x, origin.y), (-1920, 80));
        }
    }
    #[test]
    fn region_cancellation_is_preserved_and_capture_failure_skips_selector() {
        let route = Route::resolve(NEW_PIN).unwrap();
        let result = route
            .acquire_with(
                || Ok(Vec::new()),
                || {
                    Ok(Frame {
                        image: Image {
                            width: 1,
                            height: 1,
                            pixels: vec![255; 4].into(),
                        },
                        origin: POINT::default(),
                    })
                },
                |_, _, _| Ok(None),
            )
            .unwrap();
        assert!(result.is_none());
        assert!(route
            .acquire_with(
                || Ok(Vec::new()),
                || Err(Error::new(E_FAIL, "fixture capture failure")),
                |_, _, _| panic!("No selection after capture failure")
            )
            .is_err());
    }
}
