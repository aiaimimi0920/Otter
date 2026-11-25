//! Keyboard extension enters adjacent selectable owners in scene order and converts column coordinates across lines; never leave the current Surface.
use super::*;
pub(super) fn adjacent(selection: &mut Selection, text: &TextFrame, forward: bool) {
    if let Some(item) = neighbor(text, &selection.focus_node, forward) {
        // This cross-node step only enters the endpoint without additionally skipping the target's first/last grapheme.
        selection.focus_node = item.node_id().to_owned();
        selection.focus = if forward { 0 } else { item.layout().text_len() };
    }
}
fn neighbor<'a>(text: &'a TextFrame, node: &str, forward: bool) -> Option<&'a TextItem> {
    let items = text.items();
    let index = items.iter().position(|item| item.node_id() == node)?;
    if forward {
        items[index + 1..].iter().find(|item| item.selectable())
    } else {
        items[..index].iter().rev().find(|item| item.selectable())
    }
}
pub(super) fn vertical(selection: &mut Selection, text: &TextFrame, down: bool) -> Result<()> {
    let item = text
        .items()
        .iter()
        .find(|item| item.node_id() == selection.focus_node)
        .ok_or_else(|| {
            windows::core::Error::from_hresult(windows::Win32::Foundation::E_INVALIDARG)
        })?;
    let (position, point, moved) =
        item.layout()
            .vertical_text_step(selection.focus, down, selection.vertical_point)?;
    if !moved {
        if let Some(next) = neighbor(text, &selection.focus_node, down) {
            let edge = if down { 0 } else { next.layout().text_len() };
            let (_, row) = next.layout().line_edge(edge, !down, None)?;
            // Preserve global column intent and convert it to the new owner's local column; never directly reuse the old local x.
            let target = [point[0] + item.origin()[0] - next.origin()[0], row[1]];
            let hit = next.layout().hit_text(target)?;
            selection.focus_node = next.node_id().to_owned();
            selection.focus = hit.position;
            selection.vertical_point = Some(target);
            return Ok(());
        }
    }
    selection.focus = position;
    selection.vertical_point = Some(point);
    Ok(())
}
