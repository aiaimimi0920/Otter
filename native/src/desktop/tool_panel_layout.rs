//! 工具/操作分组共享的 DIP 布局；操作从下一整行开始，尺寸随目录数量推导。
use windows::Win32::Foundation::RECT;
const COLUMNS: usize = 5;
fn px(value: usize, dpi: u32) -> i32 {
    ((value as u64 * dpi.max(96) as u64 + 48) / 96).min(i32::MAX as u64) as i32
}
pub(super) fn client(tools: usize, actions: usize, dpi: u32) -> (i32, i32) {
    let rows = tools.div_ceil(COLUMNS) + actions.div_ceil(COLUMNS);
    (px(504, dpi), px(4 + rows * 35, dpi))
}
pub(super) fn button(index: usize, tools: usize, dpi: u32) -> RECT {
    let slot = if index < tools {
        index
    } else {
        tools.div_ceil(COLUMNS) * COLUMNS + index - tools
    };
    let (x, y) = (4 + slot % COLUMNS * 100, 4 + slot / COLUMNS * 35);
    RECT {
        left: px(x, dpi),
        top: px(y, dpi),
        right: px(x + 96, dpi),
        bottom: px(y + 31, dpi),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_controls_fit_without_overlap_and_actions_start_on_a_new_row() {
        for dpi in [96, 120, 144, 192, 240, 288] {
            let tools = crate::desktop::tool_commands::TOOLS.len();
            let actions = crate::desktop::tool_panel_actions::ACTIONS.len();
            let (width, height) = client(tools, actions, dpi);
            let rects: Vec<_> = (0..tools + actions)
                .map(|index| button(index, tools, dpi))
                .collect();
            for (index, rect) in rects.iter().enumerate() {
                assert!(
                    rect.left >= 0 && rect.top >= 0 && rect.right <= width && rect.bottom <= height
                );
                for other in &rects[..index] {
                    assert!(
                        rect.left >= other.right
                            || other.left >= rect.right
                            || rect.top >= other.bottom
                            || other.top >= rect.bottom
                    );
                }
            }
            assert!(rects[tools].top > rects[tools - 1].bottom);
            assert_eq!(rects[tools].left, rects[0].left);
        }
    }
}
