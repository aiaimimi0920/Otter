//! 一个场景仅持有一个文字选区；索引借用现有布局，换场景通过 Presentation 替换撤销。
use super::*;
use crate::surface_native_measure::TextItem;
use std::ops::Range;
use windows::Win32::Graphics::Direct2D::{Common::*, *};
#[path = "surface_selection_autoscroll.rs"]
mod autoscroll;
#[path = "surface_selection_cross_node.rs"]
mod cross_node;
#[path = "surface_selection_keyboard.rs"]
mod keyboard;
#[path = "surface_selection_reveal.rs"]
mod reveal;
#[path = "surface_selection_word.rs"]
mod word;

pub(crate) struct Selection {
    node: String,
    focus_node: String,
    anchor: u32,
    focus: u32,
    dragging: bool,
    vertical_point: Option<[f32; 2]>,
    word: Option<word::WordDrag>,
}
impl Selection {
    fn range(&self) -> Range<u32> {
        self.anchor.min(self.focus)..self.anchor.max(self.focus)
    }
}
impl cross_node::ProjectedSelection {
    pub(crate) fn draw(
        &self,
        item: &TextItem,
        index: usize,
        target: &ID2D1RenderTarget,
        brush: &ID2D1SolidColorBrush,
    ) -> Result<()> {
        let Some(range) = self
            .item_range(index, item)
            .filter(|range| !range.is_empty())
        else {
            return Ok(());
        };
        let rects = item.selection_rects(range)?;
        unsafe {
            let previous = brush.GetColor();
            // 与现有焦点信号黄同源，按前端选区合同派生 42% 透明度，不替换字色。
            brush.SetColor(&D2D1_COLOR_F {
                r: 217. / 255.,
                g: 1.,
                b: 56. / 255.,
                a: 0.42,
            });
            for rect in rects {
                target.FillRectangle(
                    &D2D_RECT_F {
                        left: rect.left,
                        top: rect.top,
                        right: rect.right,
                        bottom: rect.bottom,
                    },
                    brush,
                );
            }
            brush.SetColor(&previous);
        }
        Ok(())
    }
}
impl Presentation {
    pub(super) fn reconcile_text_selection(&mut self, next: &TextFrame) {
        let Some(selection) = self.selection.as_ref() else {
            return;
        };
        if selection.cross_node() {
            if !self.cross_selection_valid(selection, next) {
                self.selection = None;
            }
            return;
        }
        let old = self
            .text
            .items()
            .iter()
            .find(|item| item.node_id() == selection.node);
        let new = next
            .items()
            .iter()
            .find(|item| item.node_id() == selection.node && item.selectable());
        if !old
            .zip(new)
            .is_some_and(|(old, new)| old.layout().same_text(new.layout()))
        {
            self.selection = None;
        }
    }
    pub fn begin_text_selection(&mut self, point: [f32; 2]) -> Result<bool> {
        self.selection = None;
        let Some((id, position)) = self.text_selection_hit(point)? else {
            return Ok(false);
        };
        let caret = self
            .text
            .items()
            .iter()
            .find(|item| item.node_id() == id)
            .unwrap()
            .selection_caret(point)?;
        self.selection = Some(Selection {
            node: id.to_owned(),
            focus_node: id.to_owned(),
            anchor: position,
            focus: position,
            dragging: true,
            vertical_point: Some(caret),
            word: None,
        });
        Ok(true)
    }
    pub fn selectable_text_at(&self, point: [f32; 2]) -> Result<bool> {
        Ok(self.text_selection_hit(point)?.is_some())
    }
    fn text_selection_hit(&self, point: [f32; 2]) -> Result<Option<(&str, u32)>> {
        // 使用正式最上层命中；不能绕过遮挡、圆角或滚动条独占输入。
        let Some(id) = self.hit_test(point) else {
            return Ok(None);
        };
        let Some(item) = self.text.items().iter().find(|item| item.node_id() == id) else {
            return Ok(None);
        };
        let Some(position) = item.selection_start(point)? else {
            return Ok(None);
        };
        Ok(Some((id, position)))
    }
    pub fn text_selection_dragging(&self) -> bool {
        self.selection.as_ref().is_some_and(|s| s.dragging)
    }
    pub fn text_selection_keyboard_ready(&self, node: &str, extend: bool) -> bool {
        self.selection.as_ref().is_some_and(|s| {
            s.node == node && !s.dragging && (extend || (!s.cross_node() && s.anchor != s.focus))
        })
    }
    /// 保留当前导航锚点；只移动选区 focus，端点处可进入相邻可选择文字。
    pub fn step_text_selection(&mut self, node: &str, forward: bool, extend: bool) -> Result<bool> {
        if !self.text_selection_keyboard_ready(node, extend) {
            return Ok(false);
        }
        let selection = self.selection.as_mut().unwrap();
        let Some(item) = self
            .text
            .items()
            .iter()
            .find(|item| item.node_id() == selection.focus_node && item.selectable())
        else {
            self.selection = None;
            return Ok(false);
        };
        if extend {
            let next = item.layout().step_text(selection.focus, forward)?;
            if next != selection.focus {
                selection.focus = next;
            } else {
                keyboard::adjacent(selection, &self.text, forward);
            }
        } else {
            let range = selection.range();
            selection.focus = if forward { range.end } else { range.start };
            selection.anchor = selection.focus;
        }
        selection.vertical_point = None;
        Ok(true)
    }
    pub fn text_selection_vertical_ready(&self, node: &str) -> bool {
        self.text_selection_keyboard_ready(node, true)
    }
    pub fn vertical_text_selection(&mut self, node: &str, down: bool) -> Result<bool> {
        if !self.text_selection_vertical_ready(node) {
            return Ok(false);
        }
        let selection = self.selection.as_mut().unwrap();
        keyboard::vertical(selection, &self.text, down)?;
        Ok(true)
    }
    pub(super) fn reset_text_navigation(&mut self) {
        if let Some(selection) = &mut self.selection {
            selection.vertical_point = None;
        }
    }
    pub fn line_edge_text_selection(&mut self, node: &str, end: bool) -> Result<bool> {
        if !self.text_selection_keyboard_ready(node, true) {
            return Ok(false);
        }
        let selection = self.selection.as_mut().unwrap();
        let Some(item) = self
            .text
            .items()
            .iter()
            .find(|item| item.node_id() == selection.focus_node && item.selectable())
        else {
            self.selection = None;
            return Ok(false);
        };
        let (focus, point) =
            item.layout()
                .line_edge(selection.focus, end, selection.vertical_point)?;
        selection.focus = focus;
        selection.vertical_point = Some(point);
        Ok(true)
    }
    pub fn update_text_selection(&mut self, point: [f32; 2]) -> Result<bool> {
        let hit = self.text_selection_hit(point)?.and_then(|(node, _)| {
            self.selection
                .as_ref()
                .filter(|s| s.dragging && s.focus_node != node)
                .map(|_| node.to_owned())
        });
        let Some(selection) = self.selection.as_mut().filter(|s| s.dragging) else {
            return Ok(false);
        };
        if let Some(node) = hit {
            selection.focus_node = node;
        }
        let Some(item) = self
            .text
            .items()
            .iter()
            .find(|item| item.node_id() == selection.focus_node && item.selectable())
        else {
            self.selection = None;
            return Ok(false);
        };
        let focus = item.selection_position(point)?;
        let caret = item.selection_caret(point)?;
        word::extend(selection, item, focus, &self.text, point)?;
        selection.vertical_point = Some(caret);
        Ok(true)
    }
    /// 抬起先完成选区，再释放 HWND 捕获，避免同步 WM_CAPTURECHANGED 清掉已完成选择。
    pub fn finish_text_selection(&mut self, point: [f32; 2]) -> Result<bool> {
        if !self.update_text_selection(point)? {
            return Ok(false);
        }
        let selection = self.selection.as_mut().unwrap();
        selection.dragging = false;
        selection.word = None;
        Ok(selection.cross_node() || selection.anchor != selection.focus)
    }
    pub fn clear_text_selection(&mut self) {
        self.selection = None;
    }
    pub fn cancel_text_drag(&mut self) {
        if self.text_selection_dragging() {
            self.selection = None;
        }
    }
    pub fn selected_text(&self) -> Result<Option<String>> {
        let Some(selection) = self
            .selection
            .as_ref()
            .filter(|s| !s.cross_node() && s.anchor != s.focus)
        else {
            return Ok(None);
        };
        self.text
            .items()
            .iter()
            .find(|item| item.node_id() == selection.node && item.selectable())
            .map(|item| item.layout().selected_text(selection.range()))
            .transpose()
    }
}
#[cfg(test)]
#[path = "surface_presentation_selection_tests.rs"]
mod tests;
