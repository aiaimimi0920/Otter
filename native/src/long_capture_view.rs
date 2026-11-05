//! 经边界验证的只读帧视图，匹配裁剪区域时不复制像素、不新增像素租约。
use super::{Axis, Error, Frame};
#[derive(Clone, Copy)]
pub(super) struct View<'a> {
    frame: &'a Frame,
    left: u32,
    top: u32,
    pub width: u32,
    pub height: u32,
}
impl<'a> View<'a> {
    pub fn new(
        frame: &'a Frame,
        left: u32,
        top: u32,
        width: u32,
        height: u32,
    ) -> Result<Self, Error> {
        if width == 0
            || height == 0
            || left.checked_add(width).is_none_or(|n| n > frame.width)
            || top.checked_add(height).is_none_or(|n| n > frame.height)
        {
            return Err(Error::Dimensions);
        }
        Ok(Self {
            frame,
            left,
            top,
            width,
            height,
        })
    }
    pub fn dimensions(self, axis: Axis) -> (u32, u32) {
        match axis {
            Axis::Vertical => (self.height, self.width),
            Axis::Horizontal => (self.width, self.height),
        }
    }
    pub fn pixel(self, axis: Axis, along: u32, cross: u32) -> &'a [u8] {
        let (x, y) = match axis {
            Axis::Vertical => (cross, along),
            Axis::Horizontal => (along, cross),
        };
        let offset =
            ((self.top + y) as usize * self.frame.width as usize + (self.left + x) as usize) * 4;
        &self.frame.pixels[offset..offset + 4]
    }
}
