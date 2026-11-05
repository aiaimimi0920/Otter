//! 标记、标签、快捷键和箭头各自裁剪；窄面板不会让快捷键覆盖标签。
use super::*;
pub(super) struct Columns {
    pub mark: RECT,
    pub label: RECT,
    pub shortcut: RECT,
    pub arrow: RECT,
}
pub(super) fn columns(rect: RECT, dpi: u32, shortcut: i32) -> Columns {
    let padding = font::scaled(8, dpi).min((rect.right - rect.left).max(0) / 2);
    let left = rect.left + padding;
    let right = rect.right - padding;
    let mark = RECT {
        left,
        right: (left + font::scaled(22, dpi)).min(right),
        ..rect
    };
    let arrow = RECT {
        left: (right - font::scaled(18, dpi)).max(mark.right),
        right,
        ..rect
    };
    let gap = font::scaled(8, dpi);
    let available = (arrow.left - mark.right - gap).max(0);
    let width = shortcut.max(0).min(available / 2);
    let shortcut = RECT {
        left: arrow.left - width,
        right: arrow.left,
        ..rect
    };
    let label = RECT {
        left: mark.right,
        right: (shortcut.left - if width > 0 { gap } else { 0 }).max(mark.right),
        ..rect
    };
    Columns {
        mark,
        label,
        shortcut,
        arrow,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn narrow_and_scaled_columns_remain_ordered_and_inside_row() {
        for dpi in [96, 144, 192, 288] {
            for width in [1, 10, 40, 120, 320, 960] {
                let rect = RECT {
                    left: 0,
                    top: 0,
                    right: width,
                    bottom: font::scaled(28, dpi),
                };
                let layout = columns(rect, dpi, 9999);
                let all = [layout.mark, layout.label, layout.shortcut, layout.arrow];
                for item in all {
                    assert!(item.left >= 0 && item.left <= item.right && item.right <= width);
                }
                for pair in all.windows(2) {
                    assert!(pair[0].right <= pair[1].left);
                }
            }
        }
    }
}
