//! 视口变换使用逻辑像素；图像坐标不受窗口 DPI 或平移影响。
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Viewport {
    pub origin: Point,
    pub scale: f32,
}

impl Default for Viewport {
    fn default() -> Self {
        Self {
            origin: Point { x: 20.0, y: 20.0 },
            scale: 1.0,
        }
    }
}

impl Viewport {
    pub fn image_point(&self, point: Point) -> Point {
        Point {
            x: (point.x - self.origin.x) / self.scale,
            y: (point.y - self.origin.y) / self.scale,
        }
    }

    pub fn zoom_at(&mut self, anchor: Point, wheel_steps: f32) -> bool {
        if !anchor.x.is_finite() || !anchor.y.is_finite() || !wheel_steps.is_finite() {
            return false;
        }
        let before = *self;
        let image = self.image_point(anchor);
        self.scale = (self.scale * 1.1_f32.powf(wheel_steps.clamp(-32.0, 32.0))).clamp(0.02, 64.0);
        self.origin = Point {
            x: anchor.x - image.x * self.scale,
            y: anchor.y - image.y * self.scale,
        };
        *self != before
    }

    pub fn pan(&mut self, delta: Point) {
        if delta.x.is_finite() && delta.y.is_finite() {
            self.origin.x += delta.x;
            self.origin.y += delta.y;
        }
    }
}

/// 在解码前检查尺寸，避免整数溢出或压缩炸弹的大额像素分配。
pub fn checked_image_bytes(width: u32, height: u32) -> Option<usize> {
    if width == 0 || height == 0 || width > 16384 || height > 16384 {
        return None;
    }
    let bytes = u64::from(width)
        .checked_mul(u64::from(height))?
        .checked_mul(4)?;
    (bytes <= 256 * 1024 * 1024).then_some(bytes as usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zoom_keeps_pointer_on_same_image_pixel() {
        let mut viewport = Viewport::default();
        let anchor = Point {
            x: -230.0,
            y: 400.0,
        };
        let before = viewport.image_point(anchor);
        assert!(viewport.zoom_at(anchor, 3.0));
        let after = viewport.image_point(anchor);
        assert!((before.x - after.x).abs() < 0.001);
        assert!((before.y - after.y).abs() < 0.001);
    }

    #[test]
    fn invalid_zoom_does_not_poison_geometry() {
        let mut viewport = Viewport::default();
        let before = viewport;
        assert!(!viewport.zoom_at(Point::default(), f32::NAN));
        assert_eq!(before, viewport);
        for _ in 0..100 {
            viewport.zoom_at(Point::default(), 32.0);
        }
        assert_eq!(viewport.scale, 64.0);
        for _ in 0..100 {
            viewport.zoom_at(Point::default(), -32.0);
        }
        assert_eq!(viewport.scale, 0.02);
    }

    #[test]
    fn image_admission_bounds_decode_allocation() {
        assert_eq!(checked_image_bytes(3840, 2160), Some(33_177_600));
        assert_eq!(checked_image_bytes(0, 1), None);
        assert_eq!(checked_image_bytes(u32::MAX, u32::MAX), None);
        assert_eq!(checked_image_bytes(16384, 16384), None);
        assert_eq!(checked_image_bytes(8192, 8192), Some(256 * 1024 * 1024));
    }
}
