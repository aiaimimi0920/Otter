//! 用户确认的窗口 frame 局部像素；捕获和输入共用同一裁剪，不以屏幕位置重新绑定来源。
use crate::capture_readback::Region;
use windows::{
    core::{Error, Result},
    Win32::Foundation::*,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Crop(Region);
impl Crop {
    pub fn from_screen(frame: RECT, screen: RECT) -> Result<Self> {
        if screen.left < frame.left
            || screen.top < frame.top
            || screen.right > frame.right
            || screen.bottom > frame.bottom
            || screen.right <= screen.left
            || screen.bottom <= screen.top
        {
            return Err(Error::new(
                E_INVALIDARG,
                "Live crop outside selected window",
            ));
        }
        let region = Region {
            left: (i64::from(screen.left) - i64::from(frame.left)) as u32,
            top: (i64::from(screen.top) - i64::from(frame.top)) as u32,
            width: (i64::from(screen.right) - i64::from(screen.left)) as u32,
            height: (i64::from(screen.bottom) - i64::from(screen.top)) as u32,
        };
        crate::viewport::checked_image_bytes(region.width, region.height)
            .ok_or_else(|| Error::new(E_INVALIDARG, "Live crop exceeds pixel budget"))?;
        let crop = Self(region);
        crop.screen(frame)?;
        Ok(crop)
    }
    pub fn pixels(self) -> Region {
        self.0
    }
    pub fn input(self) -> super::input_coordinates::FrameRegion {
        super::input_coordinates::FrameRegion {
            left: self.0.left,
            top: self.0.top,
            width: self.0.width,
            height: self.0.height,
        }
    }
    /// 固定局部像素跟随移动，不随窗口缩放而拉伸；缩小到选区之外时明确拒绝。
    pub fn screen(self, frame: RECT) -> Result<RECT> {
        let width = i64::from(frame.right) - i64::from(frame.left);
        let height = i64::from(frame.bottom) - i64::from(frame.top);
        let right = i64::from(self.0.left) + i64::from(self.0.width);
        let bottom = i64::from(self.0.top) + i64::from(self.0.height);
        if width <= 0 || height <= 0 || right > width || bottom > height {
            return Err(Error::new(E_ABORT, "Live window no longer contains crop"));
        }
        Ok(RECT {
            left: (i64::from(frame.left) + i64::from(self.0.left)) as i32,
            top: (i64::from(frame.top) + i64::from(self.0.top)) as i32,
            right: (i64::from(frame.left) + right) as i32,
            bottom: (i64::from(frame.top) + bottom) as i32,
        })
    }
    /// DWM frame 与 WGC 内容必须一一对应，不能猜阴影偏移或按比例纠正。
    /// resize 期间的旧帧可跳过；不包含裁剪的真实新边界则返回错误。
    pub fn matches_texture(self, frame: RECT, size: [u32; 2]) -> Result<bool> {
        self.screen(frame)?;
        Ok(
            i64::from(frame.right) - i64::from(frame.left) == i64::from(size[0])
                && i64::from(frame.bottom) - i64::from(frame.top) == i64::from(size[1]),
        )
    }
}

#[cfg(test)]
#[path = "live_window_region_tests.rs"]
mod tests;
