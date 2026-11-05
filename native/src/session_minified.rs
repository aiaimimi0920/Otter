//! 主干缩略是取景而非整图缩放；图片与标注分别对应 stickerFrameGeometry 的两个 viewport。
use crate::{crop_geometry::Size, session_window::Window};
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct View {
    pub size: Size,
    pub offset: (f64, f64),
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Views {
    pub image: View,
    pub annotations: View,
}
/// 文档坐标到缩略逻辑客户区的轴向变换，绘制与命中共同消费。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mapping {
    pub scale: (f64, f64),
    pub offset: (f64, f64),
}
impl View {
    pub fn document_mapping(
        self,
        frame: Option<crate::crop_geometry::Rect>,
    ) -> Result<Mapping, Error> {
        let frame = frame.unwrap_or(crate::crop_geometry::Rect {
            x: 0.0,
            y: 0.0,
            width: self.size.width,
            height: self.size.height,
        });
        if ![
            frame.x,
            frame.y,
            frame.width,
            frame.height,
            self.size.width,
            self.size.height,
            self.offset.0,
            self.offset.1,
        ]
        .iter()
        .all(|v| v.is_finite())
            || frame.width <= 0.0
            || frame.height <= 0.0
            || self.size.width <= 0.0
            || self.size.height <= 0.0
        {
            return Err(Error::Geometry);
        }
        let scale = (
            self.size.width / frame.width,
            self.size.height / frame.height,
        );
        let offset = (
            -frame.x * scale.0 - self.offset.0,
            -frame.y * scale.1 - self.offset.1,
        );
        if ![scale.0, scale.1, offset.0, offset.1]
            .iter()
            .all(|v| v.is_finite() && v.abs() <= f32::MAX as f64)
        {
            return Err(Error::Geometry);
        }
        Ok(Mapping { scale, offset })
    }
}
#[derive(Debug, PartialEq)]
pub enum Error {
    Geometry,
}

pub fn restore(window: &Window<'_>, intrinsic: Option<Size>) -> Result<Option<Views>, Error> {
    if !window.is_minified() {
        return Ok(None);
    }
    if !window.valid() {
        return Err(Error::Geometry);
    }
    let offset = window
        .crop_offset
        .map_or((0.0, 0.0), |offset| (offset.x, offset.y));
    let annotations = View {
        size: window.saved_rect.map_or(
            Size {
                width: window.w,
                height: window.h,
            },
            |rect| Size {
                width: rect.w,
                height: rect.h,
            },
        ),
        offset,
    };
    let image =
        if let Some(crop) = crate::session_crop::restore(window).map_err(|_| Error::Geometry)? {
            View {
                size: crop.crop.source,
                offset: (crop.crop.rect.x + offset.0, crop.crop.rect.y + offset.1),
            }
        } else if let (Some(saved), Some(source)) = (window.saved_rect, intrinsic) {
            if !source.width.is_finite()
                || !source.height.is_finite()
                || source.width <= 0.0
                || source.height <= 0.0
            {
                return Err(Error::Geometry);
            }
            let scale = (saved.w / source.width).min(saved.h / source.height);
            let size = Size {
                width: source.width * scale,
                height: source.height * scale,
            };
            View {
                size,
                offset: (
                    (offset.0 - (saved.w - size.width) / 2.0)
                        .clamp(0.0, (size.width - window.w.max(1.0)).max(0.0)),
                    (offset.1 - (saved.h - size.height) / 2.0)
                        .clamp(0.0, (size.height - window.h.max(1.0)).max(0.0)),
                ),
            }
        } else {
            View {
                size: window.saved_rect.map_or(
                    Size {
                        width: 100.0,
                        height: 100.0,
                    },
                    |rect| Size {
                        width: rect.w,
                        height: rect.h,
                    },
                ),
                offset,
            }
        };
    for view in [image, annotations] {
        if ![
            view.size.width,
            view.size.height,
            view.offset.0,
            view.offset.1,
        ]
        .iter()
        .all(|value| value.is_finite() && value.abs() <= 1_000_000.0)
            || view.size.width <= 0.0
            || view.size.height <= 0.0
        {
            return Err(Error::Geometry);
        }
    }
    Ok(Some(Views { image, annotations }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn document_mapping_scales_each_axis_then_subtracts_crop_and_mini_offsets() {
        let view = View {
            size: Size {
                width: 40.0,
                height: 12.0,
            },
            offset: (7.0, -2.0),
        };
        let frame = crate::crop_geometry::Rect {
            x: 3.25,
            y: 1.5,
            width: 20.0,
            height: 4.0,
        };
        let mapping = view.document_mapping(Some(frame)).unwrap();
        assert_eq!(mapping.scale, (2.0, 3.0));
        assert_eq!(mapping.offset, (-13.5, -2.5));
        assert_eq!(frame.x * mapping.scale.0 + mapping.offset.0, -7.0);
        assert_eq!(frame.y * mapping.scale.1 + mapping.offset.1, 2.0);
        assert!(view
            .document_mapping(Some(crate::crop_geometry::Rect {
                width: 0.0,
                ..frame
            }))
            .is_err());
        assert!(view
            .document_mapping(Some(crate::crop_geometry::Rect {
                x: f64::NAN,
                ..frame
            }))
            .is_err());
    }
    #[test]
    fn contain_margins_shift_image_but_not_annotation_coordinates() {
        let window = serde_json::from_str(r#"{"id":"u","x":200,"y":300,"w":100,"h":100,"minified":true,"savedRect":{"x":0,"y":0,"w":400,"h":200},"cropOffset":{"x":250,"y":40}}"#).unwrap();
        let views = restore(
            &window,
            Some(Size {
                width: 1000.0,
                height: 1000.0,
            }),
        )
        .unwrap()
        .unwrap();
        assert_eq!(
            views.image,
            View {
                size: Size {
                    width: 200.0,
                    height: 200.0
                },
                offset: (100.0, 40.0)
            }
        );
        assert_eq!(
            views.annotations,
            View {
                size: Size {
                    width: 400.0,
                    height: 200.0
                },
                offset: (250.0, 40.0)
            }
        );
    }
    #[test]
    fn crop_adds_offset_in_saved_source_space_and_missing_saved_uses_main_defaults() {
        let window = serde_json::from_str(r#"{"id":"u","x":0,"y":0,"w":80,"h":60,"minified":true,"cropOffset":{"x":-10,"y":5},"imageEditState":{"cropRect":{"x":20,"y":30,"w":200,"h":100},"sourceSize":{"w":400,"h":300}}}"#).unwrap();
        let views = restore(&window, None).unwrap().unwrap();
        assert_eq!(
            views.image,
            View {
                size: Size {
                    width: 400.0,
                    height: 300.0
                },
                offset: (10.0, 35.0)
            }
        );
        assert_eq!(
            views.annotations.size,
            Size {
                width: 80.0,
                height: 60.0
            }
        );
        let window =
            serde_json::from_str(r#"{"id":"u","x":0,"y":0,"w":80,"h":60,"minified":true}"#)
                .unwrap();
        assert_eq!(
            restore(&window, None).unwrap().unwrap().image.size,
            Size {
                width: 100.0,
                height: 100.0
            }
        );
    }
}
