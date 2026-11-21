//! 列表与三列卡片共享的像素布局；绘制、命中、子菜单锚点和 UIA 使用相同矩形。
use windows::Win32::Foundation::RECT;

#[derive(Clone, Copy)]
pub(super) struct Layout {
    pub columns: usize,
    pub row: i32,
}
impl Layout {
    pub fn delete_rect(self, slot: usize, width: i32) -> RECT {
        let card = self.rect(slot, width);
        let inset = (self.row / 20).max(1);
        let size = (self.row / 4)
            .max(1)
            .min((card.right - card.left - inset * 2).max(0));
        RECT {
            left: card.right - inset - size,
            right: card.right - inset,
            top: card.top + inset,
            bottom: card.top + inset + size,
        }
    }
    pub fn align(self, index: usize) -> usize {
        index / self.columns * self.columns
    }
    pub fn rect(self, slot: usize, width: i32) -> RECT {
        let column = (slot % self.columns) as i32;
        let top = (slot / self.columns) as i32 * self.row;
        let columns = self.columns as i32;
        let gap = if self.columns == 1 {
            0
        } else {
            (self.row / 16).max(1)
        };
        RECT {
            left: width * column / columns + gap,
            right: (width * (column + 1) / columns - gap).max(width * column / columns + gap),
            top: top + gap,
            bottom: top + self.row - gap,
        }
    }
    pub fn hit(self, x: i32, y: i32, width: i32, count: usize) -> Option<usize> {
        if x < 0 || x >= width || y < 0 || width <= 0 {
            return None;
        }
        let slot = y as usize / self.row as usize * self.columns
            + ((x as i64 * self.columns as i64 / i64::from(width)) as usize);
        let rect = self.rect(slot, width);
        (slot < count && x >= rect.left && x < rect.right && y >= rect.top && y < rect.bottom)
            .then_some(slot)
    }
    pub fn max_first(self, count: usize, visible: usize) -> usize {
        count
            .div_ceil(self.columns)
            .saturating_sub(visible / self.columns)
            * self.columns
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn delete_button_stays_inside_the_selection_border_at_each_scale() {
        for dpi in [96, 120, 144, 192, 288] {
            let layout = Layout {
                columns: 3,
                row: 80 * dpi / 96,
            };
            let width = 248 * dpi / 96;
            let border = (2 * dpi + 48) / 96;
            for slot in 0..15 {
                let card = layout.rect(slot, width);
                let button = layout.delete_rect(slot, width);
                assert!(button.left >= card.left + border);
                assert!(button.top > card.top + border);
                assert!(button.right < card.right - border);
                assert!(button.bottom <= card.bottom - border);
                assert_eq!(button.right - button.left, layout.row / 4);
                assert_eq!(button.bottom - button.top, layout.row / 4);
                assert_eq!(layout.hit(button.left, button.top, width, 15), Some(slot));
            }
        }
    }
    #[test]
    fn three_cards_share_a_row_and_gaps_or_empty_last_slots_do_not_activate() {
        for scale in [1, 2, 3] {
            let layout = Layout {
                columns: 3,
                row: 80 * scale,
            };
            let width = 248 * scale;
            for slot in 0..7 {
                let rect = layout.rect(slot, width);
                assert_eq!(
                    layout.hit(
                        (rect.left + rect.right) / 2,
                        (rect.top + rect.bottom) / 2,
                        width,
                        7
                    ),
                    Some(slot)
                );
            }
            assert_eq!(layout.rect(0, width).top, layout.rect(2, width).top);
            assert!(layout.rect(3, width).top > layout.rect(2, width).bottom);
            assert_eq!(layout.hit(0, 10, width, 7), None);
            let missing = layout.rect(7, width);
            assert_eq!(
                layout.hit(missing.left + 1, missing.top + 1, width, 7),
                None
            );
            assert_eq!(layout.max_first(25, 12), 15);
            assert_eq!(layout.align(16), 15);
        }
    }
}
