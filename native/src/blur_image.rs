//! 已完成的模糊图快照；标注、撤销和导出共享像素，不复制整图。
use crate::{pixels::Pixels, viewport::checked_image_bytes};
#[derive(Clone, Debug)]
pub struct BlurImage {
    width: u32,
    height: u32,
    pixels: Pixels,
    strength: f32,
    frame: crate::crop_geometry::Rect,
    effect_clip: Option<crate::crop_geometry::Rect>,
    effect_mask_scale: (f32, f32),
}

impl BlurImage {
    pub fn new(width: u32, height: u32, pixels: Pixels, sigma: u8) -> Option<Self> {
        Self::with_strength(width, height, pixels, sigma as f32)
    }
    pub fn with_strength(width: u32, height: u32, pixels: Pixels, strength: f32) -> Option<Self> {
        (checked_image_bytes(width, height) == Some(pixels.len())
            && strength.is_finite()
            && (1.0..=64.0).contains(&strength))
        .then_some(Self {
            width,
            height,
            pixels,
            strength,
            effect_clip: None,
            effect_mask_scale: (1.0, 1.0),
            frame: crate::crop_geometry::Rect {
                x: 0.0,
                y: 0.0,
                width: width as f64,
                height: height as f64,
            },
        })
    }
    pub fn sigma(&self) -> u8 {
        self.strength.round() as u8
    }
    /// 变换与重新计算必须使用未取整强度；sigma 仅用于既有整数控件。
    pub fn strength(&self) -> f32 {
        self.strength
    }
    pub fn width(&self) -> u32 {
        self.width
    }
    pub fn height(&self) -> u32 {
        self.height
    }
    pub fn pixels(&self) -> &Pixels {
        &self.pixels
    }
    /// 只改变纹理的逻辑采样框，像素仍共享；导出密度不应改变标注几何。
    pub fn with_frame(mut self, frame: crate::crop_geometry::Rect) -> Option<Self> {
        if ![frame.x, frame.y, frame.width, frame.height]
            .iter()
            .all(|v| v.is_finite())
            || frame.x.abs() > 1_000_000.0
            || frame.y.abs() > 1_000_000.0
            || frame.width <= 0.0
            || frame.height <= 0.0
            || frame.width > 1_000_000.0
            || frame.height > 1_000_000.0
            || (frame.width / self.width as f64) as f32 <= 0.0
            || (frame.height / self.height as f64) as f32 <= 0.0
        {
            return None;
        }
        self.frame = frame;
        Some(self)
    }
    pub fn frame(&self) -> crate::crop_geometry::Rect {
        self.frame
    }
    /// 局部导出纹理：frame 限制整个效果，clip 仅限制取样颜色、不限制白色覆盖。
    pub fn with_effect_clip(mut self, clip: crate::crop_geometry::Rect) -> Option<Self> {
        if ![clip.x, clip.y, clip.width, clip.height]
            .iter()
            .all(|v| v.is_finite())
            || clip.width < 0.0
            || clip.height < 0.0
            || clip.x.abs() > 2_000_000.0
            || clip.y.abs() > 2_000_000.0
            || clip.width > 1_000_000.0
            || clip.height > 1_000_000.0
        {
            return None;
        }
        self.effect_clip = Some(clip);
        Some(self)
    }
    pub fn effect_clip(&self) -> Option<crate::crop_geometry::Rect> {
        self.effect_clip
    }
    /// 局部 Canvas 取整后的内容（遮罩和 wash）与纹理一同压回持久框。
    pub fn with_effect_mask_scale(mut self, scale: (f32, f32)) -> Option<Self> {
        if self.effect_clip.is_none()
            || ![scale.0, scale.1]
                .iter()
                .all(|v| v.is_finite() && *v > 0.0 && *v <= 1.0)
        {
            return None;
        }
        self.effect_mask_scale = scale;
        Some(self)
    }
    pub fn effect_mask_scale(&self) -> (f32, f32) {
        self.effect_mask_scale
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn logical_frame_is_independent_from_shared_texture_dimensions() {
        let pixels: Pixels = vec![255; 8 * 4 * 4].into();
        let original = BlurImage::new(8, 4, pixels.clone(), 4).unwrap();
        let frame = crate::crop_geometry::Rect {
            x: -2.5,
            y: 3.5,
            width: 4.0,
            height: 8.0,
        };
        let mapped = original.clone().with_frame(frame).unwrap();
        assert_eq!(mapped.frame(), frame);
        assert_eq!((mapped.width(), mapped.height()), (8, 4));
        assert!(mapped.pixels().same_storage(&pixels));
        assert_eq!(original.frame().width, 8.0);
        for bad in [
            crate::crop_geometry::Rect {
                width: 0.0,
                ..frame
            },
            crate::crop_geometry::Rect {
                x: f64::NAN,
                ..frame
            },
            crate::crop_geometry::Rect {
                height: f64::INFINITY,
                ..frame
            },
        ] {
            assert!(original.clone().with_frame(bad).is_none());
        }
    }
    #[test]
    fn fractional_strength_is_preserved_and_invalid_values_are_rejected() {
        let pixels: Pixels = vec![255; 4].into();
        let image = BlurImage::with_strength(1, 1, pixels.clone(), 4.4).unwrap();
        assert_eq!(image.strength(), 4.4);
        assert_eq!(image.sigma(), 4);
        for strength in [f32::NAN, f32::INFINITY, 0.9, 64.1] {
            assert!(BlurImage::with_strength(1, 1, pixels.clone(), strength).is_none());
        }
    }
}
