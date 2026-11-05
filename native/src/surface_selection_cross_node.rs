//! 跨节点范围按 TextFrame 的场景先序投影；不按屏幕坐标排序，不聚合剪贴板文本。
use super::*;
/// 每次 draw 只解析一次端点；不跨布局提交缓存，不拥有文字或分配逐节点数组。
#[derive(Clone, Copy)]
pub(crate) struct ProjectedSelection {
    low: usize,
    high: usize,
    start: u32,
    end: u32,
}
impl ProjectedSelection {
    pub(crate) fn item_range(&self, index: usize, item: &TextItem) -> Option<Range<u32>> {
        if !item.selectable() || index < self.low || index > self.high {
            return None;
        }
        Some(
            (if index == self.low { self.start } else { 0 })..(if index == self.high {
                self.end
            } else {
                item.layout().text_len()
            }),
        )
    }
}
impl Selection {
    pub(crate) fn project(&self, text: &TextFrame) -> Option<ProjectedSelection> {
        let a = text
            .items()
            .iter()
            .position(|item| item.node_id() == self.node)?;
        if !self.cross_node() {
            let range = self.range();
            return Some(ProjectedSelection {
                low: a,
                high: a,
                start: range.start,
                end: range.end,
            });
        }
        let b = text
            .items()
            .iter()
            .position(|item| item.node_id() == self.focus_node)?;
        let (low, start, high, end) = if a < b {
            (a, self.anchor, b, self.focus)
        } else {
            (b, self.focus, a, self.anchor)
        };
        Some(ProjectedSelection {
            low,
            high,
            start,
            end,
        })
    }
    pub(super) fn cross_node(&self) -> bool {
        self.node != self.focus_node
    }
    pub(super) fn forward(&self, text: &TextFrame) -> bool {
        if !self.cross_node() {
            return self.focus >= self.anchor;
        }
        let a = text
            .items()
            .iter()
            .position(|item| item.node_id() == self.node);
        let b = text
            .items()
            .iter()
            .position(|item| item.node_id() == self.focus_node);
        a.zip(b).is_some_and(|(a, b)| a < b)
    }
    // 保留旧算法作为测试 oracle；生产逐项绘制不再做三次线性查找。
    #[cfg(test)]
    pub(super) fn item_range(&self, item: &TextItem, text: &TextFrame) -> Option<Range<u32>> {
        if !item.selectable() {
            return None;
        }
        if !self.cross_node() {
            return (item.node_id() == self.node).then(|| self.range());
        }
        let items = text.items();
        let a = items.iter().position(|item| item.node_id() == self.node)?;
        let b = items
            .iter()
            .position(|item| item.node_id() == self.focus_node)?;
        let i = items
            .iter()
            .position(|candidate| candidate.node_id() == item.node_id())?;
        let (low, start, high, end) = if a < b {
            (a, self.anchor, b, self.focus)
        } else {
            (b, self.focus, a, self.anchor)
        };
        if i < low || i > high {
            return None;
        }
        Some(
            (if i == low { start } else { 0 })..(if i == high {
                end
            } else {
                item.layout().text_len()
            }),
        )
    }
}
impl Presentation {
    pub(super) fn cross_selection_valid(&self, selection: &Selection, next: &TextFrame) -> bool {
        let extent = |text: &TextFrame| {
            let a = text
                .items()
                .iter()
                .position(|item| item.node_id() == selection.node && item.selectable())?;
            let b = text
                .items()
                .iter()
                .position(|item| item.node_id() == selection.focus_node && item.selectable())?;
            Some(a.min(b)..a.max(b) + 1)
        };
        let Some((old, new)) = extent(&self.text).zip(extent(next)) else {
            return false;
        };
        // 图片 alt 等不可选择绘制项的出现/消失不改变用户选中的文本范围。
        let mut old = self.text.items()[old]
            .iter()
            .filter(|item| item.selectable());
        let mut new = next.items()[new].iter().filter(|item| item.selectable());
        loop {
            match (old.next(), new.next()) {
                (None, None) => return true,
                (Some(a), Some(b))
                    if a.node_id() == b.node_id() && a.layout().same_text(b.layout()) => {}
                _ => return false,
            }
        }
    }
}
