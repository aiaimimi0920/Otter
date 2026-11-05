//! 非破坏性裁剪坐标合同：源框是逻辑坐标而非像素，保留框外内容以供重置。
//! 对照主干 stickerEditingGeometry / stickerCropImage；只计算布局，不分配图像。

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Size {
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Crop {
    pub rect: Rect,
    pub source: Size,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Change {
    pub frame: Rect,
    pub crop: Option<Crop>,
    /// 现有标注/擦除轨迹只平移，不缩放或删除框外几何。
    pub edit_offset: (f64, f64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidGeometry;

impl Size {
    fn valid(self) -> bool {
        self.width.is_finite()
            && self.height.is_finite()
            && self.width > 0.0
            && self.height > 0.0
            && self.width <= 1_000_000.0
            && self.height <= 1_000_000.0
    }
}
impl Rect {
    fn size(self) -> Size {
        Size {
            width: self.width,
            height: self.height,
        }
    }
    fn valid(self) -> bool {
        self.size().valid()
            && self.x.is_finite()
            && self.y.is_finite()
            && self.x.abs() <= 1_000_000.0
            && self.y.abs() <= 1_000_000.0
    }
}
impl Crop {
    fn valid(self) -> bool {
        self.rect.valid() && self.source.valid()
    }
}

/// selection 使用当前可见框的局部坐标；在原有裁剪上累积，不重采样源图。
pub fn next(
    frame: Rect,
    existing: Option<Crop>,
    selection: Rect,
) -> Result<Change, InvalidGeometry> {
    if !frame.valid() || !selection.valid() || existing.is_some_and(|crop| !crop.valid()) {
        return Err(InvalidGeometry);
    }
    let base = existing.unwrap_or(Crop {
        rect: Rect {
            x: 0.0,
            y: 0.0,
            width: frame.width,
            height: frame.height,
        },
        source: frame.size(),
    });
    let scale_x = base.rect.width / frame.width;
    let scale_y = base.rect.height / frame.height;
    let crop = Crop {
        rect: Rect {
            x: base.rect.x + selection.x * scale_x,
            y: base.rect.y + selection.y * scale_y,
            width: selection.width * scale_x,
            height: selection.height * scale_y,
        },
        source: base.source,
    };
    let frame = Rect {
        x: frame.x + selection.x,
        y: frame.y + selection.y,
        ..selection
    };
    if !frame.valid() || !crop.valid() {
        return Err(InvalidGeometry);
    }
    Ok(Change {
        frame,
        crop: Some(crop),
        edit_offset: (-selection.x, -selection.y),
    })
}

/// 恢复完整源框并保持当前显示比例；清除旧 sourceSize，下一轮裁剪从恢复后的框开始。
pub fn reset(frame: Rect, crop: Option<Crop>) -> Result<Change, InvalidGeometry> {
    if !frame.valid() || crop.is_some_and(|crop| !crop.valid()) {
        return Err(InvalidGeometry);
    }
    let Some(crop) = crop else {
        return Ok(Change {
            frame,
            crop: None,
            edit_offset: (0.0, 0.0),
        });
    };
    let sx = frame.width / crop.rect.width;
    let sy = frame.height / crop.rect.height;
    let restored = Rect {
        x: frame.x - crop.rect.x * sx,
        y: frame.y - crop.rect.y * sy,
        width: crop.source.width * sx,
        height: crop.source.height * sy,
    };
    if !restored.valid() {
        return Err(InvalidGeometry);
    }
    Ok(Change {
        frame: restored,
        crop: None,
        edit_offset: (frame.x - restored.x, frame.y - restored.y),
    })
}

/// 先在保留的完整源框 contain-fit，再按裁剪窗口偏移；不拉伸或丢掉留白。
pub fn image_placement(frame: Rect, crop: Crop, pixels: Size) -> Result<Rect, InvalidGeometry> {
    if !frame.valid() || !crop.valid() || !pixels.valid() {
        return Err(InvalidGeometry);
    }
    let sx = frame.width / crop.rect.width;
    let sy = frame.height / crop.rect.height;
    let width = crop.source.width * sx;
    let height = crop.source.height * sy;
    let scale = (width / pixels.width).min(height / pixels.height);
    let image_width = pixels.width * scale;
    let image_height = pixels.height * scale;
    let placement = Rect {
        x: frame.x + (width - image_width) / 2.0 - crop.rect.x * sx,
        y: frame.y + (height - image_height) / 2.0 - crop.rect.y * sy,
        width: image_width,
        height: image_height,
    };
    if !placement.valid() {
        return Err(InvalidGeometry);
    }
    Ok(placement)
}

pub fn source_resolution(crop: Crop, pixels: Size) -> Result<Size, InvalidGeometry> {
    if !crop.valid() || !pixels.valid() {
        return Err(InvalidGeometry);
    }
    let density = (pixels.width / crop.source.width).max(pixels.height / crop.source.height);
    let result = Size {
        width: crop.rect.width * density,
        height: crop.rect.height * density,
    };
    if !result.valid() {
        return Err(InvalidGeometry);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn rect(x: f64, y: f64, width: f64, height: f64) -> Rect {
        Rect {
            x,
            y,
            width,
            height,
        }
    }
    fn size(width: f64, height: f64) -> Size {
        Size { width, height }
    }
    #[test]
    fn main_fixture_second_crop_after_resize_and_reset_preserve_scale() {
        let crop = Crop {
            rect: rect(100.0, 50.0, 200.0, 100.0),
            source: size(400.0, 200.0),
        };
        let frame = rect(500.0, 300.0, 400.0, 200.0);
        let changed = next(frame, Some(crop), rect(60.0, 40.0, 200.0, 100.0)).unwrap();
        assert_eq!(changed.frame, rect(560.0, 340.0, 200.0, 100.0));
        assert_eq!(
            changed.crop.unwrap(),
            Crop {
                rect: rect(130.0, 70.0, 100.0, 50.0),
                source: crop.source
            }
        );
        assert_eq!(changed.edit_offset, (-60.0, -40.0));
        let restored = reset(frame, Some(crop)).unwrap();
        assert_eq!(restored.frame, rect(300.0, 200.0, 800.0, 400.0));
        assert_eq!(restored.crop, None);
    }
    #[test]
    fn main_fixture_repeated_crop_keeps_outside_geometry_and_reset_offset() {
        let original = rect(100.0, 200.0, 400.0, 300.0);
        let first = next(original, None, rect(50.0, 40.0, 300.0, 200.0)).unwrap();
        let second = next(first.frame, first.crop, rect(30.0, 20.0, 200.0, 100.0)).unwrap();
        assert_eq!(second.crop.unwrap().rect, rect(80.0, 60.0, 200.0, 100.0));
        let outside = (
            10.0 + first.edit_offset.0 + second.edit_offset.0,
            10.0 + first.edit_offset.1 + second.edit_offset.1,
        );
        assert_eq!(outside, (-70.0, -50.0));
        let restored = reset(second.frame, second.crop).unwrap();
        assert_eq!(restored.frame, original);
        assert_eq!(
            (
                outside.0 + restored.edit_offset.0,
                outside.1 + restored.edit_offset.1
            ),
            (10.0, 10.0)
        );
    }
    #[test]
    fn main_fixtures_use_logical_coordinates_and_keep_letterbox() {
        let crop = Crop {
            rect: rect(100.0, 50.0, 200.0, 100.0),
            source: size(400.0, 200.0),
        };
        assert_eq!(
            image_placement(rect(0.0, 0.0, 200.0, 100.0), crop, size(800.0, 400.0)).unwrap(),
            rect(-100.0, -50.0, 400.0, 200.0)
        );
        assert_eq!(
            source_resolution(crop, size(800.0, 400.0)).unwrap(),
            size(400.0, 200.0)
        );
        let crop = Crop {
            rect: rect(50.0, 80.0, 250.0, 200.0),
            source: size(400.0, 400.0),
        };
        assert_eq!(
            image_placement(rect(0.0, 0.0, 250.0, 200.0), crop, size(800.0, 400.0)).unwrap(),
            rect(-50.0, 20.0, 400.0, 200.0)
        );
    }
    #[test]
    fn invalid_and_overflowing_geometry_never_produces_partial_change() {
        let frame = rect(-500.0, -200.0, 400.0, 200.0);
        assert_eq!(reset(frame, None).unwrap().frame, frame);
        for value in [0.0, -1.0, f64::NAN, f64::INFINITY, 1e100] {
            assert!(next(frame, None, rect(0.0, 0.0, value, 20.0)).is_err());
        }
        let tiny = Crop {
            rect: rect(0.0, 0.0, f64::MIN_POSITIVE, 1.0),
            source: size(400.0, 200.0),
        };
        assert!(reset(frame, Some(tiny)).is_err());
        assert!(image_placement(frame, tiny, size(800.0, 400.0)).is_err());
    }
}
