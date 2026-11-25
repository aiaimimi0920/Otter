//! Cross-node ranges follow TextFrame scene preorder without screen-coordinate sorting or aggregated clipboard text.
use super::*;
/// Resolve endpoints once per draw; no cache across layout commits, text ownership, or per-node array allocation.
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
    // Retain the old algorithm as a test oracle; production per-item painting no longer performs three linear searches.
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
        // Appearance/disappearance of unselectable paint items such as image alt text does not change the user's selected text range.
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
