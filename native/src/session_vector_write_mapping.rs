//! Editor -> 窗口 DIP -> 物理像素 -> 会话局部逻辑单位；屏幕位置由窗口几何 owner 另行写出。
use super::*;
#[derive(Clone, Copy, Debug)]
pub struct Mapping {
    scale: f32,
    offset: Point,
}
impl Mapping {
    /// 仅有界仿射接缝使用；普通保存仍从明确viewport/annotation frame构造。
    pub fn from_components(scale: f32, offset: Point) -> Result<Self, Error> {
        if scale <= 0.0 {
            return Err(Error::Geometry);
        }
        finite(scale)?;
        finite(offset.x)?;
        finite(offset.y)?;
        Ok(Self { scale, offset })
    }
    pub fn components(self) -> (f32, Point) {
        (self.scale, self.offset)
    }
    pub fn inverse(self) -> Result<Self, Error> {
        Self::from_components(
            1.0 / self.scale,
            Point {
                x: -self.offset.x / self.scale,
                y: -self.offset.y / self.scale,
            },
        )
    }
    pub fn then(self, next: Self) -> Result<Self, Error> {
        Self::from_components(self.scale * next.scale, next.point(self.offset)?)
    }
    /// 缩略标注保存到展开参考框；cropOffset 由窗口字段拥有，不能在此重复减去。
    /// 当前主干样式只有标量线宽/字号，非等比变换不能伪装为无损写出。
    pub fn from_annotation_frame(
        frame: Option<crate::crop_geometry::Rect>,
        target: crate::crop_geometry::Size,
    ) -> Result<Self, Error> {
        let frame = frame.unwrap_or(crate::crop_geometry::Rect {
            x: 0.0,
            y: 0.0,
            width: target.width,
            height: target.height,
        });
        if ![
            frame.x,
            frame.y,
            frame.width,
            frame.height,
            target.width,
            target.height,
        ]
        .iter()
        .all(|v| v.is_finite() && v.abs() <= 1_000_000.0)
            || frame.width <= 0.0
            || frame.height <= 0.0
            || target.width <= 0.0
            || target.height <= 0.0
        {
            return Err(Error::Geometry);
        }
        let sx = target.width / frame.width;
        let sy = target.height / frame.height;
        // 只容忍 f32 转换看不见的数值误差，不用容差掩盖实际的横纵拉伸。
        if sx as f32 != sy as f32 {
            return Err(Error::Unsupported);
        }
        let mapping = Self {
            scale: sx as f32,
            offset: Point {
                x: (-frame.x * sx) as f32,
                y: (-frame.y * sy) as f32,
            },
        };
        if mapping.scale <= 0.0 {
            return Err(Error::Geometry);
        }
        finite(mapping.scale)?;
        finite(mapping.offset.x)?;
        finite(mapping.offset.y)?;
        Ok(mapping)
    }
    pub fn from_viewport(
        view: crate::viewport::Viewport,
        dpi: u32,
        space: crate::session_placement::Space,
    ) -> Result<Self, Error> {
        if dpi == 0
            || !space.scale.is_finite()
            || space.scale <= 0.0
            || !view.scale.is_finite()
            || view.scale <= 0.0
        {
            return Err(Error::Geometry);
        }
        let factor = f64::from(dpi) / 96.0 / space.scale;
        let mapping = Self {
            scale: (f64::from(view.scale) * factor) as f32,
            offset: Point {
                x: (f64::from(view.origin.x) * factor) as f32,
                y: (f64::from(view.origin.y) * factor) as f32,
            },
        };
        if mapping.scale <= 0.0 {
            return Err(Error::Geometry);
        }
        finite(mapping.scale)?;
        finite(mapping.offset.x)?;
        finite(mapping.offset.y)?;
        Ok(mapping)
    }
    pub fn point(self, point: Point) -> Result<Point, Error> {
        let p = Point {
            x: point.x * self.scale + self.offset.x,
            y: point.y * self.scale + self.offset.y,
        };
        finite(p.x)?;
        finite(p.y)?;
        Ok(p)
    }
    pub(super) fn apply(
        self,
        stroke: &Stroke,
        value: &mut Value,
        baseline: &mut impl FnMut(&crate::text_annotation::TextAnnotation) -> Result<f32, Error>,
        cancelled: &impl Fn() -> bool,
    ) -> Result<(), Error> {
        let object = value.as_object_mut().ok_or(Error::Shape)?;
        for (key, offset) in [("x", self.offset.x), ("y", self.offset.y)] {
            field(object, key, self.scale, offset)?;
        }
        for key in ["w", "h", "brushWidth", "strength"] {
            field(object, key, self.scale, 0.0)?;
        }
        if let Some(Value::Array(points)) = object.get_mut("points") {
            for point in points {
                check(cancelled)?;
                let point = point.as_object_mut().ok_or(Error::Shape)?;
                field(point, "x", self.scale, self.offset.x)?;
                field(point, "y", self.scale, self.offset.y)?;
            }
        }
        if let Some(text) = &stroke.text {
            let text = text
                .with_font_size(text.font_size() * self.scale)
                .ok_or(Error::Unsupported)?;
            object.insert("fontSize".into(), finite(text.font_size())?);
            if stroke.tool == Tool::Text {
                // 主干文字的 4px 锚点偏移不是文档内容，不能随画布缩放重复放大。
                let (start, _) = bounds(&stroke.points)?;
                let start = self.point(start)?;
                let base = baseline(&text)?;
                if !base.is_finite() || base < 0.0 {
                    return Err(Error::Geometry);
                }
                object.insert("x".into(), finite(start.x + 4.0)?);
                object.insert("y".into(), finite(start.y + base)?);
            }
        }
        let style = object
            .get_mut("style")
            .and_then(Value::as_object_mut)
            .ok_or(Error::Shape)?;
        for key in ["width", "cornerRadius"] {
            field(style, key, self.scale, 0.0)?;
        }
        let width = number(style.get("width").ok_or(Error::Shape)?)?;
        let radius = number(style.get("cornerRadius").ok_or(Error::Shape)?)?;
        let valid = crate::stroke_style::StrokeStyle::restored(width, stroke.color)
            .ok_or(Error::Unsupported)?;
        if stroke.tool == Tool::Serial {
            if !(0.0..=1_000_000.0).contains(&radius) {
                return Err(Error::Unsupported);
            }
        } else if valid.with_corner_radius_value(radius).is_none() {
            return Err(Error::Unsupported);
        }
        if matches!(stroke.tool, Tool::Blur | Tool::Mosaic) {
            let strength = number(object.get("strength").ok_or(Error::Shape)?)?;
            if (stroke.tool == Tool::Blur && !(1.0..=64.0).contains(&strength))
                || (stroke.tool == Tool::Mosaic && (strength < 2.0 || strength.fract() != 0.0))
            {
                return Err(Error::Unsupported);
            }
        }
        Ok(())
    }
}
fn number(value: &Value) -> Result<f32, Error> {
    let value = value.as_f64().ok_or(Error::Geometry)? as f32;
    finite(value)?;
    Ok(value)
}
fn field(object: &mut Map<String, Value>, key: &str, scale: f32, offset: f32) -> Result<(), Error> {
    if let Some(value) = object.get_mut(key) {
        *value = finite(number(value)? * scale + offset)?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "session_vector_write_mapping_tests.rs"]
mod tests;
