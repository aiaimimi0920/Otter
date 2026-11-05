//! 擦除手势只收集有界图像坐标；CPU 擦除在松手后的后台任务执行。
use super::canvas::Canvas;
use crate::{erase_job::Request, erase_layers::Mode, viewport::Point};
impl Canvas {
    pub(super) fn begin_erase(&mut self, point: Point) -> bool {
        let Some(point) = self.image_point(point, false) else {
            return false;
        };
        let Some(source) = self.editor.layers() else {
            return false;
        };
        if self.erase_annotations_only && source.annotations.is_none() {
            return false;
        }
        self.erase_gesture = Some(Request {
            preview_identity: std::sync::Arc::new(()),
            annotation_frame: self.editor.annotation_frame(),
            base_projection: self.editor.raster_base_projection(),
            source: source.clone(),
            revision: self.editor.revision(),
            mode: if self.erase_annotations_only {
                Mode::RasterAnnotations
            } else {
                Mode::Content
            },
            points: vec![point],
            width: self.editor.style().width().clamp(4.0, 96.0),
        });
        true
    }
    pub(super) fn extend_erase(&mut self, point: Point) -> bool {
        let point = self.viewport.image_point(point);
        if !point.x.is_finite()
            || !point.y.is_finite()
            || point.x.abs() > 1_000_000.0
            || point.y.abs() > 1_000_000.0
        {
            return false;
        }
        let Some(gesture) = self.erase_gesture.as_mut() else {
            return false;
        };
        if gesture.points.last() == Some(&point) {
            return false;
        }
        if gesture.points.len() >= 65_536 {
            self.erase_gesture = None;
            eprintln!("native-erase: point budget exceeded; gesture cancelled");
            return true;
        }
        gesture.points.push(point);
        true
    }
}

/// 未接入实时像素预览前仅显示尺寸环，不用覆盖色冒充透明擦除结果。
pub(super) unsafe fn draw_cursor(
    target: &windows::Win32::Graphics::Direct2D::ID2D1RenderTarget,
    brush: &windows::Win32::Graphics::Direct2D::ID2D1SolidColorBrush,
    gesture: Option<&Request>,
    viewport: crate::viewport::Viewport,
) {
    use windows::Win32::Graphics::Direct2D::{Common::*, D2D1_ELLIPSE};
    let Some(gesture) = gesture else {
        return;
    };
    let Some(point) = gesture.points.last() else {
        return;
    };
    let ellipse = D2D1_ELLIPSE {
        point: D2D_POINT_2F {
            x: viewport.origin.x + point.x * viewport.scale,
            y: viewport.origin.y + point.y * viewport.scale,
        },
        radiusX: gesture.width * viewport.scale / 2.0,
        radiusY: gesture.width * viewport.scale / 2.0,
    };
    let previous = brush.GetColor();
    brush.SetColor(&D2D1_COLOR_F {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    });
    target.DrawEllipse(&ellipse, brush, 3.0, None);
    brush.SetColor(&D2D1_COLOR_F {
        r: 1.0,
        g: 1.0,
        b: 1.0,
        a: 1.0,
    });
    target.DrawEllipse(&ellipse, brush, 1.0, None);
    brush.SetColor(&previous);
}
