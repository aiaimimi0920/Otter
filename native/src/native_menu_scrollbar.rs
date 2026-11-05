//! 菜单按整行滚动；同一份客户端像素几何用于绘制、轨道命中和绝对拖动。
use windows::Win32::Foundation::RECT;
#[derive(Clone, Copy)]
pub(crate) struct Bar {
    pub track: RECT,
    pub thumb: RECT,
    max: usize,
    travel: i32,
}
impl Bar {
    pub(crate) fn new(
        width: i32,
        height: i32,
        row: i32,
        total: usize,
        visible: usize,
        first: usize,
    ) -> Option<Self> {
        if width <= 0 || height <= 0 || row <= 0 || visible == 0 || total <= visible || total > 1024
        {
            return None;
        }
        let max = total - visible;
        let length = ((i64::from(height) * visible as i64 / total as i64) as i32)
            .max((row / 2).max(1))
            .min(height);
        let travel = height - length;
        let top = (i64::from(travel) * first.min(max) as i64 / max as i64) as i32;
        let track = RECT {
            left: width - (row / 3).clamp(6, 24).min(width),
            top: 0,
            right: width,
            bottom: height,
        };
        Some(Self {
            track,
            thumb: RECT {
                top,
                bottom: top + length,
                ..track
            },
            max,
            travel,
        })
    }
    pub(crate) fn hit(&self, x: i32, y: i32) -> bool {
        x >= self.track.left && x < self.track.right && y >= 0 && y < self.track.bottom
    }
    pub(crate) fn in_thumb(&self, y: i32) -> bool {
        y >= self.thumb.top && y < self.thumb.bottom
    }
    pub(crate) fn drag(&self, start_y: i32, first: usize, y: i32) -> usize {
        if self.travel <= 0 {
            return first.min(self.max);
        }
        (first as i64
            + (i64::from(y) - i64::from(start_y)) * self.max as i64 / i64::from(self.travel))
        .clamp(0, self.max as i64) as usize
    }
}
