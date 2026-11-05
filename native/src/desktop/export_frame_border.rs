//! 工作流外框在底图之后、独立标注之前绘制；不继承底图透明度。
use super::*;

pub(in crate::desktop) struct Border {
    width: f32,
    color: D2D1_COLOR_F,
}
impl Border {
    pub(in crate::desktop) fn read(edit: &serde_json::Value) -> Result<Option<Self>> {
        let width = match edit.get("borderWidth") {
            None | Some(serde_json::Value::Null) => 0.0,
            Some(value) => value
                .as_f64()
                .ok_or_else(|| Error::new(E_INVALIDARG, "Invalid workflow border width"))?,
        };
        if !width.is_finite() || !(0.0..=8192.0).contains(&width) {
            return Err(Error::new(E_INVALIDARG, "Invalid workflow border width"));
        }
        let color = match edit.get("borderColor") {
            None | Some(serde_json::Value::Null) => "",
            Some(value) => value
                .as_str()
                .ok_or_else(|| Error::new(E_INVALIDARG, "Invalid workflow border color"))?,
        };
        if width == 0.0 || color.is_empty() {
            return Ok(None);
        }
        let crate::surface_color::Color([r, g, b, a]) =
            crate::surface_color::Color::parse(color, crate::surface_color::Color::BLACK)
                .ok_or_else(|| Error::new(E_INVALIDARG, "Unsupported workflow border color"))?;
        Ok(Some(Self {
            width: width as f32,
            color: D2D1_COLOR_F { r, g, b, a },
        }))
    }

    pub(in crate::desktop::export) unsafe fn draw(
        &self,
        target: &ID2D1RenderTarget,
        frame: crate::crop_geometry::Rect,
    ) -> Result<()> {
        let brush = target.CreateSolidColorBrush(&self.color, None)?;
        let inset = self.width / 2.0;
        let left = frame.x as f32 + inset;
        let top = frame.y as f32 + inset;
        let rect = D2D_RECT_F {
            left,
            top,
            right: left + (frame.width as f32 - self.width).max(0.0),
            bottom: top + (frame.height as f32 - self.width).max(0.0),
        };
        target.DrawRectangle(&rect, &brush, self.width, None);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn border_input_is_bounded_and_missing_style_is_noop() {
        for edit in [
            json!({}),
            json!({"borderColor":"red"}),
            json!({"borderWidth":2}),
        ] {
            assert!(Border::read(&edit).unwrap().is_none());
        }
        for edit in [
            json!({"borderWidth":-1}),
            json!({"borderWidth":8193}),
            json!({"borderWidth":"2"}),
            json!({"borderWidth":2,"borderColor":"url(secret)"}),
        ] {
            assert!(Border::read(&edit).is_err());
        }
        assert!(
            Border::read(&json!({"borderWidth":2,"borderColor":"#1238"}))
                .unwrap()
                .is_some()
        );
    }
}
