//! 普通/Live 外框与合成预览共用的物理宽度、预乘白色 alpha；不改变文档像素。
pub(crate) fn width(dpi: u32) -> u32 {
    (2 * dpi.clamp(96, 768) + 48) / 96
}
pub(crate) fn alpha(opacity: f32) -> u8 {
    ((0.5 + opacity * 0.5) * 255.0).round() as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn border_opacity_has_half_strength_floor_and_tracks_current_image() {
        for (image, expected) in [(0.0, 128), (0.3, 166), (0.5, 191), (1.0, 255)] {
            assert_eq!(alpha(image), expected);
        }
        assert_eq!(width(96), 2);
        assert_eq!(width(144), 3);
        assert_eq!(width(768), 16);
    }
}
