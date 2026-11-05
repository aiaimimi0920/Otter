//! 词级手势复用唯一选区；保存初始词与有界边界表，反向拖动不丢失起始词。
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
        Ok(true)
    }
}
pub(super) fn extend(
    selection: &mut Selection,
    item: &TextItem,
    position: u32,
    text: &TextFrame,
    point: [f32; 2],
) -> Result<()> {
    let cross_node = selection.cross_node();
    let forward = selection.forward(text);
    let Some(word) = &mut selection.word else {
        selection.focus = position;
        return Ok(());
    };
    // 不移动直接抬起仍保留初始双击空白；一旦拖动，返回原点也不恢复该空白。
    if !word.moved && !cross_node && point == word.initial_point {
        return Ok(());
    }
    word.moved = true;
    // 边界表只属于当前 focus owner；先成功查询新表再替换，不能借用旧节点索引。
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
