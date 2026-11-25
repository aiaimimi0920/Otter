//! 候选区附着于右侧显式输出之后；几何与真实 strip HRGN、同批移动共用。
use super::*;
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-save")))]
#[path = "workspace_candidate_auxiliary_tests.rs"]
mod tests;
pub(super) fn capture(app: &Application, nodes: &[Node]) -> Vec<(usize, usize)> {
    nodes
        .iter()
        .filter(|n| n.expanded && n.interactive && !n.minified)
        .filter_map(|n| {
            let doc = app.documents.iter().find(|d| {
                d.window.0 .0 as isize == n.hwnd
                    && d.state.is_art()
                    && d.state.local_art_id.is_none()
            })?;
            let stamp = desktop::configuration_stamp::Stamp::capture(&doc.state);
            let results = doc.state.execution_results.borrow();
            let set = results.candidate_view(&stamp)?;
            Some((n.endpoint, set.items.len()))
        })
        .collect()
}
pub(super) unsafe fn cancel(ui: &Ui) {
    let rails = ui.rails.borrow().clone();
    for (_, panel) in rails {
        panel.hide_auxiliary();
    }
}
pub(super) fn layout(ui: &Ui, node: &Node) -> Option<RECT> {
    if !node.expanded || !node.interactive || node.minified {
        return None;
    }
    let count = ui
        .auxiliary_counts
        .borrow()
        .iter()
        .find(|(n, _)| *n == node.endpoint)?
        .1;
    let grid = neuro_mini_ui::PreviewGridLayout::new(count, node.dpi, 0.0);
    let p = |n: i32| (n * node.dpi.clamp(96, 768) as i32 + 48) / 96;
    let x = node.rect.right - p(14) + desktop::pin_border_style::width(node.dpi) as i32;
    let y = node.rect.top + p(22) + node.outputs as i32 * p(36);
    Some(RECT {
        left: x,
        top: y,
        right: x + grid.width,
        bottom: y + grid.height,
    })
}
pub(super) unsafe fn place(
    ui: &Ui,
    endpoint: usize,
    panel: &desktop::document_window::parameter_panel::Inline,
) -> Result<()> {
    let Some(node) = ui.node(endpoint) else {
        return Ok(());
    };
    let parent = ui
        .input_windows
        .borrow()
        .iter()
        .find(|(_, n, s)| *n == endpoint && *s == Side::Output)
        .map(|(h, _, _)| *h);
    let Some(parent) = parent else {
        return Ok(());
    };
    let mut bounds = RECT::default();
    GetWindowRect(parent, &mut bounds)?;
    panel.place_auxiliary(parent, bounds, layout(ui, &node), node.dpi)
}
