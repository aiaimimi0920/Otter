//! Word gestures reuse the unique selection; retain the initial word and bounded boundary tables so reverse dragging preserves the starting word.
use super::*;
pub(super) struct WordDrag {
    initial: Range<u32>,
    boundaries: Vec<u32>,
    boundary_node: String,
    initial_point: [f32; 2],
    moved: bool,
}
impl Presentation {
    pub(crate) fn begin_word_selection(&mut self, point: [f32; 2]) -> Result<bool> {
        if !self.begin_text_selection(point)? {
            return Ok(false);
        }
        self.select_initial_word(point)?;
        Ok(true)
    }
    pub(super) fn select_initial_word(&mut self, point: [f32; 2]) -> Result<()> {
        let selection = self.selection.as_mut().unwrap();
        let item = self
            .text
            .items()
            .iter()
            .find(|item| item.node_id() == selection.node)
            .unwrap();
        let boundaries = item.layout().word_boundaries()?;
        let initial = item.layout().word_range(selection.focus, &boundaries)?;
        let core = item.layout().word_segment(selection.focus, &boundaries)?;
        selection.anchor = initial.start;
        selection.focus = initial.end;
        selection.word = Some(WordDrag {
            initial: core,
            boundaries,
            boundary_node: selection.node.clone(),
            initial_point: point,
            moved: false,
        });
        Ok(())
    }
}
pub(super) fn extend(
    selection: &mut Selection,
    item: &TextItem,
    position: u32,
    text: &TextFrame,
    point: [f32; 2],
) -> Result<()> {
    if selection.word.is_none() {
        selection.focus = position;
        return Ok(());
    }
    let cross_node = selection.cross_node();
    let forward = cross_node && selection.forward(text);
    let word = selection.word.as_mut().unwrap();
    // Releasing without movement preserves initial double-click whitespace; after dragging, even returning to the origin does not restore it.
    if !word.moved && !cross_node && point == word.initial_point {
        return Ok(());
    }
    word.moved = true;
    // Boundary tables belong only to the current focus owner; query the new table successfully before replacement, never borrowing old-node indexes.
    if word.boundary_node != item.node_id() {
        let boundaries = item.layout().word_boundaries()?;
        word.boundaries = boundaries;
        word.boundary_node = item.node_id().to_owned();
    }
    let range = item.layout().word_segment(position, &word.boundaries)?;
    if cross_node {
        selection.anchor = if forward {
            word.initial.start
        } else {
            word.initial.end
        };
        selection.focus = if forward { range.end } else { range.start };
    } else if range.start < word.initial.start {
        selection.anchor = word.initial.end;
        selection.focus = range.start;
    } else {
        selection.anchor = word.initial.start;
        selection.focus = range.end.max(word.initial.end);
    }
    Ok(())
}
