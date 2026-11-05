//! 历史子菜单的屏幕坐标布局和自绘滚动输入；绘制和命中共享同一份几何。
use windows::Win32::Foundation::{POINT, RECT};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hit {
    Row(usize),
    Thumb,
    Track,
}

pub struct List {
    bounds: RECT,
    row: i32,
    count: usize,
    offset: i32,
    bar: i32,
    drag: Option<(i32, i32)>,
}

fn contains(r: RECT, p: POINT) -> bool {
    p.x >= r.left && p.x < r.right && p.y >= r.top && p.y < r.bottom
}

impl List {
    /// 所有参数均为同一 DPI 空间的物理像素；拒绝无效尺寸，不混用 client 坐标。
    pub fn place(anchor: RECT, work: RECT, width: i32, row: i32, count: usize) -> Option<Self> {
        let available_width = work.right.checked_sub(work.left)?;
        let available_height = work.bottom.checked_sub(work.top)?;
        if available_width <= 0
            || available_height <= 0
            || width <= 0
            || !(1..=1024).contains(&row)
            || count > 25
        {
            return None;
        }
        let width = width.min(available_width);
        let height = (count.max(1) as i32 * row)
            .min(8 * row)
            .min(available_height);
        let right_x = i64::from(anchor.right);
        let left_x = i64::from(anchor.left) - i64::from(width);
        let x = if right_x + i64::from(width) <= i64::from(work.right) {
            right_x
        } else {
            left_x
        };
        let x = x.clamp(i64::from(work.left), i64::from(work.right - width)) as i32;
        let y = anchor.top.clamp(work.top, work.bottom - height);
        Some(Self {
            bounds: RECT {
                left: x,
                top: y,
                right: x + width,
                bottom: y + height,
            },
            row,
            count,
            offset: 0,
            bar: (row / 3).clamp(1, 16).min(width),
            drag: None,
        })
    }

    pub fn offset(&self) -> i32 {
        self.offset
    }
    pub fn bounds(&self) -> RECT {
        self.bounds
    }
    fn page(&self) -> i32 {
        self.bounds.bottom - self.bounds.top
    }
    fn max(&self) -> i32 {
        (self.count as i32 * self.row - self.page()).max(0)
    }

    pub fn thumb(&self) -> Option<RECT> {
        if self.max() == 0 {
            return None;
        }
        let page = self.page();
        let height = (page * page / (self.count as i32 * self.row))
            .max(self.row.min(page / 2))
            .max(1);
        let top = self.bounds.top
            + (i64::from(page - height) * i64::from(self.offset) / i64::from(self.max())) as i32;
        Some(RECT {
            left: self.bounds.right - self.bar,
            right: self.bounds.right,
            top,
            bottom: top + height,
        })
    }

    /// 返回裁剪后的可见行；空状态占位不是可选条目。
    pub fn row_rect(&self, index: usize) -> Option<RECT> {
        if index >= self.count {
            return None;
        }
        let top = i64::from(self.bounds.top) + index as i64 * i64::from(self.row)
            - i64::from(self.offset);
        let bottom = (top + i64::from(self.row)).min(i64::from(self.bounds.bottom));
        let top = top.max(i64::from(self.bounds.top));
        if top >= bottom {
            return None;
        }
        Some(RECT {
            left: self.bounds.left,
            top: top as i32,
            bottom: bottom as i32,
            right: self.bounds.right - if self.max() > 0 { self.bar } else { 0 },
        })
    }

    pub fn hit(&self, p: POINT) -> Option<Hit> {
        if !contains(self.bounds, p) {
            return None;
        }
        if let Some(thumb) = self.thumb() {
            if p.x >= thumb.left {
                return Some(if contains(thumb, p) {
                    Hit::Thumb
                } else {
                    Hit::Track
                });
            }
        }
        let index = ((p.y - self.bounds.top + self.offset) / self.row) as usize;
        (index < self.count).then_some(Hit::Row(index))
    }

    /// 上层按系统滚轮设置换算像素；这里统一处理限位和命中偏移。
    pub fn scroll(&mut self, pixels: i32) -> bool {
        self.set_offset(i64::from(self.offset) + i64::from(pixels))
    }
    fn set_offset(&mut self, value: i64) -> bool {
        let next = value.clamp(0, i64::from(self.max())) as i32;
        let changed = next != self.offset;
        self.offset = next;
        changed
    }
    pub fn press(&mut self, p: POINT) -> bool {
        match self.hit(p) {
            Some(Hit::Thumb) => {
                self.drag = Some((p.y, self.offset));
                true
            }
            Some(Hit::Track) => {
                let direction = if p.y < self.thumb().unwrap().top {
                    -1
                } else {
                    1
                };
                self.scroll(direction * self.page())
            }
            _ => false,
        }
    }
    pub fn drag(&mut self, p: POINT) -> bool {
        let Some((start, offset)) = self.drag else {
            return false;
        };
        let Some(thumb) = self.thumb() else {
            return false;
        };
        let travel = self.page() - (thumb.bottom - thumb.top);
        if travel <= 0 {
            return false;
        }
        self.set_offset(
            i64::from(offset)
                + (i64::from(p.y) - i64::from(start)) * i64::from(self.max()) / i64::from(travel),
        )
    }
    pub fn cancel_drag(&mut self) {
        self.drag = None;
    }
}

#[cfg(test)]
#[path = "clipboard_history_list_tests.rs"]
mod tests;
