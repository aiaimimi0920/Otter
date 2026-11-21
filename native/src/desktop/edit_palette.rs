//! 调色板是应用会话状态；跨钉图共享，不新增磁盘持久化。删除只清工具默认值。
use super::{canvas::Canvas, State};
use crate::{stroke_style::Color, workspace_id::UnitId};
use neuro_mini_ui::Rgba;
use std::{cell::RefCell, rc::Rc};
pub(super) type Shared = Rc<RefCell<Palette>>;
pub(super) struct Reset {
    pub origin: UnitId,
    pub removed: Vec<Rgba>,
}
pub(super) struct Palette {
    pub colors: Vec<Rgba>,
    pub active: Rgba,
    pub sampled: Option<Color>,
    pending: Option<Reset>,
}
impl Default for Palette {
    fn default() -> Self {
        Self {
            colors: [
                "#00000000",
                "#ffffff",
                "#000000",
                "#ef4444",
                "#f59e0b",
                "#eab308",
                "#22c55e",
                "#06b6d4",
                "#3b82f6",
                "#8b5cf6",
            ]
            .map(|s| Rgba::parse(s).unwrap())
            .to_vec(),
            active: Rgba::rgb(239, 68, 68),
            sampled: None,
            pending: None,
        }
    }
}
impl Palette {
    pub(super) fn record(&mut self, state: &State, colors: Vec<Rgba>, removed: Vec<Rgba>) {
        self.colors = colors;
        if removed.is_empty() {
            return;
        }
        // 全应用的模态互斥保证一次仅有一个面板；外层消息结算前不会再进入另一个面板。
        assert!(
            self.pending.is_none(),
            "Palette reset must settle before next modal"
        );
        reset(&mut state.canvas.borrow_mut(), &removed);
        self.pending = Some(Reset {
            origin: state.unit_id.clone(),
            removed,
        });
    }
    pub(super) fn take_reset(&mut self) -> Option<Reset> {
        self.pending.take()
    }
}
pub(super) fn reset(c: &mut Canvas, removed: &[Rgba]) {
    let mut style = c.editor.style();
    for color in removed {
        style = c.tool_profiles.remove_palette_color(
            c.tool,
            style,
            Color(color.r, color.g, color.b),
            color.a,
        );
    }
    c.editor.set_style(style);
}
