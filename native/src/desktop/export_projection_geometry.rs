//! 无本地编辑框时的目标几何；UI 只捕获小值，后台用原会话坐标与 canonical 取整提示换算。
use crate::{
    crop_geometry::Size, pin_geometry::PinRect, session_placement::Space, session_window::Rect,
};
use windows::{
    core::{Error, Result},
    Win32::Foundation::E_INVALIDARG,
};

#[derive(Clone, Copy)]
pub(in crate::desktop) enum Geometry {
    Window { frame: PinRect, space: Space },
    Expanded(Size),
}
impl Geometry {
    pub(super) fn logical(self, hint: Option<Rect>) -> Result<Size> {
        let invalid = || Error::new(E_INVALIDARG, "Invalid live projection geometry");
        let size = match self {
            Self::Window { frame, space } => {
                let rect = space.logical(frame, hint).map_err(|_| invalid())?;
                Size {
                    width: rect.w,
                    height: rect.h,
                }
            }
            Self::Expanded(size) => size,
        };
        if [size.width, size.height]
            .iter()
            .all(|v| v.is_finite() && *v > 0.0 && *v <= 1_000_000.0)
        {
            Ok(size)
        } else {
            Err(invalid())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn projection_live_geometry_keeps_canonical_fractional_residue_not_old_dimensions() {
        let space = Space {
            origin: (-1920, -200),
            scale: 1.5,
        };
        let hint = Rect {
            x: -20.25,
            y: 10.125,
            w: 80.25,
            h: 60.125,
        };
        let mut frame = space.physical(hint).unwrap();
        let logical = Geometry::Window { frame, space }
            .logical(Some(hint))
            .unwrap();
        assert_eq!((logical.width, logical.height), (80.25, 60.125));
        frame.width += 3;
        let logical = Geometry::Window { frame, space }
            .logical(Some(hint))
            .unwrap();
        assert_eq!((logical.width, logical.height), (82.25, 60.125));
        assert_eq!(
            space
                .physical(Rect {
                    w: logical.width,
                    h: logical.height,
                    ..hint
                })
                .unwrap(),
            frame
        );
    }
    #[test]
    fn projection_live_geometry_rejects_bad_space_and_expanded_sizes_without_old_fallback() {
        for value in [0.0, -1.0, f64::NAN, f64::INFINITY, 1_000_001.0] {
            assert!(Geometry::Expanded(Size {
                width: value,
                height: 10.0
            })
            .logical(None)
            .is_err());
        }
        assert!(Geometry::Window {
            frame: PinRect {
                x: 0,
                y: 0,
                width: 10,
                height: 10
            },
            space: Space {
                origin: (0, 0),
                scale: 0.0
            },
        }
        .logical(None)
        .is_err());
    }
    #[test]
    fn projection_live_geometry_is_only_fallback_and_preserves_existing_crop_source() {
        use super::super::*;
        let _com = Apartment::new().unwrap();
        let image = Image {
            width: 64,
            height: 32,
            pixels: vec![255; 64 * 32 * 4].into(),
        };
        let mut editor = Editor::with_layers(crate::erase_layers::Layers {
            size: (32, 16),
            base: vec![255; 32 * 16 * 4].into(),
            annotations: None,
            annotation_size: None,
        })
        .unwrap();
        editor
            .crop_to(crate::crop_geometry::Rect {
                x: 8.0,
                y: 4.0,
                width: 16.0,
                height: 8.0,
            })
            .unwrap();
        let source = br#"{"stickers":[{"id":"target","w":1,"h":1}]}"#.to_vec().into();
        let output = target_frame(
            &source,
            "target",
            &image,
            &editor,
            Presentation {
                opacity: 1.0,
                kind: crate::projection_delivery::sender::Kind::Sticker,
                geometry: Some(Geometry::Expanded(Size {
                    width: f64::NAN,
                    height: 1.0,
                })),
            },
            &std::sync::atomic::AtomicBool::new(false),
        )
        .unwrap();
        assert_eq!(
            (output.width, output.height),
            (32, 16),
            "live visible frame must not crop the full source twice"
        );
    }
}
